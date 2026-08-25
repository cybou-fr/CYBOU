// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! One private model gateway for one live agent-capsule lease.

#[cfg(not(unix))]
compile_error!("cybou-agent-gateway is a Debian runtime and requires Unix domain sockets");

use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{BufReader, Write as _},
    num::NonZeroU64,
    os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _},
    os::unix::net::UnixListener as StdUnixListener,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use cybou_capsule::Lease;
use cybou_model_brokerd::BrokerCore;
use cybou_model_gateway::{GatewayCore, TokenPolicy, router};
use cybou_protocol::model::{ModelIdentity, ModelManifest, ModelRoute, ModelUsageSnapshot};
use cybou_provider_litellm::{LiteLlmRoute, LiteLlmWorker};
use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;
use time::OffsetDateTime;
use tokio::net::UnixListener;
use tower::ServiceExt as _;
use uuid::Uuid;

struct Config {
    runtime_dir: PathBuf,
    lease_file: PathBuf,
    task_id: Uuid,
    provider: String,
    model_group: String,
    base_url: String,
    master_key: String,
    deployment_sha256: [u8; 32],
    zero_cost: bool,
    token_limit: u64,
    max_output_tokens: u32,
    sensitivity: u8,
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
        let lease_file = PathBuf::from(get("CYBOU_AGENT_LEASE_FILE")?);
        if !runtime_dir.is_absolute() || !lease_file.is_absolute() {
            return Err("runtime directory and lease file must be absolute paths".to_owned());
        }
        let provider = nonblank("CYBOU_LITELLM_PROVIDER", get("CYBOU_LITELLM_PROVIDER")?)?;
        let model_group = nonblank(
            "CYBOU_LITELLM_MODEL_GROUP",
            get("CYBOU_LITELLM_MODEL_GROUP")?,
        )?;
        let master_key = load_master_key()?;
        let token_limit = parse("CYBOU_MODEL_TOKEN_LIMIT")?;
        let max_output_tokens = narrow(
            "CYBOU_MODEL_MAX_OUTPUT_TOKENS",
            parse("CYBOU_MODEL_MAX_OUTPUT_TOKENS")?,
        )?;
        let sensitivity = parse("CYBOU_MODEL_SENSITIVITY")?
            .try_into()
            .map_err(|_| "CYBOU_MODEL_SENSITIVITY exceeds 255".to_owned())?;
        if token_limit == 0 || max_output_tokens == 0 {
            return Err("token ceilings must be positive".to_owned());
        }
        Ok(Self {
            runtime_dir,
            lease_file,
            task_id: parse_uuid("CYBOU_AGENT_TASK_ID", &get("CYBOU_AGENT_TASK_ID")?)?,
            provider,
            model_group,
            base_url: get("CYBOU_LITELLM_BASE_URL")?,
            master_key,
            deployment_sha256: parse_sha256(&get("CYBOU_LITELLM_DEPLOYMENT_SHA256")?)?,
            zero_cost: declared("CYBOU_LITELLM_ZERO_COST", &get("CYBOU_LITELLM_ZERO_COST")?)?,
            token_limit,
            max_output_tokens,
            sensitivity,
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

/// An operator's declaration that a route bills nothing, spelled out rather than defaulted.
///
/// Only an operator knows what their deployment charges; Cybou cannot see a price list. A default
/// either way would be Cybou deciding on their behalf — one direction silently forbids the free
/// models a person selected, the other silently spends their money.
fn declared(name: &str, value: &str) -> Result<bool, String> {
    match value {
        "yes" => Ok(true),
        "no" => Ok(false),
        _ => Err(format!("{name} must be exactly yes or no")),
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

/// Read the one authoritative lease this instance serves.
///
/// The lease is minted once, by whoever owns the launch a person approved, and travels here as
/// data. Rebuilding an equivalent-looking lease from environment values would produce a second
/// authority: the two can each be internally valid and still describe different permissions, and
/// nothing downstream could tell which one the person actually selected.
///
/// ## Opened here, and not by the service manager
///
/// This used to arrive as a `LoadCredential`. It cannot, and the reason is measured rather than
/// assumed: the manager reads a credential source as root and follows symlinks, so a symlink at the
/// lease path delivers the contents of whatever it points at. The launch directory is owned by the
/// unprivileged user that writes leases into it, which would have made "name a root-only file and
/// have root read it out" a thing that user could do — the proxy credential this service is handed
/// among the targets.
///
/// Opened here it is the same user reading a file it wrote, so nothing is crossed and nothing
/// escalates. The check below is therefore not a security boundary; it is this process declining to
/// treat something other than the file the owner wrote as that file.
fn load_lease(path: &Path) -> Result<Lease, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("inspect {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!(
            "{} is not the regular file a launch writes a lease to",
            path.display()
        ));
    }
    let file = File::open(path).map_err(|error| format!("open {}: {error}", path.display()))?;
    ciborium::from_reader(BufReader::new(file))
        .map_err(|error| format!("read the lease at {}: {error}", path.display()))
}

fn worker(config: &Config, model_class: &str) -> Result<LiteLlmWorker, String> {
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
            model_class: model_class.to_owned(),
            model_group: config.model_group.clone(),
            zero_cost: config.zero_cost,
        }],
        config.microusd_per_unit,
        config.timeout_ms,
    )
    .map_err(|error| error.to_string())
}

