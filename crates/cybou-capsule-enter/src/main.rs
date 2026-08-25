// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The last thing that runs before an agent does.
//!
//! Two of the ten things a capsule is made of cannot be done by building an argument vector.
//! Landlock is a restriction a process applies **to itself**, inherited across `exec` and never
//! removable afterwards; a seccomp filter is installed the same way, and bubblewrap's own
//! `--seccomp` wants a file descriptor, which is not a thing a command line can carry. Both have to
//! happen inside a process, at the last moment before the agent replaces it.
//!
//! So the capsule is built in two hops:
//!
//! ```text
//! systemd-run … -- bwrap … -- cybou-capsule-enter … -- the agent
//! ```
//!
//! This runs after bubblewrap has finished, which is why the paths it is given are the ones seen
//! *inside* the capsule and not on the host. Landlock applied before bubblewrap would have to permit
//! everything bubblewrap needs in order to build the sandbox, which is most of the host.
//!
//! ## Why this cannot be used to widen anything
//!
//! An agent that finds this binary on its path can run it. That is fine, and it is worth being
//! explicit about why: Landlock and seccomp are monotonic. A process may add restrictions to itself
//! and may never remove one, so invoking this again with a generous list produces a process that is
//! bounded by the intersection — which is what it already had. There is no argument to this program
//! that grants anything.
//!
//! ## Failing loudly
//!
//! If the kernel will not enforce what was asked, this refuses to `exec`. A version that carried on
//! would produce an agent running with no second barrier and no record of the fact, which is the
//! shape of failure this repository keeps finding: a check that did not run, read afterwards as a
//! check that passed.

#[cfg(target_os = "linux")]
mod reshaping;

use std::path::PathBuf;

/// What this was asked to do.
struct Request {
    /// Paths the capsule may read and execute from.
    readable: Vec<PathBuf>,
    /// Paths the capsule may also write to.
    writable: Vec<PathBuf>,
    /// Optional capsule-local compatibility transport.
    egress: Option<EgressBridge>,
    /// The agent, and its arguments.
    program: Vec<String>,
}

/// Everything needed to start transport, and nothing about egress policy.
struct EgressBridge {
    program: PathBuf,
    socket: PathBuf,
    port: u16,
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("cybou-capsule-enter: {why}");
            // Distinct from anything the agent could exit with, so a capsule that never started is
            // never mistaken in a record for one that started and failed.
            std::process::ExitCode::from(78)
        }
    }
}

fn run() -> Result<(), String> {
    let request = parse(std::env::args().skip(1))?;
    restrict(&request)?;
    refuse_reshaping()?;
    if let Some(egress) = &request.egress {
        start_egress_bridge(egress)?;
    }
    execute(&request.program)
}

