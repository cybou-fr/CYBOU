// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Owner of the authorized-action lifecycle described by ADR-0022.

use std::{collections::HashMap, sync::Mutex};

use cybou_protocol::{
    action::{
        ActionOutcome, ActionProposal, AuthorizationDecision, AuthorizationVerdict, CriticismCheck,
        ExecutableAction, ExecutionAttempt, ExecutionClaim, ExecutionPermit, ExecutionStarted,
        Proposer,
    },
    telemetry::SystemInsight,
};
use cybou_remediation::{
    Operation, StandingPolicy, authorize, criticise, criticise_request, propose,
};
use thiserror::Error;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

pub mod journal;

#[cfg(target_os = "linux")]
pub mod service;

/// How long an authorization remains a claimable capability.
const PERMIT_LIFETIME: Duration = Duration::seconds(60);

pub use cybou_protocol::action::ActionRecord;

/// A refusal at the action boundary.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ActionError {
    /// The requested operation is not one this build can propose.
    #[error("'{0}' is not an operation")]
    UnknownOperation(String),
    /// The action does not name a concrete executor target.
    #[error("the executor target is not concrete: {0}")]
    InvalidTarget(String),
    /// The permit is absent, expired, or was already consumed.
    #[error("permit is absent, expired, or already consumed")]
    PermitUnavailable,
    /// Internal lifecycle state could not be accessed.
    #[error("action lifecycle state is unavailable")]
    StateUnavailable,
    /// Nothing here proposed what an attempt or an outcome names.
    ///
    /// Refused rather than stored beside nothing. A record of what was done, with no record of what
    /// authorized it, is the one shape this owner exists to prevent.
    #[error("proposal {0} was not made here")]
    UnknownProposal(Uuid),
    /// A final report did not match the execution identity minted at permit claim.
    #[error("attempt {0} does not match the execution Action1 started")]
    AttemptMismatch(Uuid),
    /// The proposal is not one a person may answer for, in the way they tried to answer it.
    ///
    /// One variant for every refusal in [`ActionCore::confirm`] rather than one each. Which of
    /// them was tripped is exactly what a caller would need in order to keep trying, and a
    /// confirmation surface that reports how close a guess came is a way to search the lifecycle
    /// for something confirmable.
    #[error("this proposal is not awaiting a confirmation of that kind")]
    NotAwaitingConfirmation,
}

/// How long after a proposal was made a person may still answer it.
///
/// A proposal carries a diagnosis drawn from readings taken at one instant. Long enough afterwards
/// the readings are gone and confirming means agreeing to a claim nobody re-checked — the same
/// reason the permit that follows a confirmation is measured in seconds rather than hours.
pub const CONFIRMATION_WINDOW: Duration = Duration::minutes(15);

/// Owner of the action lifecycle.
///
/// Records are held here and written to the Journal by whatever owns the bus surface, so this type
/// stays free of I/O and remains the thing tests can reason about. Permits live only here: a
/// single-use sixty-second capability has no business surviving the process that issued it, and
/// [`crate::journal`] says why in full.
pub struct ActionCore {
    policy: StandingPolicy,
    records: Mutex<HashMap<Uuid, ActionRecord>>,
    permits: Mutex<HashMap<Uuid, ExecutionPermit>>,
}

impl ActionCore {
    /// Start Action1 with the standing policy an operator supplied.
    #[must_use]
    pub fn new(policy: StandingPolicy) -> Self {
        Self {
            policy,
            records: Mutex::new(HashMap::new()),
            permits: Mutex::new(HashMap::new()),
        }
    }

    /// Create, criticise, decide, and retain one proposal from a Mind finding.
    ///
    /// # Errors
    ///
    /// Refuses unknown verbs, invalid concrete targets, or unavailable internal state.
    pub fn evaluate_insight(
        &self,
        insight: &SystemInsight,
        verb: &str,
        now: OffsetDateTime,
    ) -> Result<ActionRecord, ActionError> {
        let operation = operation_for(verb)?;
        let proposal = propose(insight, now, |_| Uuid::new_v4())
            .into_iter()
            .find(|proposal| proposal.operation == verb)
            .ok_or_else(|| ActionError::UnknownOperation(verb.to_owned()))?;
        let mut checks = criticise(&proposal, insight);
        let adapter = executable_action(&proposal, operation);
        checks.push(CriticismCheck {
            rule_id: "executor-adapter-exists".to_owned(),
            description: "The first executor has a typed adapter for this action and target."
                .to_owned(),
            passed: adapter.is_ok(),
            objection: adapter.as_ref().err().map(ToString::to_string),
        });
        let decision = authorize(
            &proposal,
            &checks,
            insight.strength == cybou_protocol::telemetry::EvidenceStrength::Weak,
            &self.policy,
            now,
        );
        let permit = if matches!(decision.verdict, AuthorizationVerdict::Granted) {
            adapter
                .ok()
                .map(|action| permit_for(&proposal, &decision, action, now))
        } else {
            None
        };
        let permit_id = permit.as_ref().map(|permit| permit.permit_id);
        if let Some(permit) = permit {
            self.permits
                .lock()
                .map_err(|_| ActionError::StateUnavailable)?
                .insert(permit.permit_id, permit);
        }
        let record = ActionRecord {
            proposal,
            checks,
            decision,
            permit_id,
            execution_started: None,
            attempt: None,
            outcome: None,
        };
        self.records
            .lock()
            .map_err(|_| ActionError::StateUnavailable)?
            .insert(record.proposal.proposal_id, record.clone());
        Ok(record)
    }

