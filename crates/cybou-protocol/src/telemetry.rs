// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! What a host is doing right now, and the line between that and what happened to it (ADR-0041 S7).
//!
//! Perception records what is stable about a machine — its kernel, its hostname, how much memory it
//! has — and stops there on purpose. Cybou therefore cannot notice that anything is wrong, because
//! nothing observes the machine between one restart and the next. Fixing that is the whole of
//! ADR-0041's S0 gate, and it introduces exactly one danger worth designing around.
//!
//! ## Telemetry is not biography
//!
//! A CPU sample every second is not a life event. A Journal accumulating them would be a telemetry
//! database wearing a life story: every rule that makes the Journal worth having — erasure,
//! retention, dependency closure, provenance — would be applied to numbers that individually mean
//! nothing and collectively cost something to keep forever. Worse, the biography would come to be
//! mostly noise, and a question like *what happened to this host* would return a hundred thousand
//! readings and the four things that mattered.
//!
//! So the boundary is drawn in the types, not in a convention:
//!
//! - [`Reading`] is a number at an instant. It has **no** conversion to a contribution, in this
//!   module or anywhere else, and it never will. Raw samples live in a bounded transient window and
//!   are dropped when they fall out of it.
//! - [`SystemInsight`] is something the host concluded. It cites the readings that led to it, is
//!   always a hypothesis rather than a fact, and is the only thing here that crosses into Event1.
//!
//! The asymmetry is the point. Observation is cheap and forgettable; a conclusion drawn from
//! observation is neither.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Something about a host that is worth watching over time.
///
/// A closed set. An open string subject would let any probe invent a series, and a series nobody
/// declared is a series nobody decided how to interpret, bound, or explain — which is how a
/// telemetry system becomes a pile of numbers with a search box.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Subject {
    /// One-minute load average.
    LoadAverage,
    /// Share of memory in use, in [0.0, 1.0].
    MemoryUsed,
    /// Share of swap in use, in [0.0, 1.0].
    SwapUsed,
    /// Memory pressure, some-avg10 from the kernel, in [0.0, 100.0].
    ///
    /// Pressure rather than free memory, because free memory on Linux is not a measure of anything
    /// a person cares about: a host with almost none free may be perfectly well, and one with
    /// plenty free may be stalling. Pressure is the kernel saying how much time was lost waiting.
    MemoryPressure,
    /// I/O pressure, some-avg10 from the kernel, in [0.0, 100.0].
    IoPressure,
    /// CPU pressure, some-avg10 from the kernel, in [0.0, 100.0].
    CpuPressure,
    /// Share of the root filesystem in use, in [0.0, 1.0].
    RootFilesystemUsed,
    /// How many systemd units are in a failed state.
    FailedUnits,
}

impl Subject {
    /// The frozen spelling this subject is recorded and rendered under.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::LoadAverage => "load.average",
            Self::MemoryUsed => "memory.used",
            Self::SwapUsed => "swap.used",
            Self::MemoryPressure => "memory.pressure",
            Self::IoPressure => "io.pressure",
            Self::CpuPressure => "cpu.pressure",
            Self::RootFilesystemUsed => "filesystem.root.used",
            Self::FailedUnits => "systemd.units.failed",
        }
    }

    /// The value at which this subject is a problem regardless of what is ordinary here.
    ///
    /// `None` for the subjects where there is no such number: a load average of 4 is a crisis on one
    /// machine and a Tuesday on another, and only the baseline can say which.
    ///
    /// One number, read by both the detector and the projection. They used to hold their own copies,
    /// which is how a system comes to report that a disk is fine and that it reaches trouble in three
    /// days — two true statements about two different thresholds.
    #[must_use]
    pub const fn alarming(self) -> Option<f64> {
        match self {
            Self::RootFilesystemUsed | Self::MemoryUsed => Some(0.95),
            Self::MemoryPressure | Self::IoPressure | Self::CpuPressure => Some(40.0),
            Self::SwapUsed => Some(0.90),
            Self::FailedUnits => Some(1.0),
            Self::LoadAverage => None,
        }
    }

    /// Whether a rising value is the direction worth worrying about.
    ///
    /// Every subject here is one where more is worse, and that is a fact about this list rather
    /// than about telemetry: a subject where a *fall* is the problem — free disk, requests served,
    /// backup recency — would need its own answer, and adding one without answering this would give
    /// it a detector that watches the wrong tail.
    #[must_use]
    pub const fn rising_is_worse(self) -> bool {
        true
    }
}

/// Every subject, so a test can hold a property across all of them.
pub const ALL_SUBJECTS: &[Subject] = &[
    Subject::LoadAverage,
    Subject::MemoryUsed,
    Subject::SwapUsed,
    Subject::MemoryPressure,
    Subject::IoPressure,
    Subject::CpuPressure,
    Subject::RootFilesystemUsed,
    Subject::FailedUnits,
];

/// One number, observed at one instant.
///
/// Deliberately has no path into the Journal. There is no `into_contribution`, no `Kind`, and no
/// conversion anywhere in this tree — a reading is transient by construction and not by policy.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reading {
    /// What was observed.
    pub subject: Subject,
    /// What it was.
    pub value: f64,
    /// When.
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
}

/// How far a reading sits from what is ordinary for this host.
///
/// In units of median absolute deviation rather than standard deviations, because a host that has
/// already been misbehaving for ten minutes has a standard deviation shaped by the misbehaviour. A
/// median-based spread is not moved by the thing it is supposed to detect.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Deviation {
    /// What is ordinary for this host, as a median of the window.
    pub ordinary: f64,
    /// How much this host ordinarily varies, as a median absolute deviation.
    pub spread: f64,
    /// The observed value.
    pub observed: f64,
    /// How many spreads away the observation is, signed.
    pub spreads_away: f64,
}

