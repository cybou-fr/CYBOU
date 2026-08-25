// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! One private model gateway for one live agent-capsule lease.

#[cfg(not(unix))]
compile_error!("cybou-agent-gateway is a Debian runtime and requires Unix domain sockets");

use std::{
    env,
    fs::{self, OpenOptions},
    io::Write as _,
    num::NonZeroU64,
    os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _},
    os::unix::net::UnixListener as StdUnixListener,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use cybou_capsule::{
    CapabilityProfile, LeaseRequest, ModelGrant, ResourceBudget, Workspace, issue_lease,
};
use cybou_model_brokerd::BrokerCore;
use cybou_model_gateway::{GatewayCore, TokenPolicy, router};
use cybou_protocol::model::{ModelIdentity, ModelManifest, ModelRoute};
use cybou_provider_litellm::{LiteLlmRoute, LiteLlmWorker};
use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;
use time::{Duration, OffsetDateTime};
use tokio::net::UnixListener;
use tower::ServiceExt as _;
use uuid::Uuid;

struct Config {
    runtime_dir: PathBuf,
    capsule_id: Uuid,
    task_id: Uuid,
    workspace: PathBuf,
    model_class: String,
    provider: String,
    model_group: String,
    base_url: String,
    master_key: String,
    deployment_sha256: [u8; 32],
    spend_limit: u64,
    token_limit: u64,
    max_output_tokens: u32,
    sensitivity: u8,
    lease_seconds: i64,
    microusd_per_unit: NonZeroU64,
    timeout_ms: u32,
}

impl Config {
    fn from_environment() -> Result<Self, String> {
        let get = |name: &str| {
            env::var(name).map_err(|_| format!("required environment variable {name} is unset"))
        };
        let parse = |name: &str| -> Result<u64, String> {
            get(name)?
                .parse()
                .map_err(|_| format!("{name} is not an unsigned integer"))
        };
        let runtime_dir = PathBuf::from(get("CYBOU_AGENT_RUNTIME_DIR")?);
        let workspace = PathBuf::from(get("CYBOU_AGENT_WORKSPACE")?);
        if !runtime_dir.is_absolute() || !workspace.is_absolute() {
            return Err("runtime directory and workspace must be absolute paths".to_owned());
        }
        let model_class = nonblank("CYBOU_MODEL_CLASS", get("CYBOU_MODEL_CLASS")?)?;
        let provider = nonblank("CYBOU_LITELLM_PROVIDER", get("CYBOU_LITELLM_PROVIDER")?)?;
        let model_group = nonblank(
            "CYBOU_LITELLM_MODEL_GROUP",
            get("CYBOU_LITELLM_MODEL_GROUP")?,
        )?;
        let master_key = load_master_key()?;
        let spend_limit = parse("CYBOU_MODEL_SPEND_LIMIT")?;
        let token_limit = parse("CYBOU_MODEL_TOKEN_LIMIT")?;
        let max_output_tokens = narrow(
            "CYBOU_MODEL_MAX_OUTPUT_TOKENS",
            parse("CYBOU_MODEL_MAX_OUTPUT_TOKENS")?,
        )?;
        let sensitivity = parse("CYBOU_MODEL_SENSITIVITY")?
            .try_into()
            .map_err(|_| "CYBOU_MODEL_SENSITIVITY exceeds 255".to_owned())?;
        let lease_seconds = parse("CYBOU_AGENT_LEASE_SECONDS")?
            .try_into()
            .map_err(|_| "CYBOU_AGENT_LEASE_SECONDS is too large".to_owned())?;
        if lease_seconds <= 0 || token_limit == 0 || max_output_tokens == 0 {
            return Err("lease and token ceilings must be positive".to_owned());
        }
        Ok(Self {
            runtime_dir,
            capsule_id: parse_uuid("CYBOU_CAPSULE_ID", &get("CYBOU_CAPSULE_ID")?)?,
            task_id: parse_uuid("CYBOU_AGENT_TASK_ID", &get("CYBOU_AGENT_TASK_ID")?)?,
            workspace,
            model_class,
            provider,
            model_group,
            base_url: get("CYBOU_LITELLM_BASE_URL")?,
            master_key,
            deployment_sha256: parse_sha256(&get("CYBOU_LITELLM_DEPLOYMENT_SHA256")?)?,
            spend_limit,
            token_limit,
            max_output_tokens,
            sensitivity,
            lease_seconds,
            microusd_per_unit: NonZeroU64::new(parse("CYBOU_MODEL_MICROUSD_PER_UNIT")?)
                .ok_or_else(|| "CYBOU_MODEL_MICROUSD_PER_UNIT must be positive".to_owned())?,
            timeout_ms: narrow(
                "CYBOU_LITELLM_TIMEOUT_MS",
                parse("CYBOU_LITELLM_TIMEOUT_MS")?,
            )?,
        })
    }
}