    /// Atomically consume a permit. A second claim receives nothing.
    ///
    /// # Errors
    ///
    /// Refuses unknown, consumed, expired permits and unavailable internal state.
    /// Carry out what a person at a seat asked for, if the boundary allows it.
    ///
    /// `Action1`'s only entrance was `evaluate_insight`, which takes a finding this host reached
    /// about itself. That is the right shape for remediation and it leaves an operator looking at a
    /// failed unit on a panel built to show them exactly that, with nothing to press. ADR-0048
    /// opens the other door.
    ///
    /// **The asking is the confirmation.** A person who was looking at the panel, read the unit
    /// name and pressed the button has already answered the question a confirmation asks; deciding
    /// `RequiresUserConfirmation` and asking them again is asking the same person the same thing
    /// twice, which teaches them to click through it. So a request that passes criticism is
    /// `GrantedOnConfirmation`, naming the seat — never `Granted`, because no policy granted this
    /// and the record must not read as though one had.
    ///
    /// Everything the boundary already refuses, it still refuses. The operation table is closed, so
    /// a verb outside it is not an operation with unknown risk but not an operation; and the
    /// operations that are never offered are not offered to a person either, because there is no
    /// answer somebody could give that would make formatting a filesystem safe.
    ///
    /// What a person cannot bring is evidence. The critics that compare a proposal against its
    /// readings have nothing to compare, so what remains is the operation table — a smaller check
    /// than a finding gets, and the reason [`Proposer::brings_its_own_evidence`] answers false for
    /// them.
    ///
    /// # Errors
    ///
    /// [`ActionError::UnknownOperation`] for a verb this build cannot express, and
    /// [`ActionError::InvalidTarget`] for one that names nothing concrete to act on.
    pub fn request(
        &self,
        verb: &str,
        target: &str,
        seat: &str,
        now: OffsetDateTime,
    ) -> Result<ActionRecord, ActionError> {
        let operation = operation_for(verb)?;

        if seat.trim().is_empty() {
            // A request from nobody is not a request. The seat is established by whatever
            // authenticated the person, so an empty one means the caller had none to give.
            return Err(ActionError::InvalidTarget(
                "a request carries the seat that asked for it".to_owned(),
            ));
        }

        let proposal = ActionProposal {
            proposal_id: Uuid::new_v4(),
            proposed_by: Proposer::Person {
                seat: seat.to_owned(),
            },
            // No cause. A person's reason for wanting a service restarted is theirs, and inventing
            // a finding to point at would be this host claiming it had concluded something.
            cause_id: None,
            intent: format!("{seat} asked for {verb} on {target}"),
            operation: verb.to_owned(),
            target_resource: target.to_owned(),
            parameters: Vec::new(),
            // Taken from the operation rather than from the caller, which is the rule that makes
            // the risk check meaningful: a proposer cannot understate what it is asking for.
            risk_level: operation.risk(),
            reversible: operation.reversible(),
            proposed_at: now,
        };

        let mut checks = criticise_request(&proposal);
        let adapter = executable_action(&proposal, operation);
        checks.push(CriticismCheck {
            rule_id: "executor-adapter-exists".to_owned(),
            description: "The first executor has a typed adapter for this action and target."
                .to_owned(),
            passed: adapter.is_ok(),
            objection: adapter.as_ref().err().map(ToString::to_string),
        });

        let objected = checks.iter().any(|check| !check.passed);
        let verdict = if operation.forbidden() {
            // Forbidden means never, for anybody. A person asking does not make it askable,
            // for the same reason it is not offered to Mind: there is no answer somebody could
            // give that would make it safe.
            AuthorizationVerdict::Denied {
                reason: format!(
                    "{} is refused whatever the evidence says",
                    proposal.operation
                ),
            }
        } else if objected {
            AuthorizationVerdict::Denied {
                reason: "a criticism refused this request".to_owned(),
            }
        } else {
            AuthorizationVerdict::GrantedOnConfirmation {
                confirmed_by: seat.to_owned(),
            }
        };

        let decision = AuthorizationDecision {
            decision_id: AuthorizationDecision::derive_id(proposal.proposal_id, &verdict, now),
            proposal_id: proposal.proposal_id,
            verdict,
            checked_capabilities: vec![operation.verb().to_owned()],
            decided_at: now,
        };

        let permit = decision
            .verdict
            .permits_execution()
            .then(|| {
                adapter
                    .ok()
                    .map(|action| permit_for(&proposal, &decision, action, now))
            })
            .flatten();
        let permit_id = permit.as_ref().map(|permit| permit.permit_id);
        if let Some(permit) = permit {
            self.permits
                .lock()
                .map_err(|_| ActionError::StateUnavailable)?
                .insert(permit.permit_id, permit);
        }

        let record = ActionRecord {
            proposal,
            checks,
            decision,
            permit_id,
            execution_started: None,
            attempt: None,
            outcome: None,
        };
        self.records
            .lock()
            .map_err(|_| ActionError::StateUnavailable)?
            .insert(record.proposal.proposal_id, record.clone());
        Ok(record)
    }

    /// Grant a proposal that was waiting on a person, because a person said yes.
    ///
    /// This is the other half of a verdict the host has been able to reach since ADR-0022 and has
    /// never been able to act on: with no standing policy — the default, and the only state a
    /// fresh installation has — every proposal decides to
    /// [`RequiresUserConfirmation`](AuthorizationVerdict::RequiresUserConfirmation) and stops
    /// there, because nothing could carry the answer back.
    ///
    /// Four things are checked, and each of them is a way this could otherwise become a way around
    /// the boundary rather than a door through it.
    ///
    /// **The verdict must still be the one that asked.** A proposal already granted needs no
    /// confirmation, one already confirmed is spent, and one that was denied cannot be confirmed
    /// into existence — a person's agreement answers a question, it does not overrule a refusal.
    ///
    /// **The decision the person saw must be the decision that is here.** They are agreeing to a
    /// prompt, and a proposal re-decided between the moment it was drawn and the moment it was
    /// clicked is a different prompt. Without this the answer to one question authorizes another.
    ///
    /// **Every criticism must have passed.** The checks were run when the proposal was decided and
    /// are stored with it; a confirmation grants the half a person owns, and it does not revive a
    /// proposal the critics objected to. This is the same rule that already stops a failed critic
    /// from letting a pre-authorized operation through.
    ///
    /// **The proposal must be recent.** A proposal is made from a finding, and a finding is made
    /// from readings. An hour later the readings are gone, the disk that was filling may be empty,
    /// and confirming is agreeing to a diagnosis nobody re-checked. The window is deliberately
    /// short for the same reason the permit that follows it lasts sixty seconds.
    ///
    /// # Errors
    ///
    /// [`ActionError::UnknownProposal`] when nothing here proposed it, and
    /// [`ActionError::NotAwaitingConfirmation`] for every other refusal above — deliberately one
    /// variant, because telling a caller which of the four it tripped tells it how to keep trying.
    pub fn confirm(
        &self,
        proposal_id: Uuid,
        decision_seen: Uuid,
        confirmed_by: &str,
        now: OffsetDateTime,
    ) -> Result<ActionRecord, ActionError> {
        if confirmed_by.trim().is_empty() {
            return Err(ActionError::NotAwaitingConfirmation);
        }

        let mut records = self
            .records
            .lock()
            .map_err(|_| ActionError::StateUnavailable)?;
        let record = records
            .get_mut(&proposal_id)
            .ok_or(ActionError::UnknownProposal(proposal_id))?;

        if !matches!(
            record.decision.verdict,
            AuthorizationVerdict::RequiresUserConfirmation { .. }
        ) {
            return Err(ActionError::NotAwaitingConfirmation);
        }
        if record.decision.decision_id != decision_seen {
            return Err(ActionError::NotAwaitingConfirmation);
        }
        if record.checks.iter().any(|check| !check.passed) {
            return Err(ActionError::NotAwaitingConfirmation);
        }
        if now < record.proposal.proposed_at
            || now - record.proposal.proposed_at > CONFIRMATION_WINDOW
        {
            return Err(ActionError::NotAwaitingConfirmation);
        }

        // Rebuilt from the stored proposal rather than taken from the caller. The person confirmed
        // a proposal; they did not supply an action, and there is no field here for them to.
        let operation = operation_for(&record.proposal.operation)?;
        let action = executable_action(&record.proposal, operation)?;

        let verdict = AuthorizationVerdict::GrantedOnConfirmation {
            confirmed_by: confirmed_by.to_owned(),
        };
        let decision = AuthorizationDecision {
            decision_id: AuthorizationDecision::derive_id(proposal_id, &verdict, now),
            proposal_id,
            verdict,
            checked_capabilities: vec![operation.verb().to_owned()],
            decided_at: now,
        };
        let permit = permit_for(&record.proposal, &decision, action, now);

        record.permit_id = Some(permit.permit_id);
        record.decision = decision;
        let confirmed = record.clone();
        drop(records);

        self.permits
            .lock()
            .map_err(|_| ActionError::StateUnavailable)?
            .insert(permit.permit_id, permit);

        Ok(confirmed)
    }

