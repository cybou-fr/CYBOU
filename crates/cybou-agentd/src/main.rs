// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The agent session owner.
//!
//! `plan` mints the one lease and prints every file, unit and teardown step a launch implies without
//! touching a host. `launch` carries that same plan out and holds the session until it ends.
//!
//! Nothing here decides anything. Every path, unit name, file body and command was already produced
//! by `cybou_agentd::plan` and `cybou_agentd::runtime`; this file is where those meet a filesystem
//! and a service manager, and it is deliberately the only part of the crate that cannot be answered
//! by reading.

#[cfg(not(unix))]
compile_error!("cybou-agentd owns Linux capsules and systemd units");

use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration as StdDuration;

use agent_client_protocol::AcpAgentConfig;
use cybou_acp::AcpSession;
use cybou_agentd::plan::SessionPlan;
use cybou_agentd::profiles::{ProfileCatalogue, Wanted};
use cybou_agentd::session::{Session, SessionEnd, SessionState};
use cybou_agentd::view::{Ledger, SessionView};
use cybou_agentd::{Ceilings, HostPrograms, Launch, TeardownStep, plan, runtime};
use cybou_capsule::{
    CapabilityProfile, KernelCapsuleSpec, Lease, LeaseRequest, ModelGrant, NetworkGrant,
    ResourceBudget, SpendPolicy, Workspace, compile, issue_lease,
};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

/// How long a launch waits for a surface it has just started to exist.
const READY_TIMEOUT: StdDuration = StdDuration::from_secs(30);

fn usage() -> &'static str {
    "usage:\n  \
     cybou-agentd plan   <selection>\n  \
     cybou-agentd launch <selection> -- <program> [argument …]\n  \
     cybou-agentd launch <selection> --prompt TEXT\n  \
     cybou-agentd start  --profile ID --agent NAME --workspace PATH [--model CLASS]\n  \
     cybou-agentd serve\n  \
     cybou-agentd sessions\n  \
     cybou-agentd stop <capsule-uuid>\n\n\
     selection:\n  \
     --profile ID --agent NAME --workspace PATH\n  \
     --memory-mib N --cpus N --tasks-max N --lifetime-seconds N\n  \
     --token-limit N --max-output-tokens N --sensitivity N\n  \
     [--model CLASS --spend-limit N|zero-cost] [--host HOST]… [--may-execute]\n  \
     [--capsule-id UUID] [--task-id UUID]"
}

/// One launch selection, exactly as a person made it.
#[derive(Default)]
struct Selection {
    profile: Option<String>,
    agent: Option<String>,
    workspace: Option<PathBuf>,
    memory_mib: Option<u32>,
    cpus: Option<u32>,
    tasks_max: Option<u32>,
    lifetime_seconds: Option<i64>,
    token_limit: Option<u64>,
    max_output_tokens: Option<u32>,
    sensitivity: Option<u8>,
    model: Option<String>,
    spend_limit: Option<String>,
    hosts: Vec<String>,
    may_execute: bool,
    capsule_id: Option<Uuid>,
    task_id: Option<Uuid>,
    program: Vec<String>,
    prompt: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let mut arguments = std::env::args().skip(1);
    let verb = arguments.next();
    match verb.as_deref() {
        Some("plan") => show_plan(&parse(arguments.collect())?),
        Some("launch") => launch(&parse(arguments.collect())?).await,
        Some("start") => start(&parse(arguments.collect())?).await,
        Some("serve") => serve().await,
        Some("sessions") => sessions().await,
        Some("stop") => stop(&parse(arguments.collect())?).await,
        _ => Err(usage().to_owned()),
    }
}