fn load_master_key() -> Result<String, String> {
    if let Ok(path) = env::var("CYBOU_LITELLM_MASTER_KEY_FILE") {
        let value = fs::read_to_string(&path)
            .map_err(|error| format!("read LiteLLM credential {path}: {error}"))?;
        return nonblank("LiteLLM credential file", value.trim_end().to_owned());
    }
    let value = env::var("CYBOU_LITELLM_MASTER_KEY")
        .map_err(|_| "LiteLLM master key file is not configured".to_owned())?;
    nonblank("CYBOU_LITELLM_MASTER_KEY", value)
}

fn nonblank(name: &str, value: String) -> Result<String, String> {
    if value.trim().is_empty() || value.trim() != value {
        Err(format!(
            "{name} must be one non-blank value without outer whitespace"
        ))
    } else {
        Ok(value)
    }
}

fn narrow(name: &str, value: u64) -> Result<u32, String> {
    value.try_into().map_err(|_| format!("{name} exceeds u32"))
}

fn parse_uuid(name: &str, value: &str) -> Result<Uuid, String> {
    Uuid::parse_str(value).map_err(|_| format!("{name} is not a UUID"))
}

fn parse_sha256(value: &str) -> Result<[u8; 32], String> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("CYBOU_LITELLM_DEPLOYMENT_SHA256 is not 64 hexadecimal digits".to_owned());
    }
    let mut out = [0_u8; 32];
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| "invalid deployment digest".to_owned())?;
    }
    Ok(out)
}

fn worker(config: &Config) -> Result<LiteLlmWorker, String> {
    LiteLlmWorker::new(
        ModelManifest {
            model_id: format!("litellm:{}", config.provider),
            identity: ModelIdentity {
                family: "litellm-proxy".to_owned(),
                revision: config.provider.clone(),
                artifact_sha256: config.deployment_sha256,
                quantization: None,
                backend: "litellm-http".to_owned(),
                template_version: 1,
            },
            tasks: Vec::new(),
            license: "NOASSERTION".to_owned(),
            languages: Vec::new(),
            min_ram_mb: 1,
            context_limit: 1_000_000,
        },
        &config.base_url,
        config.master_key.clone(),
        vec![LiteLlmRoute {
            model_class: config.model_class.clone(),
            model_group: config.model_group.clone(),
        }],
        config.microusd_per_unit,
        config.timeout_ms,
    )
    .map_err(|error| error.to_string())
}