    /// Atomically consume a permit. A second claim receives nothing.
    ///
    /// # Errors
    ///
    /// Refuses unknown, consumed, expired permits and unavailable internal state.
    pub fn claim_permit(
        &self,
        permit_id: Uuid,
        now: OffsetDateTime,
    ) -> Result<ExecutionClaim, ActionError> {
        let permit = self
            .permits
            .lock()
            .map_err(|_| ActionError::StateUnavailable)?
            .remove(&permit_id)
            .ok_or(ActionError::PermitUnavailable)?;
        if now > permit.expires_at {
            return Err(ActionError::PermitUnavailable);
        }
        let started = ExecutionStarted::from_permit(&permit, Uuid::new_v4(), now);
        let mut records = self
            .records
            .lock()
            .map_err(|_| ActionError::StateUnavailable)?;
        let record = records
            .get_mut(&permit.proposal_id)
            .ok_or(ActionError::UnknownProposal(permit.proposal_id))?;
        record.execution_started = Some(started.clone());
        Ok(ExecutionClaim { permit, started })
    }

    /// Read one retained lifecycle record.
    #[must_use]
    pub fn record(&self, proposal_id: Uuid) -> Option<ActionRecord> {
        self.records.lock().ok()?.get(&proposal_id).cloned()
    }

    /// Record what was carried out under one decision.
    ///
    /// Held against the proposal it names, so *why was this authorized* and *was it done* are one
    /// record rather than two that somebody has to correlate afterwards.
    ///
    /// # Errors
    ///
    /// Returns [`ActionError::StateUnavailable`] when the record table cannot be reached, and
    /// [`ActionError::UnknownProposal`] when nothing here proposed what the attempt names, and
    /// [`ActionError::AttemptMismatch`] when the report does not match the execution identity
    /// minted while claiming the permit.
    pub fn record_attempt(&self, attempt: ExecutionAttempt) -> Result<(), ActionError> {
        let mut records = self
            .records
            .lock()
            .map_err(|_| ActionError::StateUnavailable)?;
        let record = records
            .get_mut(&attempt.proposal_id)
            .ok_or(ActionError::UnknownProposal(attempt.proposal_id))?;
        let matches_started = record.execution_started.as_ref().is_some_and(|started| {
            started.attempt_id == attempt.attempt_id
                && started.proposal_id == attempt.proposal_id
                && started.decision_id == attempt.decision_id
                && started.operation == attempt.operation
                && started.target_resource == attempt.target_resource
                && started.started_at == attempt.started_at
        });
        if !matches_started {
            return Err(ActionError::AttemptMismatch(attempt.attempt_id));
        }
        record.attempt = Some(attempt);
        Ok(())
    }

    /// Record what the host saw afterwards.
    ///
    /// # Errors
    ///
    /// Returns [`ActionError::StateUnavailable`] when the record table cannot be reached, and
    /// [`ActionError::UnknownProposal`] when nothing here proposed what the outcome names.
    pub fn record_outcome(&self, outcome: ActionOutcome) -> Result<(), ActionError> {
        let mut records = self
            .records
            .lock()
            .map_err(|_| ActionError::StateUnavailable)?;
        let record = records
            .get_mut(&outcome.proposal_id)
            .ok_or(ActionError::UnknownProposal(outcome.proposal_id))?;
        record.outcome = Some(outcome);
        Ok(())
    }

    /// Every episode that was carried out and never concluded.
    ///
    /// What a driver has to ask for when it starts, and the question its own memory cannot answer.
    /// Asking only about findings still being reported would miss exactly the episodes that worked:
    /// a remedy that succeeded makes its finding disappear, so the successful case is the one whose
    /// conclusion would be lost forever.
    #[must_use]
    pub fn unfinished_episodes(&self) -> Vec<ActionRecord> {
        let Ok(records) = self.records.lock() else {
            return Vec::new();
        };
        records
            .values()
            .filter(|record| {
                (record.attempt.is_some() || record.execution_started.is_some())
                    && record.outcome.is_none()
            })
            .cloned()
            .map(recover_interrupted)
            .collect()
    }