fn parse(arguments: Vec<String>) -> Result<Selection, String> {
    let mut selection = Selection::default();
    let mut arguments = arguments.into_iter();
    while let Some(flag) = arguments.next() {
        // Everything after the separator is the program. A program whose name begins with a dash is
        // a program, not an option — the same rule the capsule command itself follows.
        if flag == "--" {
            selection.program = arguments.collect();
            break;
        }
        let mut value = || {
            arguments
                .next()
                .ok_or_else(|| format!("{flag} needs a value"))
        };
        match flag.as_str() {
            "--may-execute" => selection.may_execute = true,
            "--profile" => selection.profile = Some(value()?),
            "--agent" => selection.agent = Some(value()?),
            "--workspace" => selection.workspace = Some(PathBuf::from(value()?)),
            "--model" => selection.model = Some(value()?),
            "--host" => selection.hosts.push(value()?),
            "--memory-mib" => selection.memory_mib = Some(number(&flag, &value()?)?),
            "--cpus" => selection.cpus = Some(number(&flag, &value()?)?),
            "--tasks-max" => selection.tasks_max = Some(number(&flag, &value()?)?),
            "--lifetime-seconds" => selection.lifetime_seconds = Some(number(&flag, &value()?)?),
            "--token-limit" => selection.token_limit = Some(number(&flag, &value()?)?),
            "--max-output-tokens" => selection.max_output_tokens = Some(number(&flag, &value()?)?),
            "--sensitivity" => selection.sensitivity = Some(number(&flag, &value()?)?),
            "--spend-limit" => selection.spend_limit = Some(value()?),
            "--capsule-id" => selection.capsule_id = Some(uuid(&flag, &value()?)?),
            "--task-id" => selection.task_id = Some(uuid(&flag, &value()?)?),
            "--prompt" => selection.prompt = Some(value()?),
            other => return Err(format!("unknown argument {other}\n{}", usage())),
        }
    }
    Ok(selection)
}

fn number<T: std::str::FromStr>(flag: &str, value: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("{flag} is not a number in range"))
}

fn uuid(flag: &str, value: &str) -> Result<Uuid, String> {
    Uuid::parse_str(value).map_err(|_| format!("{flag} is not a UUID"))
}

/// The two spending selections, said in words rather than encoded in whether a number is zero.
///
/// `--spend-limit zero-cost` is not `--spend-limit 0`. One says *spend nothing, on a route that is
/// known to cost nothing*; the other says *there is a ceiling and it is empty*. They lead to
/// different routing and they fail differently, and an integer could not tell them apart — which is
/// how the selection a person makes to use a free model became the one every worker refused.
fn spend_policy(value: &str) -> Result<SpendPolicy, String> {
    if value == "zero-cost" {
        return Ok(SpendPolicy::ZeroCostOnly);
    }
    value
        .parse()
        .map(SpendPolicy::Capped)
        .map_err(|_| "--spend-limit takes an integer or the word zero-cost".to_owned())
}

fn required<T>(value: Option<T>, flag: &str) -> Result<T, String> {
    value.ok_or_else(|| format!("{flag} is required\n{}", usage()))
}

/// Mint the one lease this selection produces.
fn mint(selection: &Selection, now: OffsetDateTime) -> Result<Lease, String> {
    let mut profile = CapabilityProfile::bounded(
        required(selection.profile.clone(), "--profile")?,
        ResourceBudget {
            memory_mib: required(selection.memory_mib, "--memory-mib")?,
            cpus: required(selection.cpus, "--cpus")?,
            tasks_max: required(selection.tasks_max, "--tasks-max")?,
            lifetime: Duration::seconds(required(
                selection.lifetime_seconds,
                "--lifetime-seconds",
            )?),
        },
    )
    .map_err(|error| error.to_string())?;
    profile.network = NetworkGrant {
        hosts: selection.hosts.clone(),
    };
    // A class with no ceiling beside it is half a selection: it would leave the spending bound to be
    // invented by whichever component needed one first, which is the shape this crate exists to end.
    profile.model = match &selection.model {
        Some(class) => Some(ModelGrant {
            class: class.clone(),
            spend: spend_policy(&required(selection.spend_limit.clone(), "--spend-limit")?)?,
        }),
        None => None,
    };
    profile.may_execute = selection.may_execute;

    issue_lease(
        LeaseRequest {
            selected_profile: profile,
            capsule_id: selection.capsule_id.unwrap_or_else(Uuid::new_v4),
            agent: required(selection.agent.clone(), "--agent")?,
            workspace: Workspace::at(required(selection.workspace.clone(), "--workspace")?),
        },
        now,
    )
    .map_err(|error| error.to_string())
}