type Started = (Arc<GatewayCore>, Arc<Mutex<Lease>>, String);

fn gateway(config: &Config, lease: Lease) -> Result<Started, String> {
    let now = OffsetDateTime::now_utc();
    if let Some(ended) = lease.ended(now) {
        return Err(format!(
            "refusing to serve a lease that is over: {}",
            ended.describe()
        ));
    }
    // Every bound below is read off the lease rather than off the environment. The class this
    // gateway routes and the ceiling it spends against are the ones on the approved grant, so a
    // launch file cannot widen either by naming a different value.
    let model_class = lease
        .grant()
        .model
        .as_ref()
        .ok_or_else(|| "the lease grants no model; this gateway has nothing to serve".to_owned())?
        .class
        .clone();

    let mut providers = BrokerCore::new();
    providers.register_provider(
        ModelRoute {
            provider: config.provider.clone(),
            external_boundary: true,
            sensitivity_ceiling: config.sensitivity,
            tasks: Vec::new(),
            context_limit: 1_000_000,
        },
        vec![model_class.clone()],
        Box::new(worker(config, &model_class)?),
    );
    let core = Arc::new(GatewayCore::new(Arc::new(providers)));
    let lease = Arc::new(Mutex::new(lease));
    let issued = core
        .issue_token(
            Arc::clone(&lease),
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
    Ok((core, lease, issued.expose_secret().to_owned()))
}

/// How often the ledger is written down for whoever owns this session.
///
/// A poll rather than a write per completion, and the snapshot carries the instant it was taken, so
/// the lag is stated rather than hidden. Writing on every completion would put a filesystem round
/// trip on the path a person is waiting on, to save a reader at most this long.
const USAGE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);

/// Publish what this gateway has spent, so the session owner can stop guessing.
///
/// The owner holds the grant a person approved and never the ledger: it handed this process a lease
/// as bytes, and this process charges its own copy. Without this file an owner reading its own lease
/// would report nought for a session that had been billed — so it reports *unknown* instead, and this
/// is what turns that into a figure.
///
/// Written only when something changed, and replaced atomically. A reader that caught a half-written
/// file would be reading a number that was never true.
async fn publish_usage(core: Arc<GatewayCore>, lease: Arc<Mutex<Lease>>, path: PathBuf) {
    let mut previous: Option<(u64, u64, u64)> = None;
    loop {
        if let Some(snapshot) = core.usage(&lease, OffsetDateTime::now_utc()) {
            let current = (snapshot.spend_units, snapshot.tokens, snapshot.completions);
            if previous != Some(current)
                && let Err(error) = write_usage(&path, &snapshot)
            {
                eprintln!("the ledger could not be published: {error}");
                // Kept as unwritten, so the next pass tries again rather than assuming a figure
                // reached a reader that never saw it.
                previous = None;
            } else if previous != Some(current) {
                previous = Some(current);
            }
        }
        tokio::time::sleep(USAGE_INTERVAL).await;
    }
}

fn write_usage(path: &Path, snapshot: &ModelUsageSnapshot) -> Result<(), String> {
    let rendered = serde_json::to_vec(snapshot).map_err(|error| error.to_string())?;
    let staging = path.with_extension("writing");
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&staging)
            .map_err(|error| format!("create {}: {error}", staging.display()))?;
        file.write_all(&rendered)
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("write {}: {error}", staging.display()))?;
    }
    fs::rename(&staging, path).map_err(|error| format!("publish {}: {error}", path.display()))
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
    let lease = load_lease(&config.lease_file)?;
    let (core, ledger, secret) = gateway(&config, lease)?;
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

    let usage_path = config.runtime_dir.join("model-usage.json");
    println!("CYBOU_MODEL_USAGE_FILE={}", usage_path.display());

    let runtime = tokio::runtime::Runtime::new().map_err(|error| error.to_string())?;
    let published = Arc::clone(&core);
    let serving = Arc::clone(&core);
    let result = runtime.block_on(async move {
        tokio::spawn(publish_usage(published, ledger, usage_path));
        serve(listener, serving).await
    });
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