    /// The most recent episode that was actually carried out for one cause.
    ///
    /// A cause may have more than one proposal because the remediation driver walks the remedy
    /// table until policy permits one. Refused proposals are history, but they are not what
    /// [`cybou_remediation::initiative`] needs when deciding whether an action may be repeated.
    /// Returning the latest attempted record also makes the answer independent of `HashMap`
    /// iteration order if an older Journal contains more than one attempt for the same cause.
    #[must_use]
    pub fn episode_for_cause(&self, cause_id: Uuid) -> Option<ActionRecord> {
        self.records
            .lock()
            .ok()?
            .values()
            .filter(|record| {
                record.proposal.cause_id == Some(cause_id)
                    && (record.attempt.is_some() || record.execution_started.is_some())
            })
            .max_by_key(|record| {
                let started_at = record
                    .attempt
                    .as_ref()
                    .map(|attempt| attempt.started_at)
                    .or_else(|| {
                        record
                            .execution_started
                            .as_ref()
                            .map(|started| started.started_at)
                    })
                    .unwrap_or(OffsetDateTime::UNIX_EPOCH);
                (
                    started_at,
                    record.proposal.proposed_at,
                    record.proposal.proposal_id,
                )
            })
            .cloned()
            .map(recover_interrupted)
    }

    /// All retained records for one cause finding, sorted newest first.
    #[must_use]
    pub fn records_for_cause(&self, cause_id: Uuid) -> Vec<ActionRecord> {
        let Ok(records) = self.records.lock() else {
            return Vec::new();
        };
        let mut list: Vec<ActionRecord> = records
            .values()
            .filter(|record| record.proposal.cause_id == Some(cause_id))
            .cloned()
            .map(recover_interrupted)
            .collect();
        list.sort_by_key(|b| std::cmp::Reverse(b.proposal.proposed_at));
        list
    }

    /// Every retained lifecycle record, newest first.
    #[must_use]
    pub fn recent_records(&self) -> Vec<ActionRecord> {
        let Ok(records) = self.records.lock() else {
            return Vec::new();
        };
        let mut list: Vec<ActionRecord> =
            records.values().cloned().map(recover_interrupted).collect();
        list.sort_by_key(|b| std::cmp::Reverse(b.proposal.proposed_at));
        list
    }

    /// Seed this owner with lifecycle records read back from the Journal.
    ///
    /// What a restarted Action1 knows. It restores what was proposed, argued and decided, and
    /// restores no permits — an authorization nobody claimed before the restart was not claimed, and
    /// re-offering it would resurrect a capability whose whole point is that it is used once.
    ///
    /// # Errors
    ///
    /// Returns [`ActionError::StateUnavailable`] when the internal record table cannot be reached.
    pub fn restore(&self, records: Vec<ActionRecord>) -> Result<usize, ActionError> {
        let mut held = self
            .records
            .lock()
            .map_err(|_| ActionError::StateUnavailable)?;
        for mut record in records {
            record.permit_id = None;
            held.insert(record.proposal.proposal_id, record);
        }
        Ok(held.len())
    }
}

fn recover_interrupted(mut record: ActionRecord) -> ActionRecord {
    if record.attempt.is_none() {
        record.attempt = record.execution_started.as_ref().map(|started| {
            started.finish(
                cybou_protocol::action::AttemptReport::DidNotFinish,
                Vec::new(),
                None,
            )
        });
    }
    record
}

fn operation_for(verb: &str) -> Result<Operation, ActionError> {
    cybou_remediation::ALL_OPERATIONS
        .iter()
        .copied()
        .find(|operation| operation.verb() == verb)
        .ok_or_else(|| ActionError::UnknownOperation(verb.to_owned()))
}

fn concrete_service(target: &str) -> Result<String, ActionError> {
    let unit = target
        .strip_prefix("systemd:")
        .ok_or_else(|| ActionError::InvalidTarget(target.to_owned()))?;
    let valid = unit.ends_with(".service")
        && unit != "<unit>.service"
        && !unit.contains('<')
        && !unit.contains('>')
        && unit.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'@' | b':')
        });
    if !valid {
        return Err(ActionError::InvalidTarget(target.to_owned()));
    }
    Ok(unit.to_owned())
}

/// Read `process:<uid>:<pid>` and refuse anything else.
///
/// Both halves are numbers because both are facts the proposer read from `/proc`. A name here would
/// have to be resolved somewhere, and the place it would be resolved is the executor — which is the
/// one party in this chain with no business deciding who anybody is.
///
/// pid 1 is refused outright. It is init: signalling it is not an act with a risk level, it is the
/// end of the session, the desktop, and the executor doing the signalling.
fn concrete_process(target: &str) -> Result<(u32, u32), ActionError> {
    let invalid = || ActionError::InvalidTarget(target.to_owned());
    let rest = target.strip_prefix("process:").ok_or_else(invalid)?;
    let (uid, pid) = rest.split_once(':').ok_or_else(invalid)?;
    let uid: u32 = uid.parse().map_err(|_| invalid())?;
    let pid: u32 = pid.parse().map_err(|_| invalid())?;
    if pid <= 1 {
        return Err(invalid());
    }
    Ok((uid, pid))
}

/// One concrete package name from a proposal's target, or a refusal.
///
/// The target is `apt:<package>`, and the name inside it must be one Debian would recognise. A
/// placeholder such as `apt:<package>` is refused here rather than reaching the executor, which is
/// the same treatment `concrete_process` gives a placeholder process: a proposal that names nothing
/// concrete is not a proposal to act on something.
fn concrete_package(target: &str) -> Result<String, ActionError> {
    let invalid = || ActionError::InvalidTarget(target.to_owned());
    let package = target.strip_prefix("apt:").ok_or_else(invalid)?;
    if package.len() < 2 || package.len() > 128 {
        return Err(invalid());
    }
    let mut characters = package.chars();
    let first = characters.next().ok_or_else(invalid)?;
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return Err(invalid());
    }
    if !characters.all(|character| {
        character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || matches!(character, '+' | '-' | '.')
    }) {
        return Err(invalid());
    }
    Ok(package.to_owned())
}

fn permit_for(
    proposal: &ActionProposal,
    decision: &AuthorizationDecision,
    action: ExecutableAction,
    now: OffsetDateTime,
) -> ExecutionPermit {
    ExecutionPermit {
        permit_id: Uuid::new_v4(),
        decision_id: decision.decision_id,
        proposal_id: proposal.proposal_id,
        action,
        issued_at: now,
        expires_at: now + PERMIT_LIFETIME,
    }
}