fn prepare(selection: &Selection) -> Result<(Lease, SessionPlan, KernelCapsuleSpec), String> {
    let now = OffsetDateTime::now_utc();
    let lease = mint(selection, now)?;
    // Asked for only when there will be a bearer for them to bound. A session granted no model has
    // none, and demanding ceilings for it refused every capsule that was never going to ask a model
    // anything — which is the ordinary case on a host with no provider at all.
    let ceilings = if lease.grant().model.is_some() {
        Ceilings {
            token_limit: required(selection.token_limit, "--token-limit")?,
            max_output_tokens: required(selection.max_output_tokens, "--max-output-tokens")?,
            sensitivity: required(selection.sensitivity, "--sensitivity")?,
        }
    } else {
        Ceilings::none()
    };
    let launch = Launch {
        lease: lease.clone(),
        task_id: selection.task_id.unwrap_or_else(Uuid::new_v4),
        ceilings,
    };
    let plan = plan::plan(&launch, now).map_err(|error| error.to_string())?;
    let spec = compile(lease.grant()).map_err(|error| error.to_string())?;
    Ok((lease, plan, spec))
}

fn show_plan(selection: &Selection) -> Result<(), String> {
    let (lease, plan, _) = prepare(selection)?;

    println!("session {}", plan.instance);
    println!("profile {}", lease.profile_id().as_str());
    println!("agent {}", lease.grant().agent);
    println!("workspace {}", lease.grant().workspace.root.display());
    println!("expires {}", plan.expires_at);
    println!("lease-file {}", plan.lease_file.display());
    println!("launch-file {}", plan.launch_file.display());
    match (&plan.gateway_unit, &plan.model_socket, &plan.model_token) {
        (Some(unit), Some(socket), Some(token)) => {
            println!("gateway-unit {unit}");
            println!("model-socket {}", socket.display());
            println!("model-token {}", token.display());
        }
        // Said out loud rather than left blank. A capsule granted no model has no gateway, and a
        // reader should be able to tell that from a gateway whose name failed to print.
        _ => println!("gateway-unit none: this lease grants no model"),
    }
    println!("capsule-unit {}", plan.capsule_unit);
    println!("egress-unit {}", plan.egress_unit);
    println!("egress-socket {}", plan.egress_socket.display());
    for line in plan.launch_environment.lines() {
        println!("launch-env {line}");
    }
    for step in plan.teardown() {
        match step {
            TeardownStep::StopCapsule(unit) => println!("teardown stop-capsule {unit}"),
            TeardownStep::StopGateway(unit) => println!("teardown stop-gateway {unit}"),
            TeardownStep::StopEgress(unit) => println!("teardown stop-egress {unit}"),
            TeardownStep::Remove(path) => println!("teardown remove {}", path.display()),
        }
    }
    Ok(())
}

/// Carry out one launch and hold the session until it ends.
async fn launch(selection: &Selection) -> Result<(), String> {
    // A program or a prompt, never both. They are two different claims about what the capsule is
    // for, and a launch that accepted both would have to decide which one a person meant.
    match (selection.program.is_empty(), selection.prompt.is_some()) {
        (true, false) => {
            return Err(format!(
                "launch needs a program after -- or a --prompt\n{}",
                usage()
            ));
        }
        (false, true) => {
            return Err("a launch runs a program or asks an agent, not both".to_owned());
        }
        _ => {}
    }
    let (lease, plan, spec) = prepare(selection)?;
    let programs = HostPrograms::default();
    let mut session = Session::launching(lease.grant().capsule_id, OffsetDateTime::now_utc());
    println!("session {} launching", plan.instance);

    // From here on every path ends in teardown, including the ones that fail. A launch that gave up
    // halfway and returned would leave a live gateway holding a bearer for a session nobody watches.
    match bring_up(&plan, &spec, &programs, selection, &mut session).await {
        Ok(true) => session.begin_ending(SessionEnd::AgentFinished),
        Ok(false) => session.begin_ending(SessionEnd::Failed(
            "the agent exited with a failure".to_owned(),
        )),
        Err(why) => session.begin_ending(SessionEnd::Failed(why)),
    };

    // The lease has the last word on why, and it is consulted after the fact rather than before: an
    // agent whose capsule hit its deadline did not finish, and saying it did would report a session
    // that was stopped as one that completed.
    session.observe(&lease, OffsetDateTime::now_utc());
    teardown(&plan);
    session.finish_ending(OffsetDateTime::now_utc());

    match session.state() {
        SessionState::Ended(SessionEnd::Failed(why)) => {
            Err(format!("session {} failed: {why}", plan.instance))
        }
        SessionState::Ended(end) => {
            announce(&session, &plan);
            println!("session {} ended: {}", plan.instance, end.describe());
            Ok(())
        }
        other => Err(format!(
            "session {} is {other:?} after teardown",
            plan.instance
        )),
    }
}

