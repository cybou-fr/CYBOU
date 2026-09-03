// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Everything one launch implies, worked out before anything is started.
//!
//! A launch touches four places on a host: a lease file, a launch file, a gateway unit and a capsule
//! unit. Deciding those inline, at the moment each is needed, is how the pieces drifted apart in the
//! first place — each call site had all the information it needed to invent a plausible value, so
//! each one did. Here they are derived once, from the lease, and the teardown that undoes them is
//! derived from the same place rather than written out separately and kept in step by hand.

use std::path::PathBuf;

use cybou_capsule::{CannotCompile, Ended, Lease};
use time::OffsetDateTime;
use uuid::Uuid;

/// Prefix of the runtime directory systemd creates for one gateway instance.
const RUNTIME_PREFIX: &str = "/run/cybou-agent-";

/// Prefix of the directory this owner creates for one session's own runtime files.
///
/// Separate from the gateway's. That one belongs to systemd, which creates and removes it with the
/// unit; sharing it would mean two owners for one directory and a broker socket that disappears when
/// an unrelated unit restarts.
const SESSION_PREFIX: &str = "/run/cybou-session-";

/// The bounds that belong to one model token rather than to the lease.
///
/// Deliberately not on the profile. A lease says what a person granted; these say how much of it one
/// task's bearer may draw at a time, which is an operational shape rather than an authority. Keeping
/// them apart is what lets the launch file exist at all without being a second grant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ceilings {
    /// Total input plus output tokens the bearer may consume.
    pub token_limit: u64,
    /// Per-request output ceiling.
    pub max_output_tokens: u32,
    /// Most exposing content the token may carry.
    pub sensitivity: u8,
}

impl Ceilings {
    /// The bounds of a bearer that will not exist.
    ///
    /// For a session granted no model. Nothing reads these — [`plan`] does not check them and no
    /// gateway is started to be bound by them — and they exist only because a launch carries one
    /// structure whether or not a model was selected. Requiring a caller to name them anyway made
    /// the profiles that need a model least the ones that could not be launched.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            token_limit: 0,
            max_output_tokens: 0,
            sensitivity: 0,
        }
    }
}

/// One session, as selected.
#[derive(Clone, Debug, PartialEq)]
pub struct Launch {
    /// The one authority. Everything else in the plan is read off it.
    pub lease: Lease,
    /// Which task of this agent the model bearer is for.
    pub task_id: Uuid,
    /// Per-token operational bounds.
    pub ceilings: Ceilings,
}

/// Why a launch cannot become a plan.
#[derive(Clone, Debug, PartialEq)]
pub enum CannotPlan {
    /// The lease was already over when the launch was attempted.
    LeaseOver(Ended),
    /// The lease grants no model, so there is no gateway for this session to hold.
    NoModelGrant,
    /// A zero ceiling would mint a bearer that permits nothing.
    EmptyCeiling,
    /// The granted capsule cannot be compiled into something the kernel can hold.
    ///
    /// Reachable even though the mint compiled the same grant: a lease travels between processes,
    /// and a value that arrives over a wire is checked again by anything that is going to act on it.
    GrantCannotCompile(CannotCompile),
}

impl core::fmt::Display for CannotPlan {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::LeaseOver(ended) => write!(formatter, "cannot launch: {}", ended.describe()),
            Self::NoModelGrant => formatter.write_str("cannot launch: the lease grants no model"),
            Self::EmptyCeiling => {
                formatter.write_str("cannot launch: a zero ceiling permits nothing")
            }
            Self::GrantCannotCompile(reason) => {
                write!(formatter, "cannot launch: the grant cannot run: {reason}")
            }
        }
    }
}

impl core::error::Error for CannotPlan {}