fn executable_action(
    proposal: &ActionProposal,
    operation: Operation,
) -> Result<ExecutableAction, ActionError> {
    Ok(match operation {
        Operation::InspectServiceStatus => ExecutableAction::ServiceStatus {
            unit: concrete_service(&proposal.target_resource)?,
        },
        Operation::CleanPackageCache => {
            if proposal.target_resource != "apt:archives" {
                return Err(ActionError::InvalidTarget(proposal.target_resource.clone()));
            }
            ExecutableAction::PackageCacheClean
        }
        Operation::InstallPackage => ExecutableAction::PackageInstall {
            package: concrete_package(&proposal.target_resource)?,
        },
        Operation::UpgradePackage => ExecutableAction::PackageUpgrade {
            package: concrete_package(&proposal.target_resource)?,
        },
        Operation::RestartService => ExecutableAction::ServiceRestart {
            unit: concrete_service(&proposal.target_resource)?,
        },
        Operation::StartService => ExecutableAction::ServiceStart {
            unit: concrete_service(&proposal.target_resource)?,
        },
        Operation::StopService => ExecutableAction::ServiceStop {
            unit: concrete_service(&proposal.target_resource)?,
        },
        Operation::ReloadService => ExecutableAction::ServiceReload {
            unit: concrete_service(&proposal.target_resource)?,
        },
        Operation::EnableService => ExecutableAction::ServiceEnable {
            unit: concrete_service(&proposal.target_resource)?,
        },
        Operation::DisableService => ExecutableAction::ServiceDisable {
            unit: concrete_service(&proposal.target_resource)?,
        },
        Operation::TerminateProcess => {
            let (owner_uid, pid) = concrete_process(&proposal.target_resource)?;
            ExecutableAction::ProcessTerminate { pid, owner_uid }
        }
        Operation::KillProcess => {
            let (owner_uid, pid) = concrete_process(&proposal.target_resource)?;
            ExecutableAction::ProcessKill { pid, owner_uid }
        }
        Operation::PauseProcess => {
            let (owner_uid, pid) = concrete_process(&proposal.target_resource)?;
            ExecutableAction::ProcessPause { pid, owner_uid }
        }
        Operation::ResumeProcess => {
            let (owner_uid, pid) = concrete_process(&proposal.target_resource)?;
            ExecutableAction::ProcessResume { pid, owner_uid }
        }
        _ => return Err(ActionError::UnknownOperation(proposal.operation.clone())),
    })
}

#[cfg(test)]
mod tests {
    use cybou_protocol::{
        action::AttemptReport,
        telemetry::{EvidenceStrength, Finding, MetricKey, Subject, SystemInsight},
    };

    use super::*;

    fn insight() -> SystemInsight {
        SystemInsight {
            insight_id: Uuid::new_v4(),
            finding: Finding::ServiceInactive,
            about: Some(MetricKey::named(
                Subject::ServiceActive,
                "cybou-action-test.service".to_owned(),
            )),
            because: Vec::new(),
            strength: EvidenceStrength::Strong,
            concluded_at: OffsetDateTime::UNIX_EPOCH,
            since: OffsetDateTime::UNIX_EPOCH,
        }
    }

    #[test]
    fn granted_decision_mints_one_short_lived_typed_permit() {
        let core = ActionCore::new(StandingPolicy {
            pre_authorized: vec![Operation::RestartService],
            pre_authorized_for_agents: Vec::new(),
        });
        let now = OffsetDateTime::UNIX_EPOCH;
        let record = core
            .evaluate_insight(&insight(), "service.restart", now)
            .expect("evaluate");
        let id = record.permit_id.expect("permit");
        let claim = core.claim_permit(id, now).expect("first claim");
        assert_eq!(
            claim.permit.action,
            ExecutableAction::ServiceRestart {
                unit: "cybou-action-test.service".to_owned()
            }
        );
        assert_eq!(
            core.claim_permit(id, now),
            Err(ActionError::PermitUnavailable)
        );
    }

    #[test]
    fn placeholder_unit_never_becomes_a_permit() {
        let core = ActionCore::new(StandingPolicy {
            pre_authorized: vec![Operation::RestartService],
            pre_authorized_for_agents: Vec::new(),
        });
        let mut insight = insight();
        insight.about = None;
        let record = core
            .evaluate_insight(&insight, "service.restart", OffsetDateTime::UNIX_EPOCH)
            .expect("a refusal is still a lifecycle record");
        assert!(matches!(
            record.decision.verdict,
            AuthorizationVerdict::Denied { .. }
        ));
        assert!(record.permit_id.is_none());
    }

    const SEAT: &str = "linux-account:alice";
    const UNIT: &str = "systemd:demo-api.service";

    #[test]
    fn a_person_asking_is_the_confirmation_that_asking_would_otherwise_wait_for() {
        // The whole of ADR-0048. A person who read the unit name and pressed the button has already
        // answered the question a confirmation asks, and asking them again teaches them to click
        // through it.
        let core = ActionCore::new(StandingPolicy::nothing_pre_authorized());
        let record = core
            .request("service.restart", UNIT, SEAT, OffsetDateTime::UNIX_EPOCH)
            .expect("a request this build can express");

        assert_eq!(
            record.decision.verdict,
            AuthorizationVerdict::GrantedOnConfirmation {
                confirmed_by: SEAT.to_owned()
            }
        );
        let permit = record.permit_id.expect("a permit");
        assert!(
            core.claim_permit(permit, OffsetDateTime::UNIX_EPOCH)
                .is_ok()
        );
    }

    #[test]
    fn it_is_never_recorded_as_something_a_policy_granted() {
        // No policy granted this. A record reading `granted` would attribute a person's decision to
        // a standing policy, on a host whose policy authorized nothing.
        let core = ActionCore::new(StandingPolicy::nothing_pre_authorized());
        let record = core
            .request("service.restart", UNIT, SEAT, OffsetDateTime::UNIX_EPOCH)
            .expect("a request");

        assert_ne!(record.decision.verdict, AuthorizationVerdict::Granted);
        assert!(record.decision.verdict.permits_execution());
        assert!(!cybou_remediation::permits_unattended(&record.decision));
        assert!(record.proposal.proposed_by.is_a_person());
    }