/// Write the files, start the surfaces, run the capsule. Whether the capsule succeeded.
async fn bring_up(
    plan: &SessionPlan,
    spec: &KernelCapsuleSpec,
    programs: &HostPrograms,
    selection: &Selection,
    session: &mut Session,
) -> Result<bool, String> {
    write_session_files(plan)?;

    if let Some(broker) = runtime::start_egress(plan, spec, programs) {
        run(&broker)?;
        wait_for(std::slice::from_ref(&plan.egress_socket)).await?;
    }

    if let Some(gateway) = runtime::start_gateway(plan) {
        run(&gateway)?;
        let expected: Vec<PathBuf> = [plan.model_socket.clone(), plan.model_token.clone()]
            .into_iter()
            .flatten()
            .collect();
        wait_for(&expected).await?;
    }

    let program = match &selection.prompt {
        Some(_) => agent_entrypoint(plan)?,
        None => selection.program.clone(),
    };
    let capsule =
        runtime::run_capsule(plan, spec, programs, &program).map_err(|error| error.to_string())?;
    session.running().map_err(|error| error.to_string())?;
    // One line a surface can read, rather than prose a surface would have to parse. Everything on it
    // is a fact off the approved lease and the compiled spec — the ceilings a person selected, not a
    // reading of what the capsule is using, which nothing here can honestly observe yet.
    announce(session, plan);

    match &selection.prompt {
        Some(prompt) => ask(plan, &capsule, prompt).await,
        None => status_of(&capsule),
    }
}

/// The ACP entrypoint for this session's agent, and the configuration it needs to find its gateway.
///
/// Written into the workspace rather than passed as an argument, because that is where the agent
/// looks and because the configuration names a token *file* and never a token. A provider credential
/// cannot end up here: there is none in this process to write.
fn agent_entrypoint(plan: &SessionPlan) -> Result<Vec<String>, String> {
    let agent = &plan.lease.grant().agent;
    if agent != cybou_agent_opencode::AGENT_ID {
        return Err(format!(
            "no agent pack for '{agent}'; only '{}' can be driven over ACP today",
            cybou_agent_opencode::AGENT_ID
        ));
    }
    let class = &plan
        .lease
        .grant()
        .model
        .as_ref()
        .ok_or_else(|| "an agent driven over ACP needs the model its lease grants".to_owned())?
        .class;

    let directory = plan.lease.grant().workspace.root.join(".cybou");
    fs::create_dir_all(&directory)
        .map_err(|error| format!("create {}: {error}", directory.display()))?;
    let rendered = cybou_agent_opencode::configuration(class)
        .map_err(|error| format!("render the agent configuration: {error}"))?;
    let path = directory.join("opencode.json");
    fs::write(&path, rendered).map_err(|error| format!("write {}: {error}", path.display()))?;

    Ok(cybou_agent_opencode::command())
}

/// Drive one prompt turn against the agent now running inside the capsule.
///
/// The deadline is what remains of the lease, not a constant. A turn cut short by a number in this
/// file would be a second clock beside the one a person actually granted.
async fn ask(plan: &SessionPlan, capsule: &[String], prompt: &str) -> Result<bool, String> {
    let remaining = plan.expires_at - OffsetDateTime::now_utc();
    let remaining = StdDuration::try_from(remaining)
        .map_err(|_| "the lease has no time left to ask anything in".to_owned())?;

    let (program, arguments) = capsule
        .split_first()
        .ok_or_else(|| "the capsule has no command".to_owned())?;
    let mut process = AcpAgentConfig::new(program);
    for argument in arguments {
        process = process.arg(argument);
    }

    let turn = AcpSession::within(remaining)
        .one_turn(process, plan.lease.grant().workspace.root.clone(), prompt)
        .await
        .map_err(|error| error.to_string())?;

    print!("{}", turn.message);
    if !turn.message.ends_with('\n') {
        println!();
    }
    // Surfaced rather than swallowed. An agent that keeps asking the client to widen its capsule is
    // telling a person something about the profile they selected, and a refusal recorded nowhere
    // would have thrown that away.
    for wanted in &turn.refused_permissions {
        eprintln!("session {} refused a request to {wanted}", plan.instance);
    }
    println!("session {} turn ended: {}", plan.instance, turn.stop_reason);
    Ok(turn.ended_by_the_agent())
}

