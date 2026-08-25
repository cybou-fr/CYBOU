// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Authorized Action and Governed Remediation Boundary (ADR-0022 & ADR-0036).
//!
//! Separates action proposals, automated criticism checks, policy authorization,
//! and typed execution from unchecked model output.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Assessed risk category of an action proposal.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RiskLevel {
    /// Read-only inspection or reversible cosmetic change.
    Low,
    /// Service restart or bounded operational mutation.
    Medium,
    /// Package upgrade, configuration overwrite, or network firewall change.
    High,
    /// Irreversible deletion, disk formatting, or system shutdown.
    Critical,
}

/// A structured proposal for mutating external system state.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionProposal {
    /// Unique proposal identifier.
    pub proposal_id: Uuid,
    /// Who is asking.
    ///
    /// Absent until 2026-08-25, and its absence was a live defect the moment agents were designed.
    /// A person pre-authorizes an operation because *Cybou's own diagnosis* is trustworthy — it came
    /// from readings Cybou gathered itself and cited as evidence. A flat list of permitted
    /// operations then extends that trust to anybody who asks for the same verb, including a party
    /// inside a capsule that Cybou trusts not at all.
    pub proposed_by: Proposer,
    /// Causal intention or problem that gave rise to this proposal.
    pub cause_id: Option<Uuid>,
    /// High-level communicative intent.
    pub intent: String,
    /// Typed operation verb (e.g. "service.restart", "package.install", "firewall.block").
    pub operation: String,
    /// Target resource identifier.
    pub target_resource: String,
    /// Operation parameters and payload.
    pub parameters: Vec<(String, String)>,
    /// Evaluated risk level.
    pub risk_level: RiskLevel,
    /// Whether the action has an automated rollback mechanism.
    pub reversible: bool,
    /// Proposal creation timestamp.
    #[serde(with = "time::serde::rfc3339")]
    pub proposed_at: OffsetDateTime,
}

/// Who is asking for an action.
///
/// The distinction is not about politeness. Cybou's own proposal arrives with a finding behind it
/// that the critics can check the action against; an agent's arrives from a party that may be
/// mistaken, confused, or hostile, with nothing but a request. Those are different questions and a
/// policy that could not tell them apart would answer both with the more permissive one.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "proposer")]
pub enum Proposer {
    /// Cybou, from a finding it reached about itself.
    Mind,
    /// An agent inside a capsule.
    #[serde(rename_all = "camelCase")]
    Agent {
        /// Which capsule it is running in.
        capsule_id: Uuid,
        /// Which agent.
        agent: String,
    },
}

impl Proposer {
    /// How this reads to a person.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Mind => "this host, from its own readings".to_owned(),
            Self::Agent { agent, capsule_id } => {
                format!("the agent {agent} in capsule {capsule_id}")
            }
        }
    }

    /// Whether this party is one whose own account of things may be relied on.
    ///
    /// True only for Mind, and not because Mind is clever: because a proposal from Mind carries a
    /// finding, and a finding carries the readings behind it. What is trusted is the evidence, not
    /// the proposer.
    #[must_use]
    pub const fn brings_its_own_evidence(&self) -> bool {
        matches!(self, Self::Mind)
    }
}

/// Result of an automated criticism rule evaluating a proposal before authorization.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CriticismCheck {
    /// Evaluation rule identifier.
    pub rule_id: String,
    /// Human-readable rule description.
    pub description: String,
    /// Whether the check passed without objection.
    pub passed: bool,
    /// Diagnostic objection or risk detail if check failed.
    pub objection: Option<String>,
}

/// Authorization verdict reached by the Mind governance engine.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AuthorizationVerdict {
    /// Pre-authorized for immediate execution under standing policy.
    Granted,
    /// Safe but exceeds automated authority; requires explicit user approval.
    RequiresUserConfirmation {
        /// User confirmation prompt.
        prompt: String,
    },
    /// Refused by policy, criticism check, or security boundary.
    Denied {
        /// Reason for refusal.
        reason: String,
    },
}

/// Final authorization decision record.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationDecision {
    /// Unique identifier for this decision.
    ///
    /// Without it an attempt can be traced to the proposal it carried out and not to the
    /// permission it rested on, and *what was authorized* — one of the five things ADR-0022 says
    /// every attempted action must be able to answer — has nothing to point at. Derived rather than
    /// generated, so a decision read twice is one decision.
    pub decision_id: Uuid,
    /// Proposal being authorized.
    pub proposal_id: Uuid,
    /// Verdict.
    pub verdict: AuthorizationVerdict,
    /// Specific capability grants checked.
    pub checked_capabilities: Vec<String>,
    /// Instant when authorization decision was finalized.
    #[serde(with = "time::serde::rfc3339")]
    pub decided_at: OffsetDateTime,
}