    #[test]
    fn a_person_brings_no_evidence_and_the_record_says_so() {
        // A proposal from Mind carries a finding and the readings behind it. A person's request
        // carries a name they typed, and a record that could not tell the two apart would let a
        // reader take somebody's preference for something this host observed.
        let core = ActionCore::new(StandingPolicy::nothing_pre_authorized());
        let record = core
            .request("service.restart", UNIT, SEAT, OffsetDateTime::UNIX_EPOCH)
            .expect("a request");

        assert!(!record.proposal.proposed_by.brings_its_own_evidence());
        assert_eq!(record.proposal.cause_id, None);
    }

    #[test]
    fn what_is_never_offered_is_not_offered_to_a_person_either() {
        // There is no answer somebody could give that makes formatting a filesystem safe, which is
        // the same reason it is not offered to Mind.
        let core = ActionCore::new(StandingPolicy::nothing_pre_authorized());

        for verb in [
            "filesystem.format",
            "system.poweroff",
            "service.data.delete",
        ] {
            let record = core
                .request(verb, UNIT, SEAT, OffsetDateTime::UNIX_EPOCH)
                .expect("a refusal is still a lifecycle record");
            assert!(
                matches!(record.decision.verdict, AuthorizationVerdict::Denied { .. }),
                "{verb} was not refused"
            );
            assert!(record.permit_id.is_none(), "{verb} produced a permit");
        }
    }

    #[test]
    fn a_verb_outside_the_table_is_not_an_operation_with_unknown_risk() {
        // It is not an operation. Accepting one would be accepting a string as an instruction,
        // which is the shape this whole boundary exists to refuse.
        let core = ActionCore::new(StandingPolicy::nothing_pre_authorized());

        assert!(matches!(
            core.request("service.obliterate", UNIT, SEAT, OffsetDateTime::UNIX_EPOCH),
            Err(ActionError::UnknownOperation(_))
        ));
    }

    #[test]
    fn a_request_from_nobody_is_not_a_request() {
        // The seat is established by whatever authenticated the person. An empty one means the
        // caller had none to give, and a permit minted for it would name nobody.
        let core = ActionCore::new(StandingPolicy::nothing_pre_authorized());

        assert!(matches!(
            core.request("service.restart", UNIT, "  ", OffsetDateTime::UNIX_EPOCH),
            Err(ActionError::InvalidTarget(_))
        ));
    }

    #[test]
    fn a_package_name_that_is_not_a_package_name_never_reaches_the_executor() {
        let core = ActionCore::new(StandingPolicy::nothing_pre_authorized());
        // Each of these is a way of saying something other than "this package": a placeholder, an
        // option, a version pin, a path. None may become a permit.
        for target in [
            "apt:<package>",
            "apt:--reinstall",
            "apt:ripgrep=14.0",
            "apt:../etc/passwd",
            "ripgrep",
        ] {
            let record = core
                .request("package.install", target, SEAT, OffsetDateTime::UNIX_EPOCH)
                .expect("a refusal is still a lifecycle record");
            assert!(
                record.permit_id.is_none(),
                "{target} produced a permit to install something"
            );
        }
    }

    #[test]
    fn installing_is_authorized_by_the_person_who_asked_and_by_nobody_else() {
        let core = ActionCore::new(StandingPolicy::nothing_pre_authorized());
        let record = core
            .request(
                "package.install",
                "apt:ripgrep",
                SEAT,
                OffsetDateTime::UNIX_EPOCH,
            )
            .expect("a lifecycle record");
        // A person at an authenticated seat asking for it is the confirmation, and the record says
        // whose seat it was. What is not permitted is anything asking without one.
        assert!(matches!(
            record.decision.verdict,
            AuthorizationVerdict::GrantedOnConfirmation { ref confirmed_by } if confirmed_by == SEAT
        ));
        assert!(record.permit_id.is_some());
        assert_eq!(
            record.proposal.risk_level,
            cybou_protocol::action::RiskLevel::High
        );
        assert!(!record.proposal.reversible);

        assert!(matches!(
            core.request(
                "package.install",
                "apt:ripgrep",
                "   ",
                OffsetDateTime::UNIX_EPOCH
            ),
            Err(ActionError::InvalidTarget(_))
        ));
    }

    #[test]
    fn a_target_the_executor_cannot_act_on_is_refused_rather_than_attempted() {
        // The adapter check, which is what stops a permit existing for a unit nothing can restart.
        let core = ActionCore::new(StandingPolicy::nothing_pre_authorized());
        let record = core
            .request(
                "service.restart",
                "systemd:<unit>",
                SEAT,
                OffsetDateTime::UNIX_EPOCH,
            )
            .expect("a refusal is still a lifecycle record");

        assert!(matches!(
            record.decision.verdict,
            AuthorizationVerdict::Denied { .. }
        ));
        assert!(record.permit_id.is_none());
    }

    #[test]
    fn the_risk_is_the_operations_own_and_not_the_askers() {
        // `ActionProposal` carries risk and reversibility as ordinary fields, so anything building
        // one by hand can fill them in freely. A person's request takes them from the table.
        let core = ActionCore::new(StandingPolicy::nothing_pre_authorized());
        let record = core
            .request("service.restart", UNIT, SEAT, OffsetDateTime::UNIX_EPOCH)
            .expect("a request");

        assert_eq!(
            record.proposal.risk_level,
            cybou_remediation::Operation::RestartService.risk()
        );
        assert_eq!(
            record.proposal.reversible,
            cybou_remediation::Operation::RestartService.reversible()
        );
    }

    #[test]
    fn a_request_is_answered_once_and_its_permit_spent_once() {
        let core = ActionCore::new(StandingPolicy::nothing_pre_authorized());
        let record = core
            .request("service.restart", UNIT, SEAT, OffsetDateTime::UNIX_EPOCH)
            .expect("a request");
        let permit = record.permit_id.expect("a permit");

        assert!(
            core.claim_permit(permit, OffsetDateTime::UNIX_EPOCH)
                .is_ok()
        );
        assert_eq!(
            core.claim_permit(permit, OffsetDateTime::UNIX_EPOCH),
            Err(ActionError::PermitUnavailable)
        );

        // And it cannot be confirmed afterwards: it was never waiting on an answer.
        assert_eq!(
            core.confirm(
                record.proposal.proposal_id,
                record.decision.decision_id,
                SEAT,
                OffsetDateTime::UNIX_EPOCH,
            ),
            Err(ActionError::NotAwaitingConfirmation)
        );
    }

