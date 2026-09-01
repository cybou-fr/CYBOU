// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! What may be proposed at all, and what each one costs to be wrong about.
//!
//! The whole point of this module is one thing being impossible: **a proposer cannot choose its own
//! risk.** `ActionProposal` carries a `risk_level` and a `reversible` flag as ordinary fields, which
//! means whoever builds a proposal fills them in — and something arguing for its own proposal is
//! exactly the wrong party to assess it. A model asked to restart a database and rate the danger
//! will rate it low, not dishonestly, but because it is arguing.
//!
//! So an operation is a closed set, and risk and reversibility are *functions of the operation*.
//! A proposer names the operation; the risk follows, and there is no field to override it.
//!
//! ## Reversible does not mean harmless, and irreversible does not mean forbidden
//!
//! Two distinctions that get collapsed and should not be. Restarting a service is reversible in the
//! sense that the service comes back, and it still drops every connection it was holding. Deleting a
//! package cache is irreversible in the sense that the bytes are gone, and it is one of the safest
//! things on this list because the bytes are re-downloadable. Reversibility is about whether the
//! system can undo it; risk is about what it costs if it was the wrong call. They are recorded
//! separately because they answer different questions.

use cybou_protocol::action::RiskLevel;
use cybou_protocol::telemetry::Finding;
use serde::{Deserialize, Serialize};

/// Something Cybou may propose doing to its own Body.
///
/// Closed, and short. Every entry is something with a distinct effect and a distinct cost; an open
/// string operation would be a way to add both without anyone deciding either, which is the same
/// gap that let a model-generated shell command become the architecture everywhere else.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Operation {
    /// Ask a unit what state it is in.
    InspectServiceStatus,
    /// Ask a unit to reload its configuration.
    ReloadService,
    /// Restart a unit.
    RestartService,
    /// Start a unit that is not running.
    StartService,
    /// Ask a process to exit, and let it decide how.
    TerminateProcess,
    /// End a process without asking.
    KillProcess,
    /// Suspend a process where it stands.
    PauseProcess,
    /// Let a suspended process continue.
    ResumeProcess,
    /// Stop a unit that is.
    StopService,
    /// Arrange for a unit to start at the next boot.
    EnableService,
    /// Stop a unit from starting at the next boot.
    DisableService,
    /// Delete downloaded package archives that can be fetched again.
    CleanPackageCache,
    /// Rotate and compress logs that are past their retention.
    RotateLogs,
    /// Delete temporary files nothing has opened.
    TrimTemporaryFiles,
    /// Delete a service's data directory.
    DeleteServiceData,
    /// Reformat a filesystem.
    FormatFilesystem,
    /// Power the machine off.
    PowerOff,
}

/// Every operation, so a test can hold a property across all of them.
pub const ALL_OPERATIONS: &[Operation] = &[
    Operation::InspectServiceStatus,
    Operation::ReloadService,
    Operation::RestartService,
    Operation::StartService,
    Operation::StopService,
    Operation::EnableService,
    Operation::DisableService,
    Operation::TerminateProcess,
    Operation::KillProcess,
    Operation::PauseProcess,
    Operation::ResumeProcess,
    Operation::CleanPackageCache,
    Operation::RotateLogs,
    Operation::TrimTemporaryFiles,
    Operation::DeleteServiceData,
    Operation::FormatFilesystem,
    Operation::PowerOff,
];

