// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The exact commands a launch runs, produced as data before any of them is run.
//!
//! Same discipline as [`cybou_capsule::under_budget`], and for the same reason: what a session did
//! to a host should be answerable by reading rather than by running. A coordinator that assembled
//! its commands inline, at the moment each was needed, would be a coordinator whose behaviour can
//! only be discovered by giving it a host to act on.
//!
//! ## Two service managers, on purpose
//!
//! ```text
//! the gateway   a system unit, because its provider credential is root-only
//! the capsule   a transient user unit, because nothing about it needs privilege
//! ```
//!
//! The split is not incidental. `cybou-agent-gateway@.service` is a system unit so the `LiteLLM` master
//! key can stay root-owned and reach an unprivileged process through `LoadCredential` rather than
//! through a file the `cybou` user can read. That is the only reason, and it is why an unprivileged
//! owner needs an explicit, narrow authorization to start it — deployment grants exactly
//! start and stop, on exactly that template, and nothing else.

use std::path::{Path, PathBuf};

use cybou_capsule::spec::Network;
use cybou_capsule::{
    BackendError, Bubblewrap, CapsuleBackend as _, CapsuleRuntimeBindings, KernelCapsuleSpec,
    under_budget,
};

use crate::plan::SessionPlan;

/// Where deployment installs the programs a capsule is built around.
const LIBEXEC: &str = "/usr/libexec/cybou";

/// The host programs one capsule needs, named rather than guessed at.
///
/// A default that fell back to some plausible location would build a capsule around an entry program
/// that might not be the one deployed, and the difference would not show up until a barrier was
/// missing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostPrograms {
    /// The program a capsule is entered through, which applies Landlock and seccomp before exec.
    pub entry: PathBuf,
    /// The capsule-local loopback bridge onto this session's model gateway.
    pub model_bridge: PathBuf,
    /// The capsule-local bridge onto this session's egress broker.
    pub egress_bridge: PathBuf,
    /// The host-side broker that decides, by name, where this capsule may connect.
    pub egress_broker: PathBuf,
}

impl Default for HostPrograms {
    fn default() -> Self {
        let root = Path::new(LIBEXEC);
        Self {
            entry: root.join("cybou-capsule-enter"),
            model_bridge: root.join("cybou-model-bridge"),
            egress_bridge: root.join("cybou-egress-bridge"),
            egress_broker: root.join("cybou-egressd"),
        }
    }
}

/// The command that starts this session's private model gateway, if it was granted a model.
///
/// A system unit, so this is the one step of a launch that an unprivileged owner cannot take on its
/// own authority. Nothing at all for a capsule with no model grant: a gateway with nothing to serve
/// refuses to come up, and asking for one would fail a launch on a surface it never needed.
#[must_use]
pub fn start_gateway(plan: &SessionPlan) -> Option<Vec<String>> {
    plan.gateway_unit
        .as_ref()
        .map(|unit| vec!["systemctl".to_owned(), "start".to_owned(), unit.clone()])
}

/// The command that starts this capsule's egress broker, or nothing if it was granted no network.
///
/// A transient user unit rather than a child of this process, for the same reason the capsule is
/// one: a way out that lives inside the coordinator is a way out that survives exactly as long as
/// the coordinator does, and outlives it in the worse direction if the coordinator is killed and the
/// broker is not.
///
/// The hosts come off the lease's own grant. The broker holds the network policy — this only tells
/// it which policy it is holding, and gets that from the one object a person approved.
#[must_use]
pub fn start_egress(
    plan: &SessionPlan,
    spec: &KernelCapsuleSpec,
    programs: &HostPrograms,
) -> Option<Vec<String>> {
    let Network::Brokered { .. } = spec.network else {
        return None;
    };
    let mut argv = vec![
        "systemd-run".to_owned(),
        "--user".to_owned(),
        format!("--unit={}", plan.egress_unit),
        "--collect".to_owned(),
        "--".to_owned(),
        programs.egress_broker.display().to_string(),
        "--socket".to_owned(),
        plan.egress_socket.display().to_string(),
    ];
    for host in &plan.hosts {
        argv.push("--host".to_owned());
        argv.push(host.clone());
    }
    Some(argv)
}

/// The command that ends this capsule's egress broker.
#[must_use]
pub fn stop_egress(plan: &SessionPlan) -> Vec<String> {
    vec![
        "systemctl".to_owned(),
        "--user".to_owned(),
        "stop".to_owned(),
        format!("{}.service", plan.egress_unit),
    ]
}

/// The command that ends this session's private model gateway, if it has one.
#[must_use]
pub fn stop_gateway(plan: &SessionPlan) -> Option<Vec<String>> {
    plan.gateway_unit
        .as_ref()
        .map(|unit| vec!["systemctl".to_owned(), "stop".to_owned(), unit.clone()])
}