/// Read the argument list.
///
/// Deliberately plain flags rather than an encoded blob. What a capsule was held to should be
/// legible in `ps` and in a journal line, by somebody who has not read this crate.
fn parse(arguments: impl Iterator<Item = String>) -> Result<Request, String> {
    let mut readable = Vec::new();
    let mut writable = Vec::new();
    let mut program = Vec::new();
    let mut egress_bridge = None;
    let mut egress_socket = None;
    let mut egress_port = None;
    let mut arguments = arguments.peekable();

    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--ro" => readable.push(PathBuf::from(arguments.next().ok_or("--ro wants a path")?)),
            "--rw" => writable.push(PathBuf::from(arguments.next().ok_or("--rw wants a path")?)),
            "--egress-bridge" => {
                egress_bridge = Some(PathBuf::from(
                    arguments.next().ok_or("--egress-bridge wants a path")?,
                ));
            }
            "--egress-socket" => {
                egress_socket = Some(PathBuf::from(
                    arguments.next().ok_or("--egress-socket wants a path")?,
                ));
            }
            "--egress-port" => {
                let value = arguments.next().ok_or("--egress-port wants a number")?;
                let port = value
                    .parse::<u16>()
                    .map_err(|_| format!("{value} is not a port"))?;
                if port == 0 {
                    return Err("zero is not an egress port".to_owned());
                }
                egress_port = Some(port);
            }
            "--" => {
                program.extend(arguments.by_ref());
                break;
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    if program.is_empty() {
        return Err("no program after `--`; a capsule with nothing to run is not a capsule".into());
    }
    // A capsule with no writable path could not do the work it was granted, and one with no readable
    // path could not start a program. Either is a mistake in whatever built this command line, and
    // running anyway would hide it.
    if readable.is_empty() {
        return Err("no readable path; nothing could be executed".into());
    }
    if writable.is_empty() {
        return Err("no writable path; the agent could not do anything it was granted".into());
    }
    let egress = match (egress_bridge, egress_socket, egress_port) {
        (None, None, None) => None,
        (Some(program), Some(socket), Some(port)) => Some(EgressBridge {
            program,
            socket,
            port,
        }),
        _ => return Err("egress bridge, socket and port must be supplied together".to_owned()),
    };
    Ok(Request {
        readable,
        writable,
        egress,
        program,
    })
}

/// Start the compatibility transport as one child in the capsule's own cgroup and namespaces.
///
/// It inherits Landlock and seccomp because both were applied before this fork. The parent then
/// becomes the agent, so there is no third supervisor and both processes count under one `TasksMax`.
fn start_egress_bridge(egress: &EgressBridge) -> Result<(), String> {
    let mut child = std::process::Command::new(&egress.program)
        .arg("--port")
        .arg(egress.port.to_string())
        .arg("--socket")
        .arg(&egress.socket)
        .spawn()
        .map_err(|why| {
            format!(
                "could not start egress bridge {}: {why}",
                egress.program.display()
            )
        })?;
    let address = (std::net::Ipv4Addr::LOCALHOST, egress.port);
    for _ in 0..100 {
        if std::net::TcpStream::connect(address).is_ok() {
            return Ok(());
        }
        if let Some(status) = child
            .try_wait()
            .map_err(|why| format!("could not inspect the egress bridge: {why}"))?
        {
            return Err(format!(
                "egress bridge stopped before it was ready: {status}"
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let _ = child.kill();
    Err("egress bridge did not become ready within one second".to_owned())
}

/// Apply Landlock to this process, so the agent inherits it and cannot shed it.
#[cfg(target_os = "linux")]
fn restrict(request: &Request) -> Result<(), String> {
    use landlock::{
        ABI, Access, AccessFs, RulesetAttr, RulesetCreatedAttr, RulesetStatus, path_beneath_rules,
    };

    // The newest set of rights this crate knows about. Best effort is the default and is what is
    // wanted: on an older kernel the rights that do not exist are dropped rather than the whole
    // ruleset failing. What is *not* accepted is the ruleset having no effect at all, which is
    // checked below rather than assumed.
    let abi = ABI::V5;

    let status = landlock::Ruleset::default()
        .handle_access(AccessFs::from_all(abi))
        .map_err(|why| format!("this kernel does not know the access rights asked for: {why}"))?
        .create()
        .map_err(|why| format!("could not create a Landlock ruleset: {why}"))?
        .add_rules(path_beneath_rules(
            &request.readable,
            AccessFs::from_read(abi),
        ))
        .map_err(|why| format!("could not grant reading: {why}"))?
        .add_rules(path_beneath_rules(
            &request.writable,
            AccessFs::from_all(abi),
        ))
        .map_err(|why| format!("could not grant writing: {why}"))?
        .restrict_self()
        .map_err(|why| format!("could not restrict this process: {why}"))?;

    match status.ruleset {
        // Everything asked for is held.
        RulesetStatus::FullyEnforced => Ok(()),
        // The kernel supports Landlock but not every right this asked for — an older ABI. The
        // boundary is real and narrower than requested, which is reported rather than hidden: an
        // operator reading a journal should be able to tell which kernel their agents ran on.
        RulesetStatus::PartiallyEnforced => {
            eprintln!(
                "cybou-capsule-enter: Landlock is enforced, but this kernel does not implement \
                 every right asked for"
            );
            Ok(())
        }
        // Nothing was applied. Refusing here is the whole point of reading the status: carrying on
        // would produce an agent with no second barrier and nothing in any record saying so.
        RulesetStatus::NotEnforced => Err(
            "this kernel does not enforce Landlock, so the capsule would have one barrier \
                 where it is supposed to have two"
                .into(),
        ),
    }
}

/// Refuse the calls that would let the capsule rearrange itself.
#[cfg(target_os = "linux")]
fn refuse_reshaping() -> Result<(), String> {
    reshaping::refuse_reshaping()
}

#[cfg(not(target_os = "linux"))]
fn refuse_reshaping() -> Result<(), String> {
    Err("a capsule is a Linux kernel arrangement; this platform has none of it".into())
}

/// There is no second barrier anywhere but Linux, and this is not built for anywhere else.
#[cfg(not(target_os = "linux"))]
fn restrict(_request: &Request) -> Result<(), String> {
    Err("a capsule is a Linux kernel arrangement; this platform has none of it".into())
}

/// Become the agent.
///
/// `exec` rather than spawn-and-wait: a supervising parent inside the capsule would be one more
/// process for the task ceiling to count, one more thing to signal when the lease ends, and a place
/// for an exit status to be lost. What the capsule's cgroup contains should be the agent.
#[cfg(target_os = "linux")]
fn execute(program: &[String]) -> Result<(), String> {
    use std::os::unix::process::CommandExt;

    let (command, arguments) = program.split_first().ok_or("no program to become")?;
    // `exec` only returns when it failed.
    Err(format!(
        "could not run {command}: {}",
        std::process::Command::new(command).args(arguments).exec()
    ))
}

#[cfg(not(target_os = "linux"))]
fn execute(_program: &[String]) -> Result<(), String> {
    Err("a capsule is a Linux kernel arrangement; this platform has none of it".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_of(arguments: &[&str]) -> Result<Request, String> {
        parse(arguments.iter().map(|argument| (*argument).to_owned()))
    }

    #[test]
    fn the_program_is_whatever_follows_the_separator() {
        // Including things that look like this program's own flags. An agent invoked as `--ro` is
        // an agent, and the separator is what says so.
        let request =
            parse_of(&["--rw", "/workspace", "--ro", "/usr", "--", "--ro", "x"]).expect("parses");
        assert_eq!(request.program, vec!["--ro".to_owned(), "x".to_owned()]);
        assert_eq!(request.readable, vec![PathBuf::from("/usr")]);
        assert_eq!(request.writable, vec![PathBuf::from("/workspace")]);
        assert!(request.egress.is_none());
    }

    #[test]
    fn a_capsule_with_nothing_to_run_is_refused() {
        assert!(parse_of(&["--ro", "/usr", "--rw", "/workspace"]).is_err());
        assert!(parse_of(&["--ro", "/usr", "--rw", "/workspace", "--"]).is_err());
    }

    #[test]
    fn a_command_line_that_grants_nothing_is_a_mistake_and_not_a_tight_capsule() {
        // Running anyway would produce an agent that cannot start, reported as an agent that
        // crashed — and the mistake is in whatever built this line.
        assert!(parse_of(&["--rw", "/workspace", "--", "sh"]).is_err());
        assert!(parse_of(&["--ro", "/usr", "--", "sh"]).is_err());
    }

    #[test]
    fn an_unknown_argument_is_refused_rather_than_ignored() {
        // A flag this version does not know is a flag from a newer builder, and guessing at what it
        // meant is guessing about a boundary.
        assert!(parse_of(&["--everything", "--", "sh"]).is_err());
    }

    #[test]
    fn a_missing_path_is_not_an_empty_one() {
        assert!(parse_of(&["--ro"]).is_err());
        assert!(parse_of(&["--rw"]).is_err());
    }

    #[test]
    fn bridge_plumbing_is_all_or_nothing() {
        let complete = parse_of(&[
            "--ro",
            "/usr",
            "--rw",
            "/workspace",
            "--egress-bridge",
            "/.cybou-egress-bridge",
            "--egress-socket",
            "/run/cybou/egress.sock",
            "--egress-port",
            "3128",
            "--",
            "agent",
        ])
        .expect("complete bridge");
        let egress = complete.egress.expect("egress");
        assert_eq!(egress.port, 3128);
        assert!(
            parse_of(&[
                "--ro",
                "/usr",
                "--rw",
                "/workspace",
                "--egress-port",
                "3128",
                "--",
                "agent"
            ])
            .is_err()
        );
    }
}