/// Write the lease and the launch file, in that order.
///
/// The lease first. It is the authority; a launch file that existed without one would let a unit
/// start against ceilings with nothing to check them against.
fn write_session_files(plan: &SessionPlan) -> Result<(), String> {
    private_directory(&plan.session_runtime)?;

    let mut encoded = Vec::new();
    ciborium::into_writer(&plan.lease, &mut encoded)
        .map_err(|error| format!("encode the lease: {error}"))?;
    write_private(&plan.lease_file, &encoded)?;
    write_private(&plan.launch_file, plan.launch_environment.as_bytes())
}

fn private_directory(path: &Path) -> Result<(), String> {
    if path.exists() {
        let metadata = fs::symlink_metadata(path)
            .map_err(|error| format!("inspect {}: {error}", path.display()))?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(format!("{} is not a real directory", path.display()));
        }
    } else {
        fs::create_dir_all(path).map_err(|error| format!("create {}: {error}", path.display()))?;
    }
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|error| format!("restrict {}: {error}", path.display()))
}

/// Create a file that did not exist, mode `0600`.
///
/// `create_new`, so a session never writes over another session's launch. Two sessions holding the
/// same capsule id is not a collision to resolve; it is a launch that must not happen.
fn write_private(path: &Path, contents: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create {}: {error}", parent.display()))?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|error| format!("create {}: {error}", path.display()))?;
    file.write_all(contents)
        .and_then(|()| file.sync_all())
        .map_err(|error| format!("write {}: {error}", path.display()))
}

/// Wait for surfaces that were just started to exist.
async fn wait_for(paths: &[PathBuf]) -> Result<(), String> {
    let deadline = tokio::time::Instant::now() + READY_TIMEOUT;
    loop {
        if paths.iter().all(|path| path.exists()) {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            let missing: Vec<String> = paths
                .iter()
                .filter(|path| !path.exists())
                .map(|path| path.display().to_string())
                .collect();
            return Err(format!("timed out waiting for {}", missing.join(", ")));
        }
        tokio::time::sleep(StdDuration::from_millis(50)).await;
    }
}

/// Stop one unit, and judge it by whether the unit is running rather than by what stopping returned.
///
/// A capsule run to completion has already exited and been collected, so asking systemd to stop it
/// fails. Believing that exit code meant every clean session ended by printing a teardown error,
/// which teaches an operator to ignore teardown errors — and the one that matters looks the same.
///
/// What teardown is for is that the unit is not running. So the stop is attempted, its own answer is
/// discarded, and the host is asked.
fn stop_unit(argv: &[String], plan: &SessionPlan, step: &TeardownStep) -> Result<(), String> {
    let Some(check) = runtime::still_running(plan, step) else {
        return status_of(argv).map(|_| ());
    };
    // Asked before it is told, so a unit that already finished is left alone. Not only to keep the
    // result right: stopping something that is gone makes the service manager print a failure of its
    // own, and output a person is meant to ignore on every clean run is output they will ignore on
    // the run that mattered.
    if status_of(&check) == Ok(false) {
        return Ok(());
    }
    let attempted = status_of(argv);
    match status_of(&check) {
        Ok(true) => Err(format!("{} is still running", argv.join(" "))),
        Ok(false) => Ok(()),
        // The service manager could not be asked. The stop's own answer is then the best available
        // account, and saying nothing would be claiming an outcome nobody observed.
        Err(why) => attempted.map(|_| ()).map_err(|_| why),
    }
}

fn run(argv: &[String]) -> Result<(), String> {
    if status_of(argv)? {
        Ok(())
    } else {
        Err(format!("{} failed", argv.join(" ")))
    }
}

fn status_of(argv: &[String]) -> Result<bool, String> {
    let (program, arguments) = argv
        .split_first()
        .ok_or_else(|| "an empty command".to_owned())?;
    Command::new(program)
        .args(arguments)
        .status()
        .map(|status| status.success())
        .map_err(|error| format!("{program}: {error}"))
}