/// One step of undoing a launch.
///
/// Separate variants for the two units rather than one `StopUnit`, because the order between them is
/// the whole content of a teardown and a single variant would let it be reordered without anything
/// noticing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TeardownStep {
    /// Let a frozen capsule run again, so it can be ended.
    ///
    /// A frozen cgroup cannot act on being asked to exit: systemd sends its terminate signal, the
    /// processes never wake to handle it, and the stop waits out its whole timeout before killing
    /// anything. So the one thing that must happen before a paused or quarantined session can be
    /// ended is letting it run — for as long as it takes to die, and no longer. Thawing something
    /// that was never frozen changes nothing.
    ThawCapsule(String),
    /// End the capsule holding the agent.
    StopCapsule(String),
    /// End the private model gateway.
    StopGateway(String),
    /// End the capsule's egress broker.
    StopEgress(String),
    /// Remove a launch-time file.
    Remove(PathBuf),
}

/// Every path, unit and file body one launch implies.
#[derive(Clone, Debug, PartialEq)]
pub struct SessionPlan {
    /// The one authority this session runs under, carried so it can be written where the gateway
    /// reads it. Everything else in this structure was derived from it.
    pub lease: Lease,
    /// systemd instance name and session identity, the capsule's own UUID.
    pub instance: String,
    /// Where the authoritative lease is written for the gateway to read.
    pub lease_file: PathBuf,
    /// Where the non-authority launch values are written.
    pub launch_file: PathBuf,
    /// The exact body of the launch file.
    pub launch_environment: String,
    /// The templated gateway unit for this instance, when a model was granted.
    ///
    /// Absent for a capsule granted no model, in the same way the broker is absent for one granted
    /// no network. A gateway started for a session with nothing to serve refuses to come up, and a
    /// launch that started one anyway would fail on a surface it should never have asked for.
    pub gateway_unit: Option<String>,
    /// The runtime directory systemd creates for that unit, when there is one.
    pub gateway_runtime: Option<PathBuf>,
    /// The gateway socket the capsule is given, when a model was granted.
    pub model_socket: Option<PathBuf>,
    /// The ephemeral bearer file the capsule is given, when a model was granted.
    pub model_token: Option<PathBuf>,
    /// Where that gateway publishes what it has spent.
    ///
    /// Not given to the capsule. The agent has no business reading its own ledger from a file: an
    /// agent reporting its own consumption is the executor grading its own homework, and this is the
    /// figure that exists precisely so nobody has to ask it.
    pub model_usage: Option<PathBuf>,
    /// The transient unit the capsule itself runs under.
    pub capsule_unit: String,
    /// The directory this owner creates for the session's own runtime files.
    pub session_runtime: PathBuf,
    /// The one way out of this capsule, if it was granted any network.
    pub egress_socket: PathBuf,
    /// The transient unit this session's egress broker runs under.
    pub egress_unit: String,
    /// Exactly the hosts the approved grant permits, for the broker that decides by name.
    pub hosts: Vec<String>,
    /// When the lease runs out. The deadline the kernel is given, not a timer anybody watches.
    pub expires_at: OffsetDateTime,
}