    /// One host that has been asked and has agreed to nothing yet.
    ///
    /// The default standing policy, which is the only one a fresh installation has, so every
    /// proposal here stops at the verdict a person is meant to answer.
    fn awaiting_confirmation() -> (ActionCore, ActionRecord, OffsetDateTime) {
        let now = OffsetDateTime::UNIX_EPOCH;
        let core = ActionCore::new(StandingPolicy::nothing_pre_authorized());
        let record = core
            .evaluate_insight(&insight(), "service.restart", now)
            .expect("evaluate");
        assert!(matches!(
            record.decision.verdict,
            AuthorizationVerdict::RequiresUserConfirmation { .. }
        ));
        assert!(record.permit_id.is_none());
        (core, record, now)
    }

    #[test]
    fn a_person_saying_yes_is_what_turns_a_question_into_a_permit() {
        let (core, record, now) = awaiting_confirmation();

        let confirmed = core
            .confirm(
                record.proposal.proposal_id,
                record.decision.decision_id,
                "linux-account:alice",
                now,
            )
            .expect("a confirmation");

        // The permit exists and is claimable exactly once, like any other.
        let permit_id = confirmed.permit_id.expect("a permit");
        assert!(core.claim_permit(permit_id, now).is_ok());
        assert_eq!(
            core.claim_permit(permit_id, now),
            Err(ActionError::PermitUnavailable)
        );
    }

    #[test]
    fn a_confirmed_grant_is_never_mistaken_for_a_standing_one() {
        // The two authorize the same execution and are not the same authorization. A record that
        // could not tell them apart would answer "who authorized this" with the policy, on a host
        // whose policy authorized nothing.
        let (core, record, now) = awaiting_confirmation();

        let confirmed = core
            .confirm(
                record.proposal.proposal_id,
                record.decision.decision_id,
                "linux-account:alice",
                now,
            )
            .expect("a confirmation");

        assert_eq!(
            confirmed.decision.verdict,
            AuthorizationVerdict::GrantedOnConfirmation {
                confirmed_by: "linux-account:alice".to_owned(),
            }
        );
        assert!(confirmed.decision.verdict.permits_execution());
        // It may execute, and it was not unattended. Code that conflates the two either refuses
        // what a person allowed or performs unasked what they were meant to be asked about.
        assert!(!cybou_remediation::permits_unattended(&confirmed.decision));
        // A different authorization is a different decision, not the same one read twice.
        assert_ne!(confirmed.decision.decision_id, record.decision.decision_id);
    }

    #[test]
    fn the_same_question_cannot_be_answered_twice() {
        let (core, record, now) = awaiting_confirmation();

        let confirmed = core
            .confirm(
                record.proposal.proposal_id,
                record.decision.decision_id,
                "linux-account:alice",
                now,
            )
            .expect("a confirmation");

        // Neither with the prompt's decision, which is no longer the one here...
        assert_eq!(
            core.confirm(
                record.proposal.proposal_id,
                record.decision.decision_id,
                "linux-account:alice",
                now,
            ),
            Err(ActionError::NotAwaitingConfirmation)
        );
        // ...nor with the decision the confirmation itself produced. A second yes must not mint a
        // second permit for one agreement.
        assert_eq!(
            core.confirm(
                record.proposal.proposal_id,
                confirmed.decision.decision_id,
                "linux-account:alice",
                now,
            ),
            Err(ActionError::NotAwaitingConfirmation)
        );
    }

    #[test]
    fn agreeing_to_a_prompt_nobody_is_showing_authorizes_nothing() {
        // A proposal re-decided between being drawn and being clicked is a different prompt, and
        // without this check the answer to one question authorizes another.
        let (core, record, now) = awaiting_confirmation();

        assert_eq!(
            core.confirm(
                record.proposal.proposal_id,
                Uuid::new_v4(),
                "linux-account:alice",
                now,
            ),
            Err(ActionError::NotAwaitingConfirmation)
        );
    }

    #[test]
    fn a_proposal_older_than_its_readings_can_no_longer_be_agreed_to() {
        // A proposal carries a diagnosis drawn from readings taken at one instant. Long enough
        // afterwards, confirming is agreeing to a claim nobody re-checked.
        let (core, record, now) = awaiting_confirmation();

        assert_eq!(
            core.confirm(
                record.proposal.proposal_id,
                record.decision.decision_id,
                "linux-account:alice",
                now + CONFIRMATION_WINDOW + Duration::seconds(1),
            ),
            Err(ActionError::NotAwaitingConfirmation)
        );
        // And still answerable inside the window, so the bound is a bound rather than a refusal.
        assert!(
            core.confirm(
                record.proposal.proposal_id,
                record.decision.decision_id,
                "linux-account:alice",
                now + CONFIRMATION_WINDOW,
            )
            .is_ok()
        );
    }

    #[test]
    fn a_refusal_is_not_a_question_and_cannot_be_answered() {
        // A person's agreement answers a question. It does not overrule a denial: this proposal
        // names no concrete target, and no amount of saying yes makes one.
        let now = OffsetDateTime::UNIX_EPOCH;
        let core = ActionCore::new(StandingPolicy::nothing_pre_authorized());
        let mut insight = insight();
        insight.about = None;
        let record = core
            .evaluate_insight(&insight, "service.restart", now)
            .expect("a refusal is still a lifecycle record");
        assert!(matches!(
            record.decision.verdict,
            AuthorizationVerdict::Denied { .. }
        ));

        assert_eq!(
            core.confirm(
                record.proposal.proposal_id,
                record.decision.decision_id,
                "linux-account:alice",
                now,
            ),
            Err(ActionError::NotAwaitingConfirmation)
        );
    }

    #[test]
    fn a_confirmation_from_nobody_is_not_a_confirmation() {
        // The seat is established by whatever authenticated it, never supplied by the party being
        // authorized. An empty one means the caller had none to give.
        let (core, record, now) = awaiting_confirmation();

        assert_eq!(
            core.confirm(
                record.proposal.proposal_id,
                record.decision.decision_id,
                "   ",
                now,
            ),
            Err(ActionError::NotAwaitingConfirmation)
        );
    }