/// Undo the launch, in the planned order, whatever else happened.
///
/// Every step is attempted even if an earlier one failed. A teardown that stopped at the first error
/// would leave exactly the pieces that are hardest to notice: a broker with no capsule, a gateway
/// holding a bearer for a session that is over.
fn teardown(plan: &SessionPlan) {
    for step in plan.teardown() {
        let result = match &step {
            TeardownStep::StopCapsule(_) => stop_unit(&runtime::stop_capsule(plan), plan, &step),
            TeardownStep::StopGateway(_) => {
                runtime::stop_gateway(plan).map_or(Ok(()), |stop| stop_unit(&stop, plan, &step))
            }
            TeardownStep::StopEgress(_) => stop_unit(&runtime::stop_egress(plan), plan, &step),
            TeardownStep::Remove(path) => match fs::remove_file(path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(format!("remove {}: {error}", path.display())),
            },
        };
        if let Err(why) = result {
            eprintln!("teardown step {step:?} did not complete: {why}");
        }
    }
    if let Err(error) = fs::remove_dir(&plan.session_runtime)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        eprintln!("teardown left {}: {error}", plan.session_runtime.display());
    }
}

/// Print what a surface would show about this session, as one line of JSON.
///
/// A line rather than prose, and the same shape at every point in the session's life, so a card
/// drawing it does not have to know which stage produced it.
///
/// [`Ledger::Elsewhere`], and this is the whole point of that type existing. The model gateway is a
/// different process: it received the lease as bytes and charges its own copy, so nothing here can
/// see what has been spent. Reporting nought would be a claim this process is in no position to
/// make, and would be wrong the moment a completion happened.
fn announce(session: &Session, plan: &SessionPlan) {
    let view = SessionView::of(session, plan, Ledger::Elsewhere);
    match serde_json::to_string(&view) {
        Ok(line) => println!("{line}"),
        Err(error) => eprintln!("the session could not be described: {error}"),
    }
}

/// How often a held session's own lease is asked whether it is over.
///
/// A poll rather than a timer per session, and neither of them is what ends anything: the capsule
/// unit's `RuntimeMaxSec` does that, without this process. This only notices, so that a listing
/// stops showing a session as running and its leftovers get cleared.
const EXPIRY_POLL: StdDuration = StdDuration::from_secs(15);

/// Where sessions write the two files that describe them.
#[cfg(target_os = "linux")]
const LEASE_ROOT: &str = "/run/cybou-agent-leases";

/// Hold what is running on this host, and answer for it on the bus.
#[cfg(target_os = "linux")]
async fn serve() -> Result<(), String> {
    use std::sync::Mutex;

    use cybou_agentd::service::Agent1Service;
    use cybou_fabric::AGENT;

    let recovered = discover(Path::new(LEASE_ROOT), OffsetDateTime::now_utc());
    for (capsule_id, why) in &recovered.unreadable {
        eprintln!("[cybou-agentd] Session {capsule_id} could not be read back: {why}");
    }
    // Cleared before anything is served, so a listing never shows a session whose capsule is gone
    // and never leaves a gateway holding a bearer for one.
    for plan in &recovered.orphaned {
        println!(
            "[cybou-agentd] Clearing what is left of session {}",
            plan.instance
        );
        teardown(plan);
    }
    println!(
        "[cybou-agentd] Holding {} running session(s)",
        recovered.registry.len()
    );

    let registry = Arc::new(Mutex::new(recovered.registry));
    let _connection = zbus::connection::Builder::session()
        .map_err(|error| error.to_string())?
        .name(AGENT.service)
        .map_err(|error| error.to_string())?
        .serve_at(
            AGENT.object_path,
            Agent1Service::new(Arc::clone(&registry), Arc::new(HostTeardown)),
        )
        .map_err(|error| error.to_string())?
        .build()
        .await
        .map_err(|error| error.to_string())?;
    println!("[cybou-agentd] Registered {}", AGENT.service);

    loop {
        tokio::select! {
            () = tokio::time::sleep(EXPIRY_POLL) => {
                read_ledgers(&registry);
                expire(&registry);
            }
            result = tokio::signal::ctrl_c() => {
                result.map_err(|error| error.to_string())?;
                // Nothing is torn down on the way out. A capsule outlives this process on purpose,
                // and ending every session because the owner was restarted would make the coordinator
                // into the boundary that ADR-0042 says it must not be.
                println!("[cybou-agentd] Leaving running sessions running");
                return Ok(());
            }
        }
    }
}

#[cfg(not(target_os = "linux"))]
async fn serve() -> Result<(), String> {
    Err("cybou-agentd serves capsules on Linux".to_owned())
}