/// Work out everything a launch implies, or refuse it.
///
/// # Errors
///
/// Returns [`CannotPlan`] when the lease is already over, grants no model, the token ceilings would
/// mint a bearer that permits nothing, or the granted capsule cannot compile. Each is refused here
/// rather than at the first component that happens to notice, so a session that cannot work never
/// has half of itself created.
pub fn plan(launch: &Launch, now: OffsetDateTime) -> Result<SessionPlan, CannotPlan> {
    if let Some(ended) = launch.lease.ended(now) {
        return Err(CannotPlan::LeaseOver(ended));
    }
    // A capsule with no model grant is not a broken launch. It is a capsule that was never going to
    // ask — the ordinary case on an unplugged host, and the one this system exists to survive.
    // Refusing it here made the Agent Capsule a container that only exists around a model.
    let wants_a_model = launch.lease.grant().model.is_some();
    if wants_a_model && (launch.ceilings.token_limit == 0 || launch.ceilings.max_output_tokens == 0)
    {
        return Err(CannotPlan::EmptyCeiling);
    }
    let spec =
        cybou_capsule::compile(launch.lease.grant()).map_err(CannotPlan::GrantCannotCompile)?;

    // The capsule's own identity, not a fresh one. A session named separately from the capsule it
    // holds is a session whose units cannot be traced back to it from the manager's list.
    let instance = launch.lease.grant().capsule_id.to_string();
    let runtime = wants_a_model.then(|| PathBuf::from(format!("{RUNTIME_PREFIX}{instance}")));
    let session_runtime = PathBuf::from(format!("{SESSION_PREFIX}{instance}"));
    let root = crate::lease_root();

    Ok(SessionPlan {
        lease: launch.lease.clone(),
        lease_file: root.join(format!("{instance}.lease")),
        launch_file: root.join(format!("{instance}.env")),
        launch_environment: launch_environment(launch),
        gateway_unit: wants_a_model.then(|| format!("cybou-agent-gateway@{instance}.service")),
        model_socket: runtime.as_ref().map(|runtime| runtime.join("model.sock")),
        model_token: runtime.as_ref().map(|runtime| runtime.join("model-token")),
        model_usage: runtime
            .as_ref()
            .map(|runtime| runtime.join("model-usage.json")),
        gateway_runtime: runtime,
        capsule_unit: cybou_capsule::unit_name(&spec),
        egress_socket: session_runtime.join("egress.sock"),
        egress_unit: format!("cybou-egress-{instance}"),
        hosts: launch.lease.grant().network.hosts.clone(),
        session_runtime,
        expires_at: launch.lease.expires_at(),
        instance,
    })
}

/// The launch file's body: what is *not* authority, and nothing else.
///
/// Every value the lease already carries is deliberately absent. When the gateway rebuilt its own
/// lease from this file, a wider lifetime or a stronger class written here produced a second
/// authority that nothing downstream could tell from the approved one.
fn launch_environment(launch: &Launch) -> String {
    format!(
        "CYBOU_AGENT_TASK_ID={}\nCYBOU_MODEL_TOKEN_LIMIT={}\nCYBOU_MODEL_MAX_OUTPUT_TOKENS={}\nCYBOU_MODEL_SENSITIVITY={}\n",
        launch.task_id,
        launch.ceilings.token_limit,
        launch.ceilings.max_output_tokens,
        launch.ceilings.sensitivity,
    )
}

impl SessionPlan {
    /// Undo this launch, in the one order that is safe.
    ///
    /// The capsule goes first. It holds the untrusted party, and stopping anything else before it
    /// leaves an agent running against a surface that has disappeared — which it experiences as a
    /// refusal it can retry, rather than as an ending. Ending is not asking, so the thing that can
    /// ask is the thing that stops first.
    ///
    /// The files go last, because they are the record of what this session was granted, and removing
    /// them while either unit is still up would leave a running process whose authority cannot be
    /// read back.
    #[must_use]
    pub fn teardown(&self) -> Vec<TeardownStep> {
        let mut steps = vec![
            TeardownStep::ThawCapsule(self.capsule_unit.clone()),
            TeardownStep::StopCapsule(self.capsule_unit.clone()),
        ];
        // Only what was started. Stopping a unit that never existed is a failure a person would go
        // and investigate, and what they would find is a grant correctly withheld.
        if let Some(gateway) = &self.gateway_unit {
            steps.push(TeardownStep::StopGateway(gateway.clone()));
        }
        if !self.hosts.is_empty() {
            steps.push(TeardownStep::StopEgress(self.egress_unit.clone()));
            steps.push(TeardownStep::Remove(self.egress_socket.clone()));
        }
        steps.push(TeardownStep::Remove(self.launch_file.clone()));
        steps.push(TeardownStep::Remove(self.lease_file.clone()));
        steps
    }
}

#[cfg(test)]
mod tests {
    use cybou_capsule::{
        CapabilityProfile, LeaseRequest, ModelGrant, NetworkGrant, ResourceBudget, SpendPolicy,
        Workspace, issue_lease,
    };
    use time::Duration;

    use super::*;