    #[test]
    fn nothing_here_proposed_it_is_said_as_itself() {
        // The one refusal that is distinguishable, because it is about identity rather than about
        // how close a caller came to something confirmable.
        let (core, _record, now) = awaiting_confirmation();
        let invented = Uuid::new_v4();

        assert_eq!(
            core.confirm(invented, Uuid::new_v4(), "linux-account:alice", now),
            Err(ActionError::UnknownProposal(invented))
        );
    }

    #[test]
    fn default_policy_stops_before_the_executor() {
        let core = ActionCore::new(StandingPolicy::nothing_pre_authorized());
        let record = core
            .evaluate_insight(&insight(), "service.restart", OffsetDateTime::UNIX_EPOCH)
            .expect("evaluate");
        assert!(matches!(
            record.decision.verdict,
            AuthorizationVerdict::RequiresUserConfirmation { .. }
        ));
        assert!(record.permit_id.is_none());
    }

    #[test]
    fn expired_permit_is_consumed_and_refused() {
        let core = ActionCore::new(StandingPolicy {
            pre_authorized: vec![Operation::InspectServiceStatus],
            pre_authorized_for_agents: Vec::new(),
        });
        let now = OffsetDateTime::UNIX_EPOCH;
        let record = core
            .evaluate_insight(&insight(), "service.status", now)
            .expect("evaluate");
        assert_eq!(
            core.claim_permit(
                record.permit_id.expect("permit"),
                now + Duration::seconds(61)
            ),
            Err(ActionError::PermitUnavailable)
        );
    }

    #[test]
    fn an_operation_the_executor_cannot_perform_is_denied_however_it_was_granted() {
        // The operation table and the executor's adapters are two lists, and this is what stops
        // them drifting apart into a permit for something nothing can carry out. It used to reach
        // for `service.reload` as its example, which stopped being one on 2026-08-31 when reload
        // gained an adapter — so it asks about log rotation, which is in the table and has none.
        //
        // Asked through a person's request rather than through a finding, because `evaluate_insight`
        // only ever decides operations the proposer offered for that finding, and a service being
        // inactive does not suggest rotating logs. The invariant is about the two lists and not
        // about which door the operation arrived through.
        let core = ActionCore::new(StandingPolicy::nothing_pre_authorized());
        let record = core
            .request(
                "log.rotate",
                "journald:logs",
                "linux-account:alice",
                OffsetDateTime::UNIX_EPOCH,
            )
            .expect("a refusal is still a lifecycle record");
        assert!(matches!(
            record.decision.verdict,
            AuthorizationVerdict::Denied { .. }
        ));
        assert!(record.permit_id.is_none());

        // And the objection names the missing half rather than the operation, so a reader is sent
        // to the executor rather than told the table is wrong.
        assert!(
            record
                .checks
                .iter()
                .any(|check| check.rule_id == "executor-adapter-exists" && !check.passed)
        );
    }

    #[test]
    fn package_cache_clean_is_the_one_fixed_non_systemd_target() {
        let core = ActionCore::new(StandingPolicy {
            pre_authorized: vec![Operation::CleanPackageCache],
            pre_authorized_for_agents: Vec::new(),
        });
        let mut storage = insight();
        storage.finding = Finding::StorageExhaustion;
        storage.about = None;
        let record = core
            .evaluate_insight(&storage, "package.cache.clean", OffsetDateTime::UNIX_EPOCH)
            .expect("evaluate");
        let claim = core
            .claim_permit(
                record.permit_id.expect("permit"),
                OffsetDateTime::UNIX_EPOCH,
            )
            .expect("claim");
        assert_eq!(claim.permit.action, ExecutableAction::PackageCacheClean);
    }

    #[test]
    fn a_final_report_must_match_the_execution_action1_started() {
        let core = ActionCore::new(StandingPolicy {
            pre_authorized: vec![Operation::RestartService],
            pre_authorized_for_agents: Vec::new(),
        });
        let record = core
            .evaluate_insight(&insight(), "service.restart", OffsetDateTime::UNIX_EPOCH)
            .expect("evaluate");
        let claim = core
            .claim_permit(
                record.permit_id.expect("permit"),
                OffsetDateTime::UNIX_EPOCH,
            )
            .expect("claim");
        let mut substituted = claim.started.finish(
            AttemptReport::Completed,
            Vec::new(),
            Some(OffsetDateTime::UNIX_EPOCH + Duration::seconds(1)),
        );
        substituted.attempt_id = Uuid::new_v4();

        assert_eq!(
            core.record_attempt(substituted.clone()),
            Err(ActionError::AttemptMismatch(substituted.attempt_id))
        );
        assert!(
            core.record(record.proposal.proposal_id)
                .expect("record")
                .attempt
                .is_none(),
            "a substituted report must not overwrite the durable start"
        );
    }

    #[test]
    fn episode_for_cause_returns_the_latest_attempt_not_a_refused_proposal() {
        let core = ActionCore::new(StandingPolicy {
            pre_authorized: vec![Operation::RestartService],
            pre_authorized_for_agents: Vec::new(),
        });
        let finding = insight();
        let refused = core
            .evaluate_insight(&finding, "service.status", OffsetDateTime::UNIX_EPOCH)
            .expect("the unapproved inspection is still a record");
        assert!(refused.attempt.is_none());

        let first = core
            .evaluate_insight(
                &finding,
                "service.restart",
                OffsetDateTime::UNIX_EPOCH + Duration::seconds(1),
            )
            .expect("first permitted remedy");
        let second = core
            .evaluate_insight(
                &finding,
                "service.restart",
                OffsetDateTime::UNIX_EPOCH + Duration::seconds(2),
            )
            .expect("later permitted remedy");
        for (record, offset) in [(&first, 1), (&second, 2)] {
            let claim = core
                .claim_permit(
                    record.permit_id.expect("permit"),
                    OffsetDateTime::UNIX_EPOCH + Duration::seconds(offset),
                )
                .expect("claim");
            core.record_attempt(claim.started.finish(
                AttemptReport::Completed,
                Vec::new(),
                Some(OffsetDateTime::UNIX_EPOCH + Duration::seconds(offset + 1)),
            ))
            .expect("attempt belongs to its proposal");
        }

        assert_eq!(
            core.episode_for_cause(finding.insight_id)
                .expect("an attempted episode")
                .proposal
                .proposal_id,
            second.proposal.proposal_id
        );
        assert!(core.episode_for_cause(Uuid::new_v4()).is_none());
    }
}