/// Re-read what each session's gateway has published about its own spending.
///
/// The only truthful source for that figure. This process holds the grant a person approved and the
/// gateway holds the ledger, so a listing says *unknown* until one of these reads succeeds — which is
/// the honest answer, and a great deal better than a nought nobody measured.
#[cfg(target_os = "linux")]
fn read_ledgers(registry: &Arc<std::sync::Mutex<cybou_agentd::registry::SessionRegistry>>) {
    let Ok(mut held) = registry.lock() else {
        return;
    };
    held.read_ledgers(|path| {
        let bytes = fs::read(path).ok()?;
        cybou_agentd::discovery::read_usage(&bytes).ok()
    });
}

/// End the sessions whose leases have run out, exactly once each.
#[cfg(target_os = "linux")]
fn expire(registry: &Arc<std::sync::Mutex<cybou_agentd::registry::SessionRegistry>>) {
    let now = OffsetDateTime::now_utc();
    let Ok(mut held) = registry.lock() else {
        return;
    };
    for capsule_id in held.expire(now) {
        if let Some(mut live) = held.take(capsule_id) {
            println!("[cybou-agentd] Session {capsule_id} reached the end of its lease");
            teardown(&live.plan);
            live.session.finish_ending(now);
        }
    }
}

/// Read every session this host has written down, and ask the service manager which are still up.
#[cfg(target_os = "linux")]
fn discover(root: &Path, now: OffsetDateTime) -> cybou_agentd::registry::Recovered {
    use cybou_agentd::discovery;

    let files = match discovery::sessions_on(root) {
        Ok(files) => files,
        Err(error) => {
            eprintln!(
                "[cybou-agentd] {} could not be read: {error}",
                root.display()
            );
            return cybou_agentd::registry::Recovered::default();
        }
    };

    let mut found = Vec::new();
    for entry in files {
        let (Ok(lease), Ok(launch)) = (fs::read(&entry.lease), fs::read_to_string(&entry.launch))
        else {
            eprintln!(
                "[cybou-agentd] Session {} left files that cannot be opened",
                entry.capsule_id
            );
            continue;
        };
        // Asked of the service manager, never inferred from the files. A file says what a launch
        // intended; only a running unit says what is true now.
        let active = capsule_is_active(entry.capsule_id);
        match discovery::read_session(&lease, &launch, active) {
            Ok(session) => found.push(session),
            Err(why) => {
                eprintln!(
                    "[cybou-agentd] Session {} could not be read: {why}",
                    entry.capsule_id
                );
            }
        }
    }
    cybou_agentd::registry::recover(found, now)
}

#[cfg(target_os = "linux")]
fn capsule_is_active(capsule_id: Uuid) -> bool {
    Command::new("systemctl")
        .args([
            "--user",
            "is-active",
            "--quiet",
            &format!("cybou-capsule-{capsule_id}.service"),
        ])
        .status()
        .is_ok_and(|status| status.success())
}

/// Run one session's teardown on the host.
#[cfg(target_os = "linux")]
struct HostTeardown;

#[cfg(target_os = "linux")]
impl cybou_agentd::service::Teardown for HostTeardown {
    fn tear_down(&self, plan: &SessionPlan) {
        teardown(plan);
    }
}

/// Where an operator writes down the profiles this host offers.
const CATALOGUE: &str = "/etc/cybou/agent-profiles.json";