/// The command that ends the capsule.
///
/// Stopping the unit, not signalling the process. The capsule is a cgroup with a kill switch on it;
/// asking whatever is inside to leave is a request, and a boundary made of requests is not one.
#[must_use]
pub fn stop_capsule(plan: &SessionPlan) -> Vec<String> {
    vec![
        "systemctl".to_owned(),
        "--user".to_owned(),
        "stop".to_owned(),
        format!("{}.service", plan.capsule_unit),
    ]
}

/// The command that runs `program` inside this session's capsule, under its budget.
///
/// The bindings come from the plan, so the capsule is given this session's own gateway socket and
/// its own bearer file and cannot be handed another session's by a caller that had both to hand.
///
/// # Errors
///
/// Returns [`BackendError`] when the compiled spec needs a bridge or a socket this launch does not
/// have — a brokered network with no broker, or a model grant with no gateway.
pub fn run_capsule(
    plan: &SessionPlan,
    spec: &KernelCapsuleSpec,
    programs: &HostPrograms,
    program: &[String],
) -> Result<Vec<String>, BackendError> {
    let mut backend = Bubblewrap::entering_through(programs.entry.clone());
    let mut bindings = CapsuleRuntimeBindings::default();

    if !matches!(spec.network, Network::Denied) {
        backend = backend.with_egress_bridge(programs.egress_bridge.clone());
        bindings.egress_socket_host = Some(plan.egress_socket.clone());
    }
    // The same rule as the broker above: a bridge mounted for a capsule that was granted no model
    // is a loopback endpoint nobody granted, sitting there waiting for something to find it.
    if let (Some(socket), Some(token)) = (&plan.model_socket, &plan.model_token) {
        backend = backend.with_model_bridge(programs.model_bridge.clone());
        bindings.model_socket_host = Some(socket.clone());
        bindings.model_token_host = Some(token.clone());
    }

    Ok(under_budget(
        spec,
        &backend.command(spec, &bindings, program)?,
    ))
}

#[cfg(test)]
mod tests {
    use cybou_capsule::{
        CapabilityProfile, Lease, LeaseRequest, ModelGrant, NetworkGrant, ResourceBudget,
        SpendPolicy, Workspace, compile, issue_lease,
    };
    use time::{Duration, OffsetDateTime};
    use uuid::Uuid;

    use super::*;
    use crate::plan::{Ceilings, Launch, plan};