/// The only Body operations the first executor can perform.
///
/// This is deliberately not an open verb plus arguments. A new variant is a new physical
/// capability and therefore requires a code change on both sides of the action boundary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "action")]
pub enum ExecutableAction {
    /// Read one concrete systemd service's state.
    ServiceStatus {
        /// A concrete unit name ending in `.service`.
        unit: String,
    },
    /// Delete only the package manager's downloaded archive cache.
    PackageCacheClean,
    /// Restart one concrete systemd service.
    ServiceRestart {
        /// A concrete unit name ending in `.service`.
        unit: String,
    },
}

/// A short-lived, single-use capability minted from one granted decision.
///
/// The executor never accepts an operation from its caller. It receives only a permit identity,
/// atomically claims that identity from Action1, and performs the typed action stored here.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionPermit {
    /// Opaque bearer identity used exactly once.
    pub permit_id: Uuid,
    /// The decision that created this capability.
    pub decision_id: Uuid,
    /// The proposal that decision concerned.
    pub proposal_id: Uuid,
    /// The complete action; no caller-supplied arguments are added later.
    pub action: ExecutableAction,
    /// When Action1 minted the permit.
    #[serde(with = "time::serde::rfc3339")]
    pub issued_at: OffsetDateTime,
    /// Last instant at which it may be atomically claimed.
    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: OffsetDateTime,
}

/// The namespace authorization identities are derived in.
const DECISION_NAMESPACE: Uuid = Uuid::from_u128(0x6379_626f_755f_6465_6369_7369_6f6e_5f31);

impl AuthorizationDecision {
    /// The identity one decision has.
    ///
    /// Derived from the proposal, the verdict and the instant, so the same decision is the same
    /// decision on a second read. A random identity per read would mean an attempt citing one could
    /// not be checked against the record afterwards, which is the whole reason to carry it.
    #[must_use]
    pub fn derive_id(
        proposal_id: Uuid,
        verdict: &AuthorizationVerdict,
        decided_at: OffsetDateTime,
    ) -> Uuid {
        let verdict = match verdict {
            AuthorizationVerdict::Granted => "granted".to_owned(),
            AuthorizationVerdict::RequiresUserConfirmation { prompt } => {
                format!("requires-confirmation|{prompt}")
            }
            AuthorizationVerdict::Denied { reason } => format!("denied|{reason}"),
        };
        let seed = format!("{proposal_id}|{verdict}|{}", decided_at.unix_timestamp());
        Uuid::new_v5(&DECISION_NAMESPACE, seed.as_bytes())
    }
}

/// What the thing that carried out an action says about itself.
///
/// One half of an outcome, and the half that cannot be trusted alone. An executor reporting its own
/// success is the system grading its own homework: `apt clean` exits zero on a filesystem that is
/// still full, and a restart returns success for a unit that comes back up and immediately dies.
/// This is kept as *what was claimed*, beside what was independently observed, and the two are
/// separate fields precisely so they can disagree in public.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "report")]
pub enum AttemptReport {
    /// The operation ran and reported no error.
    Completed,
    /// The operation ran and reported an error.
    #[serde(rename_all = "camelCase")]
    Failed {
        /// What it said went wrong.
        because: String,
    },
    /// The operation did not run, because something declined to run it.
    ///
    /// Distinct from failing. Nothing was attempted, so nothing needs undoing, and the readings
    /// afterwards say nothing about this operation either way.
    #[serde(rename_all = "camelCase")]
    Refused {
        /// Why it was declined.
        because: String,
    },
    /// The operation began and this host does not know how it ended.
    ///
    /// A process killed, a machine rebooted mid-operation, a report lost. The honest state, and not
    /// the same as failure: something may well have happened.
    DidNotFinish,
}

impl AttemptReport {
    /// The short name a surface labels this with.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Failed { .. } => "failed",
            Self::Refused { .. } => "refused",
            Self::DidNotFinish => "did-not-finish",
        }
    }

    /// Whether anything was attempted at all.
    #[must_use]
    pub const fn was_attempted(&self) -> bool {
        !matches!(self, Self::Refused { .. })
    }
}

/// Why the readings cannot say whether something was relieved.
///
/// Its own value rather than an absence, because *I cannot tell* and *it did not work* are
/// different answers and only one of them is a reason to try something else.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "because")]
pub enum CannotTell {
    /// Nothing read the relevant thing after the attempt.
    ///
    /// A watched thing that went unreadable, or a probe that has not run since. The state before is
    /// known and the state after is not, and a comparison between them would be a comparison with
    /// one side missing.
    NotReadAfterwards,
    /// The attempt is too recent for the readings to have caught up.
    ///
    /// A restart takes longer than a sample interval. Reading immediately afterwards and declaring
    /// failure would condemn every operation that is not instantaneous.
    TooSoon,
    /// Nothing was attempted, so there is nothing to have relieved.
    NothingWasAttempted,
}