/// How sure the host is that its explanation is the right one.
///
/// Coarse and named rather than a number. A diagnosis reported as `0.81` invites a reader to
/// compare it with another `0.79` as though the difference meant something; these three are
/// distinguishable by what is actually behind them.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceStrength {
    /// One subject is out of its ordinary range and nothing corroborates it.
    Weak,
    /// Several subjects moved together in a way this explanation predicts.
    Moderate,
    /// Several subjects moved together and something categorical agrees — a unit actually failed, a
    /// filesystem is actually full.
    Strong,
}

/// Something the host concluded about itself.
///
/// The only thing in this module that may become a contribution, and it becomes a `Hypothesis`
/// rather than an `Observation`. What was observed is the readings; that they add up to *the
/// database stopped because the disk filled* is an inference, and an inference recorded as an
/// observation is a claim the host cannot support.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInsight {
    /// Unique identity of this conclusion.
    pub insight_id: Uuid,
    /// What the host thinks is happening, in a closed vocabulary.
    pub finding: Finding,
    /// The subjects that led to it, and how far each was from ordinary.
    ///
    /// Carried rather than summarised: *why do you think that* is answered by looking, in the same
    /// way an activation path answers *why did you think of honey*. An insight that could not show
    /// its readings would be indistinguishable from one a model made up.
    pub because: Vec<(Subject, Deviation)>,
    /// How well the evidence supports it.
    pub strength: EvidenceStrength,
    /// When the host concluded it.
    #[serde(with = "time::serde::rfc3339")]
    pub concluded_at: OffsetDateTime,
    /// When the behaviour started, as far as the window can tell.
    #[serde(with = "time::serde::rfc3339")]
    pub since: OffsetDateTime,
}

/// What a host can conclude about itself.
///
/// A closed set, and short on purpose. Every entry is something with a distinct remedy; a finding
/// nobody can act on differently from another finding is not a finding, it is a synonym.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Finding {
    /// Memory is under sustained pressure, and swapping with it.
    MemoryPressure,
    /// Storage is filling, or full.
    StorageExhaustion,
    /// The machine is spending its time waiting for disk.
    IoSaturation,
    /// The machine is spending its time waiting for CPU.
    CpuSaturation,
    /// One or more services are in a failed state.
    ServiceFailure,
    /// Something is out of its ordinary range and nothing here explains it.
    ///
    /// Kept as a finding rather than dropped. A detector that only reported what it had a name for
    /// would be silent exactly when a host is doing something nobody anticipated, which is the case
    /// an operator most wants to hear about.
    UnexplainedDeviation,
}

impl Finding {
    /// The frozen spelling this finding is recorded and rendered under.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::MemoryPressure => "memory-pressure",
            Self::StorageExhaustion => "storage-exhaustion",
            Self::IoSaturation => "io-saturation",
            Self::CpuSaturation => "cpu-saturation",
            Self::ServiceFailure => "service-failure",
            Self::UnexplainedDeviation => "unexplained-deviation",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_subject_and_finding_has_a_frozen_spelling() {
        let mut names: Vec<&str> = ALL_SUBJECTS.iter().map(|s| s.name()).collect();
        names.sort_unstable();
        let distinct = names.len();
        names.dedup();
        assert_eq!(names.len(), distinct, "two subjects share a name");
        for name in &names {
            assert!(name.contains('.'), "{name} is not a dotted subject name");
        }
    }

    #[test]
    fn a_reading_has_no_way_into_the_journal() {
        // The rule this module exists to hold, and the only honest way to test it is to state what
        // must remain absent. `Reading` has no `Kind`, no `into_contribution`, and nothing in this
        // tree converts one — a reading is transient by construction rather than by policy, and if
        // that ever changes it should change here, visibly, and not at a call site.
        //
        // What can be checked mechanically is the other half: an insight is a conclusion and says
        // so, and carries what it concluded from.
        let insight = SystemInsight {
            insight_id: Uuid::from_u128(1),
            finding: Finding::StorageExhaustion,
            because: vec![(
                Subject::RootFilesystemUsed,
                Deviation {
                    ordinary: 0.62,
                    spread: 0.01,
                    observed: 0.94,
                    spreads_away: 32.0,
                },
            )],
            strength: EvidenceStrength::Strong,
            concluded_at: OffsetDateTime::from_unix_timestamp(1_787_000_000).expect("an instant"),
            since: OffsetDateTime::from_unix_timestamp(1_786_999_000).expect("an instant"),
        };
        assert!(
            !insight.because.is_empty(),
            "an insight that cannot show its readings is indistinguishable from one made up"
        );
        assert!(insight.since < insight.concluded_at);
    }

    #[test]
    fn an_insight_survives_the_wire() {
        let insight = SystemInsight {
            insight_id: Uuid::from_u128(2),
            finding: Finding::UnexplainedDeviation,
            because: Vec::new(),
            strength: EvidenceStrength::Weak,
            concluded_at: OffsetDateTime::from_unix_timestamp(1_787_000_000).expect("an instant"),
            since: OffsetDateTime::from_unix_timestamp(1_787_000_000).expect("an instant"),
        };
        let mut encoded = Vec::new();
        ciborium::into_writer(&insight, &mut encoded).expect("encodes");
        let decoded: SystemInsight = ciborium::from_reader(encoded.as_slice()).expect("decodes");
        assert_eq!(decoded, insight);
    }
}