    fn at(offset: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_787_000_000 + offset).expect("a fixed instant")
    }

    const CAPSULE: Uuid = Uuid::from_u128(0x07b1);

    fn lease(hosts: &[&str]) -> Lease {
        let mut profile = CapabilityProfile::bounded(
            "sandboxed-autonomous",
            ResourceBudget {
                memory_mib: 4096,
                cpus: 2,
                tasks_max: 512,
                lifetime: Duration::hours(4),
            },
        )
        .expect("a valid profile");
        profile.network = NetworkGrant::to(hosts);
        profile.model = Some(ModelGrant {
            class: "Strong".to_owned(),
            spend: SpendPolicy::Capped(100),
        });
        profile.may_execute = true;
        issue_lease(
            LeaseRequest {
                selected_profile: profile,
                capsule_id: CAPSULE,
                agent: "opencode".to_owned(),
                workspace: Workspace::at("/srv/project"),
            },
            at(0),
        )
        .expect("a lease is issued")
    }

    fn session(hosts: &[&str]) -> (SessionPlan, KernelCapsuleSpec) {
        let lease = lease(hosts);
        let spec = compile(lease.grant()).expect("compiles");
        let plan = plan(
            &Launch {
                lease,
                task_id: Uuid::from_u128(0x07b2),
                ceilings: Ceilings {
                    token_limit: 1000,
                    max_output_tokens: 32,
                    sensitivity: 1,
                },
            },
            at(1),
        )
        .expect("a plan");
        (plan, spec)
    }

    fn socket(plan: &SessionPlan) -> String {
        plan.model_socket
            .as_ref()
            .expect("a model grant has a socket")
            .display()
            .to_string()
    }

    fn programs() -> HostPrograms {
        HostPrograms {
            entry: PathBuf::from("/usr/libexec/cybou/cybou-capsule-enter"),
            model_bridge: PathBuf::from("/usr/libexec/cybou/cybou-model-bridge"),
            egress_bridge: PathBuf::from("/usr/libexec/cybou/cybou-egress-bridge"),
            egress_broker: PathBuf::from("/usr/libexec/cybou/cybou-egressd"),
        }
    }

    #[test]
    fn the_gateway_is_started_and_stopped_by_the_same_name() {
        // A stop that named a differently-derived unit would leave a live gateway holding a bearer
        // for a session that a person believes is over.
        let (plan, _) = session(&["github.com"]);
        let started = start_gateway(&plan).expect("a model grant has a gateway");
        let stopped = stop_gateway(&plan).expect("a model grant has a gateway");
        assert_eq!(started.last(), plan.gateway_unit.as_ref());
        assert_eq!(stopped.last(), plan.gateway_unit.as_ref());
        assert_eq!(started[1], "start");
        assert_eq!(stopped[1], "stop");
    }

    #[test]
    fn the_capsule_is_stopped_as_a_unit_and_not_signalled() {
        // Stopping the unit kills the cgroup. Signalling the process asks whatever is inside to
        // leave, and a boundary made of requests is not a boundary.
        let (plan, _) = session(&["github.com"]);
        let stop = stop_capsule(&plan);

        assert_eq!(stop[0], "systemctl");
        assert_eq!(stop[1], "--user", "the capsule needs no privilege");
        assert_eq!(
            stop.last().expect("a unit"),
            &format!("{}.service", plan.capsule_unit)
        );
    }

    #[test]
    fn the_capsule_is_given_this_sessions_own_gateway() {
        // The one binding a caller with two sessions in hand could get wrong. The socket and bearer
        // come from the plan rather than from an argument.
        let (plan, spec) = session(&["github.com"]);
        let argv = run_capsule(&plan, &spec, &programs(), &["/bin/true".to_owned()])
            .expect("a capsule command");
        let joined = argv.join(" ");

        assert!(joined.contains(&socket(&plan)));
        assert!(joined.contains(&plan.egress_socket.display().to_string()));
    }

    #[test]
    fn the_capsule_runs_under_the_budget_that_was_granted() {
        // The ceilings are the kernel's, and they are on the command rather than kept by this
        // process. A lifetime enforced by something that has to still be running ends when it does.
        let (plan, spec) = session(&["github.com"]);
        let argv = run_capsule(&plan, &spec, &programs(), &["/bin/true".to_owned()])
            .expect("a capsule command");
        let joined = argv.join(" ");

        assert_eq!(argv[0], "systemd-run");
        assert!(joined.contains(&format!("--unit={}", plan.capsule_unit)));
        assert!(joined.contains("MemoryMax=4096M"));
        assert!(joined.contains("MemorySwapMax=0"));
        assert!(joined.contains("TasksMax=512"));
        assert!(joined.contains("CPUQuota=200%"));
        assert!(joined.contains("RuntimeMaxSec=14400"));
    }

    #[test]
    fn a_capsule_with_no_network_is_given_no_broker() {
        // A bridge mounted for a capsule that was granted no network is a way out that nobody
        // granted, sitting there waiting for something to find it.
        let (plan, spec) = session(&[]);
        let argv = run_capsule(&plan, &spec, &programs(), &["/bin/true".to_owned()])
            .expect("a capsule command");
        let joined = argv.join(" ");

        assert!(!joined.contains("cybou-egress-bridge"));
        assert!(!joined.contains(&plan.egress_socket.display().to_string()));
        assert!(
            joined.contains(&socket(&plan)),
            "a capsule with no network still has its model gateway"
        );
    }

    #[test]
    fn the_broker_is_told_exactly_the_hosts_that_were_granted() {
        // The broker holds the policy; this only tells it which policy it holds. A host added here
        // would be a destination nobody granted, and one dropped would be a grant nobody honoured.
        let (plan, spec) = session(&["github.com", "registry.npmjs.org"]);
        let argv =
            start_egress(&plan, &spec, &programs()).expect("a brokered capsule has a broker");

        let hosts: Vec<&String> = argv
            .iter()
            .enumerate()
            .filter(|(index, _)| *index > 0 && argv[index - 1] == "--host")
            .map(|(_, host)| host)
            .collect();
        assert_eq!(hosts, vec!["github.com", "registry.npmjs.org"]);
        assert!(argv.contains(&plan.egress_socket.display().to_string()));
        assert!(argv.contains(&format!("--unit={}", plan.egress_unit)));
    }

    #[test]
    fn a_capsule_with_no_network_gets_no_broker_at_all() {
        // Not a broker that permits nothing. A broker that exists is a socket that exists, and a
        // socket nobody granted is a way out waiting for something to find it.
        let (plan, spec) = session(&[]);
        assert_eq!(start_egress(&plan, &spec, &programs()), None);
    }

    #[test]
    fn the_broker_outlives_this_process_and_is_stopped_by_unit() {
        // A way out that lives inside the coordinator survives exactly as long as the coordinator,
        // and in the worse direction if the coordinator is killed and the broker is not.
        let (plan, spec) = session(&["github.com"]);
        let start = start_egress(&plan, &spec, &programs()).expect("a broker");
        assert_eq!(start[0], "systemd-run");

        let stop = stop_egress(&plan);
        assert_eq!(stop[1], "--user");
        assert_eq!(
            stop.last().expect("a unit"),
            &format!("{}.service", plan.egress_unit)
        );
    }

    #[test]
    fn the_program_is_the_last_thing_on_the_command() {
        // Everything after the separator is the program. A program whose name begins with a dash is
        // a program, not an option, at both of the two commands this wraps.
        let (plan, spec) = session(&["github.com"]);
        let argv = run_capsule(
            &plan,
            &spec,
            &programs(),
            &["/bin/echo".to_owned(), "-n".to_owned()],
        )
        .expect("a capsule command");

        assert_eq!(
            argv[argv.len() - 2..],
            ["/bin/echo".to_owned(), "-n".to_owned()]
        );
    }
}