/// What the readings say afterwards, taken by something that did not carry the action out.
///
/// The other half of an outcome. This is the half that decides, and it is derived from telemetry
/// gathered independently rather than from anything the executor said.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "relief")]
pub enum Relief {
    /// The condition the action claimed to relieve is no longer found.
    Relieved,
    /// It is still found.
    StillPresent,
    /// It is still found, and further from ordinary than before.
    ///
    /// Kept apart from `StillPresent` because it is the one shape that argues against trying the
    /// same thing again.
    Worse,
    /// The readings cannot say.
    #[serde(rename_all = "camelCase")]
    NotEstablished {
        /// Why not.
        because: CannotTell,
    },
}

impl Relief {
    /// The short name a surface labels this with.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Relieved => "relieved",
            Self::StillPresent => "still-present",
            Self::Worse => "worse",
            Self::NotEstablished { .. } => "not-established",
        }
    }
}

/// Whether what was claimed and what was observed tell the same story.
///
/// A first-class value rather than something a reader is left to work out, because the case that
/// matters most is the one where they differ. An action that reported success while the condition
/// it addressed is still there is the single most important thing this whole path can produce: it
/// says the operation is not the remedy somebody thought it was, and it is invisible to anything
/// that records only what the executor claimed.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "agreement")]
pub enum Agreement {
    /// The claim and the readings say the same thing.
    Agree,
    /// They do not.
    #[serde(rename_all = "camelCase")]
    Disagree {
        /// What the disagreement is, in a form a person can read.
        about: String,
    },
    /// One of the two has nothing to say, so there is nothing to compare.
    NotComparable,
}

/// What happened when an action was attempted, from both sides.
///
/// ADR-0022 requires that every attempted action produce enough typed state to say what was
/// proposed, what was authorized, what was attempted, what actually happened, and whether rollback
/// remains available. This is the *what actually happened* part, and it is deliberately two
/// answers: what the executor said, and what the readings show.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionOutcome {
    /// Unique identifier for this outcome.
    pub outcome_id: Uuid,
    /// The attempt it describes.
    pub attempt_id: Uuid,
    /// The proposal that was attempted.
    pub proposal_id: Uuid,
    /// The finding the action claimed to relieve, if it named one.
    ///
    /// Without it there is nothing to check the action against, and the outcome can only ever
    /// repeat what the executor said.
    pub cause_id: Option<Uuid>,
    /// What the thing that carried it out said about itself.
    pub reported: AttemptReport,
    /// What the readings say afterwards.
    pub observed: Relief,
    /// Whether those two tell the same story.
    pub agreement: Agreement,
    /// Whether undoing it is still possible.
    ///
    /// A property of the operation and of what happened, not a promise. An operation that is
    /// reversible in principle is not reversible after the thing it would restore has gone.
    pub rollback_available: bool,
    /// When this was concluded.
    #[serde(with = "time::serde::rfc3339")]
    pub concluded_at: OffsetDateTime,
}

/// One typed value read by a Body adapter while performing an attempt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BodyReading {
    /// Stable field name such as `systemd.active-state`.
    pub field: String,
    /// The value returned by the owning host interface.
    pub value: String,
}

/// An attempt to carry out an authorized proposal.
///
/// Records what was tried and when, and nothing about whether it worked. Whether it worked is an
/// [`ActionOutcome`], concluded separately from readings this record has no access to.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionAttempt {
    /// Unique execution attempt identifier.
    pub attempt_id: Uuid,
    /// Proposal that was attempted.
    pub proposal_id: Uuid,
    /// The decision that permitted it.
    ///
    /// Carried so an attempt cannot be traced to a proposal without also being traceable to the
    /// authorization for it. An attempt whose authorization cannot be named is an attempt nobody
    /// can argue with afterwards.
    pub decision_id: Uuid,
    /// What was carried out.
    pub operation: String,
    /// What it was carried out on.
    pub target_resource: String,
    /// What the thing that carried it out said about itself.
    pub report: AttemptReport,
    /// Values read as part of a read-only adapter.
    ///
    /// These are adapter results, not the independent post-mutation observation used to conclude
    /// an [`ActionOutcome`]. Mutation adapters therefore normally leave this empty.
    #[serde(default)]
    pub body_readings: Vec<BodyReading>,
    /// Execution start timestamp.
    #[serde(with = "time::serde::rfc3339")]
    pub started_at: OffsetDateTime,
    /// When it stopped, if it did.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub ended_at: Option<OffsetDateTime>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_proposal_and_criticism_lifecycle() {
        let proposal = ActionProposal {
            proposal_id: Uuid::new_v4(),
            proposed_by: Proposer::Mind,
            cause_id: None,
            intent: "Recover unresponsive database".into(),
            operation: "service.restart".into(),
            target_resource: "systemd:postgresql.service".into(),
            parameters: vec![("mode".into(), "graceful".into())],
            risk_level: RiskLevel::Medium,
            reversible: true,
            proposed_at: OffsetDateTime::now_utc(),
        };

        assert_eq!(proposal.risk_level, RiskLevel::Medium);
        assert!(proposal.reversible);

        let check = CriticismCheck {
            rule_id: "no-destructive-during-active-session".into(),
            description: "Verify no active transactions before restart".into(),
            passed: true,
            objection: None,
        };
        assert!(check.passed);
    }
}