    fn at(offset: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_787_000_000 + offset).expect("a fixed instant")
    }

    const CAPSULE: Uuid = Uuid::from_u128(0x0704);

    fn lease(lifetime: Duration, model: Option<ModelGrant>) -> Lease {
        let mut profile = CapabilityProfile::bounded(
            "sandboxed-autonomous",
            ResourceBudget {
                memory_mib: 4096,
                cpus: 2,
                tasks_max: 512,
                lifetime,
            },
        )
        .expect("a valid profile");
        profile.network = NetworkGrant::to(&["github.com"]);
        profile.model = model;
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

    fn strong() -> ModelGrant {
        ModelGrant {
            class: "Strong".to_owned(),
            spend: SpendPolicy::Capped(100),
        }
    }

    fn launch(lease: Lease) -> Launch {
        Launch {
            lease,
            task_id: Uuid::from_u128(0x0705),
            ceilings: Ceilings {
                token_limit: 1000,
                max_output_tokens: 32,
                sensitivity: 1,
            },
        }
    }

    #[test]
    fn every_runtime_name_is_the_capsules_own_identity() {
        // One session, one name. A gateway instance named separately from the capsule it serves is
        // a pair nobody can match up again from a service manager's list.
        let plan = plan(&launch(lease(Duration::hours(4), Some(strong()))), at(1)).expect("a plan");

        assert_eq!(plan.instance, CAPSULE.to_string());
        let gateway = plan
            .gateway_unit
            .clone()
            .expect("a model grant has a gateway");
        let gateway_runtime = plan
            .gateway_runtime
            .clone()
            .expect("and a runtime directory");
        assert!(gateway.contains(&plan.instance));
        assert!(plan.capsule_unit.contains(&plan.instance));
        assert!(plan.lease_file.to_string_lossy().contains(&plan.instance));
        assert!(plan.launch_file.to_string_lossy().contains(&plan.instance));
        assert!(
            plan.model_socket
                .as_ref()
                .expect("a socket")
                .starts_with(&gateway_runtime)
        );
        assert!(
            plan.model_token
                .as_ref()
                .expect("a bearer file")
                .starts_with(&gateway_runtime)
        );
        assert!(plan.egress_unit.contains(&plan.instance));
        assert!(plan.egress_socket.starts_with(&plan.session_runtime));
        assert!(
            !plan.session_runtime.starts_with(&gateway_runtime),
            "the owner's directory and systemd's are not the same directory"
        );
    }

    #[test]
    fn the_launch_file_carries_nothing_the_lease_already_says() {
        // The defect this crate exists to close, asserted rather than described. Anything below that
        // appeared in the launch file could disagree with the approved lease, and a reader could not
        // tell which of the two a person had selected.
        let plan = plan(&launch(lease(Duration::hours(4), Some(strong()))), at(1)).expect("a plan");

        for absent in [
            "CYBOU_CAPSULE_ID",
            "CYBOU_AGENT_WORKSPACE",
            "CYBOU_AGENT_LEASE_SECONDS",
            "CYBOU_MODEL_CLASS",
            "CYBOU_MODEL_SPEND_LIMIT",
        ] {
            assert!(
                !plan.launch_environment.contains(absent),
                "{absent} is authority and must live on the lease alone"
            );
        }
        assert!(plan.launch_environment.contains("CYBOU_AGENT_TASK_ID="));
        assert!(plan.launch_environment.contains("CYBOU_MODEL_TOKEN_LIMIT="));
    }

    #[test]
    fn the_deadline_is_the_leases_own() {
        // Not a duration recomputed here. A second arithmetic on the same lifetime is a second
        // answer to when this session ends.
        let lease = lease(Duration::hours(4), Some(strong()));
        let expected = lease.expires_at();
        assert_eq!(
            plan(&launch(lease), at(1)).expect("a plan").expires_at,
            expected
        );
    }

    #[test]
    fn teardown_stops_the_capsule_before_anything_it_was_using() {
        // The agent is the untrusted party. Taking its gateway away first is a refusal it can see
        // and retry; taking the capsule away first is an ending it cannot.
        let plan = plan(&launch(lease(Duration::hours(4), Some(strong()))), at(1)).expect("a plan");
        let steps = plan.teardown();

        assert_eq!(
            &steps[..2],
            &[
                // Nothing can be ended while it is frozen, so the thaw comes before the stop and
                // before anything else: a capsule that cannot act on being asked to exit is one
                // whose teardown waits out a timeout instead of happening.
                TeardownStep::ThawCapsule(plan.capsule_unit.clone()),
                TeardownStep::StopCapsule(plan.capsule_unit.clone()),
            ]
        );
        assert_eq!(
            &steps[2..4],
            &[
                TeardownStep::StopGateway(plan.gateway_unit.clone().expect("a gateway")),
                TeardownStep::StopEgress(plan.egress_unit.clone()),
            ],
            "the surfaces the capsule was using go after the capsule itself"
        );
        assert_eq!(
            &steps[4..],
            &[
                TeardownStep::Remove(plan.egress_socket.clone()),
                TeardownStep::Remove(plan.launch_file.clone()),
                TeardownStep::Remove(plan.lease_file.clone()),
            ],
            "the record of what was granted outlives the things it granted"
        );
    }

    #[test]
    fn a_lease_that_is_over_creates_no_part_of_a_session() {
        // Refused here, whole, rather than by whichever component first noticed. Half a session is
        // the state that leaves runtime files nobody owns.
        let over = lease(Duration::hours(4), Some(strong()));
        let after = over.expires_at();
        assert_eq!(
            plan(&launch(over), after),
            Err(CannotPlan::LeaseOver(Ended::Expired))
        );

        let mut withdrawn = lease(Duration::hours(4), Some(strong()));
        withdrawn.revoke(at(60));
        assert_eq!(
            plan(&launch(withdrawn), at(61)),
            Err(CannotPlan::LeaseOver(Ended::Revoked))
        );
    }

    #[test]
    fn a_session_with_no_model_grant_is_an_ordinary_session_with_no_gateway() {
        // Refusing this was the defect. A capsule with no model grant was never going to ask, which
        // is the ordinary case on an unplugged host — and refusing it made the Agent Capsule a
        // container that only exists around a model rather than a bounded place to compute.
        let plan = plan(&launch(lease(Duration::hours(4), None)), at(1)).expect("a plan");

        assert!(plan.gateway_unit.is_none());
        assert!(plan.gateway_runtime.is_none());
        assert!(plan.model_socket.is_none());
        assert!(plan.model_token.is_none());
        assert!(
            !plan
                .teardown()
                .iter()
                .any(|step| matches!(step, TeardownStep::StopGateway(_))),
            "nothing was started, so nothing is stopped"
        );
    }

    #[test]
    fn a_session_with_no_model_ignores_token_ceilings_it_will_never_use() {
        // The ceilings bound a bearer. There is no bearer, so a zero one is not an empty authority
        // — it is a field that does not apply, and refusing on it would refuse every local capsule.
        let mut without = launch(lease(Duration::hours(4), None));
        without.ceilings.token_limit = 0;
        without.ceilings.max_output_tokens = 0;
        assert!(plan(&without, at(1)).is_ok());
    }

    #[test]
    fn a_zero_ceiling_is_refused_before_a_bearer_exists() {
        // Distinct from a zero *spending* ceiling, which is a real selection meaning "cost nothing".
        // A zero token ceiling is a bearer that cannot be used for anything at all.
        let mut empty = launch(lease(Duration::hours(4), Some(strong())));
        empty.ceilings.token_limit = 0;
        assert_eq!(plan(&empty, at(1)), Err(CannotPlan::EmptyCeiling));

        let mut no_output = launch(lease(Duration::hours(4), Some(strong())));
        no_output.ceilings.max_output_tokens = 0;
        assert_eq!(plan(&no_output, at(1)), Err(CannotPlan::EmptyCeiling));
    }

    #[test]
    fn a_zero_cost_model_still_launches() {
        // The distinction the lease module already draws, held here too. Spending nothing is a
        // selection, not an exhausted session.
        let free = lease(
            Duration::hours(4),
            Some(ModelGrant {
                class: "Local".to_owned(),
                spend: SpendPolicy::ZeroCostOnly,
            }),
        );
        assert!(plan(&launch(free), at(1)).is_ok());
    }
}