impl Operation {
    /// The frozen verb this operation is recorded under.
    #[must_use]
    pub const fn verb(self) -> &'static str {
        match self {
            Self::InspectServiceStatus => "service.status",
            Self::ReloadService => "service.reload",
            Self::RestartService => "service.restart",
            Self::StartService => "service.start",
            Self::EnableService => "service.enable",
            Self::DisableService => "service.disable",
            Self::TerminateProcess => "process.terminate",
            Self::KillProcess => "process.kill",
            Self::PauseProcess => "process.pause",
            Self::ResumeProcess => "process.resume",
            Self::StopService => "service.stop",
            Self::CleanPackageCache => "package.cache.clean",
            Self::RotateLogs => "log.rotate",
            Self::TrimTemporaryFiles => "tmp.trim",
            Self::DeleteServiceData => "service.data.delete",
            Self::FormatFilesystem => "filesystem.format",
            Self::PowerOff => "system.poweroff",
        }
    }

    /// What being wrong about this costs.
    ///
    /// A property of the operation, not of the argument for it. There is no path by which a
    /// proposer supplies this.
    #[allow(clippy::match_same_arms)]
    #[must_use]
    pub const fn risk(self) -> RiskLevel {
        match self {
            Self::InspectServiceStatus | Self::ReloadService => RiskLevel::Low,
            // Starting something that was not running changes what this host is doing, and the
            // unit decides what that means. Lower than stopping one, because what a start
            // interrupts is nothing.
            Self::StartService => RiskLevel::Low,
            // Continuing a process that was suspended puts it back where it was.
            Self::ResumeProcess => RiskLevel::Low,
            // SIGTERM is a request. The process gets to write what it was holding and close what
            // it had open, which is the whole difference between this and the next one.
            Self::TerminateProcess => RiskLevel::Medium,
            // Suspending is reversible, and a suspended process still holds every lock and socket
            // it had. Pausing the wrong one is an outage that looks like a hang.
            Self::PauseProcess => RiskLevel::Medium,
            // SIGKILL cannot be caught, blocked or ignored. Whatever was in memory is gone, and no
            // amount of care afterwards brings it back.
            Self::KillProcess => RiskLevel::High,
            // Neither of these changes anything a person can see today, which is exactly why they
            // are not Low. What they change is what the machine does when nobody is watching it
            // come up: a unit disabled by mistake is an outage at the next reboot, weeks later,
            // with nothing connecting it back to this moment. The delay is the risk.
            Self::EnableService | Self::DisableService => RiskLevel::Medium,
            // Reversible and not harmless: the service comes back, and every connection it was
            // holding is gone.
            // A stop is a restart without the second half: everything the service was holding is
            // gone and nothing takes it up again.
            Self::RestartService
            | Self::StopService
            | Self::CleanPackageCache
            | Self::RotateLogs => RiskLevel::Medium,
            // Higher than the cache because something may be using a temporary file this cannot see.
            Self::TrimTemporaryFiles => RiskLevel::High,
            Self::DeleteServiceData | Self::FormatFilesystem | Self::PowerOff => {
                RiskLevel::Critical
            }
        }
    }

    /// Whether the system can undo it.
    ///
    /// Not whether it is safe. Deleting a package cache cannot be undone and is among the safest
    /// things here, because the bytes are re-downloadable; restarting a service can be undone and
    /// still costs every connection it was holding.
    #[must_use]
    pub const fn reversible(self) -> bool {
        // A start is undone by a stop and a stop by a start, in the sense this word is used
        // here: the system can put the unit back the way it was. What a stop cost while it was
        // down is not undone by anything, which is what `reversible` deliberately does not mean.
        //
        // Pausing and resuming are the same kind of pair. Terminating and killing are not in it:
        // a process that has exited is not something the system can put back, under any reading of
        // the word, and saying otherwise here would let a caller treat the two as interchangeable.
        matches!(
            self,
            Self::InspectServiceStatus
                | Self::ReloadService
                | Self::RestartService
                | Self::StartService
                | Self::StopService
                | Self::PauseProcess
                | Self::ResumeProcess
                | Self::EnableService
                | Self::DisableService
        )
    }

    /// Whether this is refused whatever the evidence says.
    ///
    /// The list ADR-0022 calls *destructive or forbidden regardless of model confidence*. Nothing
    /// in this build can grant them, and the reason they are here at all rather than simply absent
    /// is that a proposer that reaches for one should be visibly refused rather than silently
    /// unable to express it. A refusal is a record; an inexpressible thought is not.
    #[must_use]
    pub const fn forbidden(self) -> bool {
        matches!(self.risk(), RiskLevel::Critical)
    }

    /// What this operation would relieve, if anything.
    ///
    /// An operation that relieves nothing is not a remedy, and a proposal citing a finding this
    /// does not address is what the critic exists to catch.
    #[allow(clippy::match_same_arms)]
    #[must_use]
    pub fn relieves(self) -> &'static [Finding] {
        match self {
            Self::CleanPackageCache | Self::RotateLogs | Self::TrimTemporaryFiles => {
                &[Finding::StorageExhaustion]
            }
            Self::RestartService => &[
                Finding::ServiceFailure,
                Finding::ServiceInactive,
                Finding::MemoryPressure,
            ],
            Self::ReloadService => &[Finding::ServiceFailure, Finding::ServiceInactive],
            // A unit that is not running is the one thing starting it fixes. Stopping one relieves
            // nothing: it is a thing a person wants done, not a remedy for a finding, and offering
            // it as one would let a host reach for it while nobody is present.
            Self::StartService => &[Finding::ServiceInactive],
            Self::StopService => &[],
            // Enabling does not start anything, so it relieves nothing a host could observe now:
            // a machine that answered `ServiceInactive` by enabling the unit would have changed
            // the next boot and left the finding standing. Disabling relieves nothing either, for
            // the reason stopping does not — it is a thing a person decides, not a remedy.
            Self::EnableService | Self::DisableService => &[],
            // None of these relieves anything, and that is the entry, not an omission. A finding
            // listed here is a licence for the host to reach for the operation on its own when it
            // concludes something — and a host that may kill processes to relieve a conclusion it
            // reached about memory pressure is a host that ends somebody's work while they are
            // away from the keyboard. These are things a person does, deliberately, present.
            Self::TerminateProcess
            | Self::KillProcess
            | Self::PauseProcess
            | Self::ResumeProcess => &[],
            // Reading a unit's state relieves nothing and is worth proposing anyway: it is how a
            // person finds out more without changing anything, and a system that could only offer
            // mutations would push every investigation toward one. The forbidden three relieve
            // nothing for the opposite reason — advertising a remedy would give a proposer a reason
            // to reach for one, and the refusal would then read as obstruction.
            Self::InspectServiceStatus
            | Self::DeleteServiceData
            | Self::FormatFilesystem
            | Self::PowerOff => &[],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn risk_is_a_property_of_the_operation_and_not_of_the_argument_for_it() {
        // The whole point of the module. There is no path by which a proposer supplies this, which
        // is what makes it different from the `risk_level` field on the proposal it produces.
        assert_eq!(Operation::RestartService.risk(), RiskLevel::Medium);
        assert_eq!(Operation::DeleteServiceData.risk(), RiskLevel::Critical);
        assert_eq!(Operation::InspectServiceStatus.risk(), RiskLevel::Low);
    }

    #[test]
    fn reversible_does_not_mean_harmless_and_irreversible_does_not_mean_dangerous() {
        // Two distinctions that get collapsed and answer different questions.
        assert!(
            Operation::RestartService.reversible(),
            "the service comes back"
        );
        assert_eq!(
            Operation::RestartService.risk(),
            RiskLevel::Medium,
            "and every connection it was holding is gone"
        );

        assert!(
            !Operation::CleanPackageCache.reversible(),
            "the bytes are gone"
        );
        assert_eq!(
            Operation::CleanPackageCache.risk(),
            RiskLevel::Medium,
            "and they are re-downloadable, so this is among the safest things here"
        );
    }

    #[test]
    fn everything_critical_is_forbidden_and_nothing_else_is() {
        for operation in ALL_OPERATIONS {
            assert_eq!(
                operation.forbidden(),
                operation.risk() == RiskLevel::Critical,
                "{operation:?}"
            );
        }
        assert!(Operation::DeleteServiceData.forbidden());
        assert!(!Operation::RestartService.forbidden());
    }

    #[test]
    fn nothing_forbidden_claims_to_relieve_anything() {
        // A forbidden operation that advertised a remedy would give a proposer a reason to reach
        // for it, and the refusal would then read as the system being obstructive rather than as
        // the operation being off the table.
        for operation in ALL_OPERATIONS {
            if operation.forbidden() {
                assert!(operation.relieves().is_empty(), "{operation:?}");
            }
        }
    }

    #[test]
    fn every_operation_has_a_distinct_frozen_verb() {
        let mut verbs: Vec<&str> = ALL_OPERATIONS.iter().map(|op| op.verb()).collect();
        verbs.sort_unstable();
        let distinct = verbs.len();
        verbs.dedup();
        assert_eq!(verbs.len(), distinct, "two operations share a verb");
    }

    #[test]
    fn something_that_relieves_nothing_is_still_worth_being_able_to_propose() {
        // Reading a unit's state changes nothing, and a system that could only offer mutations
        // would push every investigation toward one.
        assert!(Operation::InspectServiceStatus.relieves().is_empty());
        assert!(!Operation::InspectServiceStatus.forbidden());
        assert_eq!(Operation::InspectServiceStatus.risk(), RiskLevel::Low);
    }
}