fn gateway(config: &Config) -> Result<(Arc<GatewayCore>, String), String> {
    let now = OffsetDateTime::now_utc();
    let budget = ResourceBudget {
        memory_mib: 512,
        cpus: 1,
        tasks_max: 64,
        lifetime: Duration::seconds(config.lease_seconds),
    };
    let mut profile =
        CapabilityProfile::bounded("opencode-live", budget).map_err(|error| error.to_string())?;
    profile.model = Some(ModelGrant {
        class: config.model_class.clone(),
        spend_limit: config.spend_limit,
    });
    profile.may_execute = true;
    let lease = issue_lease(
        LeaseRequest {
            selected_profile: profile,
            capsule_id: config.capsule_id,
            agent: "opencode".to_owned(),
            workspace: Workspace::at(&config.workspace),
        },
        now,
    )
    .map_err(|error| error.to_string())?;

    let mut providers = BrokerCore::new();
    providers.register_provider(
        ModelRoute {
            provider: config.provider.clone(),
            external_boundary: true,
            sensitivity_ceiling: config.sensitivity,
            tasks: Vec::new(),
            context_limit: 1_000_000,
        },
        vec![config.model_class.clone()],
        Box::new(worker(config)?),
    );
    let core = Arc::new(GatewayCore::new(Arc::new(providers)));
    let issued = core
        .issue_token(
            Arc::new(Mutex::new(lease)),
            config.task_id,
            TokenPolicy {
                local_only: false,
                sensitivity: config.sensitivity,
                max_output_tokens: config.max_output_tokens,
                token_limit: config.token_limit,
            },
            now,
        )
        .map_err(|error| error.to_string())?;
    Ok((core, issued.expose_secret().to_owned()))
}

fn prepare_runtime(path: &Path) -> Result<(), String> {
    if path.exists() {
        let metadata = fs::symlink_metadata(path)
            .map_err(|error| format!("inspect {}: {error}", path.display()))?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(format!("{} is not a real directory", path.display()));
        }
    } else {
        fs::create_dir(path).map_err(|error| format!("create {}: {error}", path.display()))?;
    }
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|error| format!("chmod {}: {error}", path.display()))
}

fn write_token(path: &Path, secret: &str) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|error| format!("create {}: {error}", path.display()))?;
    file.write_all(secret.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|error| format!("write {}: {error}", path.display()))
}

fn main() -> Result<(), String> {
    let config = Config::from_environment()?;
    prepare_runtime(&config.runtime_dir)?;
    let socket_path = config.runtime_dir.join("model.sock");
    let token_path = config.runtime_dir.join("model-token");
    if socket_path.exists() || token_path.exists() {
        return Err("runtime socket or token already exists; refusing to replace it".to_owned());
    }
    let (core, secret) = gateway(&config)?;
    write_token(&token_path, &secret)?;
    let listener = StdUnixListener::bind(&socket_path)
        .map_err(|error| format!("bind {}: {error}", socket_path.display()))?;
    listener
        .set_nonblocking(true)
        .map_err(|error| format!("set nonblocking {}: {error}", socket_path.display()))?;
    fs::set_permissions(&socket_path, fs::Permissions::from_mode(0o600))
        .map_err(|error| format!("chmod {}: {error}", socket_path.display()))?;
    println!("CYBOU_MODEL_SOCKET={}", socket_path.display());
    println!("CYBOU_MODEL_TOKEN_FILE={}", token_path.display());

    let runtime = tokio::runtime::Runtime::new().map_err(|error| error.to_string())?;
    let result = runtime.block_on(serve(listener, Arc::clone(&core)));
    drop(runtime);
    drop(core);
    result
}

async fn serve(listener: StdUnixListener, core: Arc<GatewayCore>) -> Result<(), String> {
    let listener = UnixListener::from_std(listener).map_err(|error| error.to_string())?;
    let app = router(core);
    loop {
        let (stream, _) = listener.accept().await.map_err(|error| error.to_string())?;
        let service = app.clone();
        tokio::spawn(async move {
            let connection = http1::Builder::new().serve_connection(
                TokioIo::new(stream),
                hyper::service::service_fn(move |request| service.clone().oneshot(request)),
            );
            if let Err(error) = connection.await {
                eprintln!("model gateway connection failed: {error}");
            }
        });
    }
}