/// Launch under bounds an operator approved, rather than bounds the caller named.
///
/// The same session as `launch` and a different door to it. `launch` takes ceilings as arguments,
/// which is right for bring-up on a host somebody is sitting at — whoever can run it is already
/// `cybou`. This one is the shape a bus method or a web endpoint can have: the caller names a
/// profile, an agent, a workspace and one of the models that profile offers, and every bound comes
/// from the file only root can write.
///
/// The flags this deliberately does not accept are the point. No memory, no CPUs, no tasks, no
/// lifetime, no hosts, no spending policy, no token ceilings, no sensitivity. A door that took those
/// would be asking its caller to invent a `CapsuleGrant`.
async fn start(selection: &Selection) -> Result<(), String> {
    let catalogue = fs::read(CATALOGUE)
        .map_err(|error| format!("read {CATALOGUE}: {error}"))
        .and_then(|bytes| ProfileCatalogue::read(&bytes).map_err(|why| why.to_string()))?;

    let wanted = Wanted {
        profile: required(selection.profile.clone(), "--profile")?,
        agent: required(selection.agent.clone(), "--agent")?,
        workspace: required(selection.workspace.clone(), "--workspace")?,
        model_class: selection.model.clone(),
    };
    let (granted, workspace, ceilings) = catalogue
        .grant(&wanted)
        .map_err(|why| format!("{why}\napproved profiles: {}", catalogue.names().join(", ")))?;

    // Rebuilt as a selection so both doors converge on one launch path. A second bring-up written
    // beside the first is a second set of decisions about what a launch does.
    let approved = Selection {
        profile: Some(granted.id.as_str().to_owned()),
        agent: Some(wanted.agent),
        workspace: Some(workspace.root),
        memory_mib: Some(granted.budget.memory_mib),
        cpus: Some(granted.budget.cpus),
        tasks_max: Some(granted.budget.tasks_max),
        lifetime_seconds: Some(granted.budget.lifetime.whole_seconds()),
        token_limit: ceilings.map(|ceilings| ceilings.token_limit),
        max_output_tokens: ceilings.map(|ceilings| ceilings.max_output_tokens),
        sensitivity: ceilings.map(|ceilings| ceilings.sensitivity),
        model: granted.model.as_ref().map(|model| model.class.clone()),
        spend_limit: granted.model.as_ref().map(|model| match model.spend {
            cybou_capsule::SpendPolicy::ZeroCostOnly => "zero-cost".to_owned(),
            cybou_capsule::SpendPolicy::Capped(limit) => limit.to_string(),
        }),
        hosts: granted.network.hosts.clone(),
        may_execute: granted.may_execute,
        capsule_id: selection.capsule_id,
        task_id: selection.task_id,
        program: selection.program.clone(),
        prompt: selection.prompt.clone(),
    };
    launch(&approved).await
}

/// Ask the running owner what it is holding.
///
/// A client of `Agent1`, not a second reader of the host. Two things walking the launch directory
/// would be two answers to *what is running*, and the one that is not the owner would be wrong the
/// moment a session started or ended between its listing and its reading.
#[cfg(target_os = "linux")]
async fn sessions() -> Result<(), String> {
    use cybou_fabric::AGENT;

    let encoded: Vec<u8> = zbus::Connection::session()
        .await
        .map_err(|error| error.to_string())?
        .call_method(
            Some(AGENT.service),
            AGENT.object_path,
            Some(AGENT.interface),
            "Sessions",
            &(),
        )
        .await
        .map_err(|error| format!("{} is not answering: {error}", AGENT.service))?
        .body()
        .deserialize()
        .map_err(|error| error.to_string())?;

    let views: Vec<SessionView> =
        cybou_fabric::decode(&encoded).map_err(|error| error.to_string())?;
    println!(
        "{}",
        serde_json::to_string_pretty(&views).map_err(|error| error.to_string())?
    );
    Ok(())
}

#[cfg(not(target_os = "linux"))]
async fn sessions() -> Result<(), String> {
    Err("cybou-agentd serves capsules on Linux".to_owned())
}

/// Ask the running owner to end one session.
///
/// Through the owner rather than by stopping units directly, because the owner is what records
/// *why* it ended. A session whose units were stopped behind its back would be reported as an agent
/// that finished, and nobody would be able to tell that from one that did.
#[cfg(target_os = "linux")]
async fn stop(selection: &Selection) -> Result<(), String> {
    use cybou_fabric::AGENT;

    let capsule_id = required(selection.capsule_id, "--capsule-id")?;
    let stopped: bool = zbus::Connection::session()
        .await
        .map_err(|error| error.to_string())?
        .call_method(
            Some(AGENT.service),
            AGENT.object_path,
            Some(AGENT.interface),
            "Stop",
            &(capsule_id.to_string(),),
        )
        .await
        .map_err(|error| format!("{} is not answering: {error}", AGENT.service))?
        .body()
        .deserialize()
        .map_err(|error| error.to_string())?;

    if stopped {
        println!("session {capsule_id} stopped");
    } else {
        // Not an error. In both cases the session is over by the time this is answered, and telling
        // a person their request failed because somebody else stopped it first would be reporting a
        // difference that does not matter to them.
        println!("session {capsule_id} was not running");
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
async fn stop(_selection: &Selection) -> Result<(), String> {
    Err("cybou-agentd serves capsules on Linux".to_owned())
}
