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

/// A threshold, and which side of it is the problem.
///
/// Both directions exist because both kinds of subject do. A filesystem share is a problem when it
/// is high; days remaining on a certificate, free memory, and time since the last successful backup
/// are problems when they are low. A single number with an implied direction would make the second
/// kind unrepresentable, and the way that failure shows up is a detector reporting a certificate as
/// healthy right up to the hour it expires.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Alarming {
    /// A problem at or above this value.
    AtOrAbove(f64),
    /// A problem at or below this value.
    AtOrBelow(f64),
}

impl Alarming {
    /// The number itself, without its direction.
    #[must_use]
    pub const fn threshold(self) -> f64 {
        match self {
            Self::AtOrAbove(value) | Self::AtOrBelow(value) => value,
        }
    }

    /// Whether an observation has reached the problem side.
    #[must_use]
    pub fn reached_by(self, observed: f64) -> bool {
        match self {
            Self::AtOrAbove(value) => observed >= value,
            Self::AtOrBelow(value) => observed <= value,
        }
    }

    /// Whether moving in this direction takes a subject toward the problem.
    #[must_use]
    pub const fn approaches(self, slope: f64) -> bool {
        match self {
            Self::AtOrAbove(_) => slope > 0.0,
            Self::AtOrBelow(_) => slope < 0.0,
        }
    }
}

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
    /// Days remaining on a TLS certificate.
    ///
    /// The first subject that needs naming: a host has none, one, or forty certificates, and which
    /// ones matter is a fact about the deployment rather than about Linux. Declared rather than
    /// discovered — a subject that went looking for certificates would decide for the operator what
    /// is worth watching, which is the guess this whole layer refuses everywhere else.
    CertificateDaysRemaining,
    /// Whether one declared service is active: 1.0 or 0.0.
    ///
    /// Distinct from the count of failed units, which says *something* is wrong and not *what*. A
    /// service can be inactive without having failed — stopped, never started, masked — and an
    /// operator who declared it wants to know it is not running, whatever the reason.
    ServiceActive,
    /// How long since a declared backup last succeeded, in days.
    ///
    /// The one subject with no universal threshold. How stale a backup may get before it is a
    /// problem is a policy the operator holds, and two backups on one host can honestly disagree, so
    /// the number comes from the declaration and this table supplies none.
    BackupAgeDays,
    /// Share of the root filesystem in use, in [0.0, 1.0].
    RootFilesystemUsed,
    /// Share of the root filesystem's inodes in use, in [0.0, 1.0].
    ///
    /// Its own subject because it is its own failure. A filesystem out of inodes cannot create a
    /// file and has free bytes; every byte-based measure reads healthy while nothing can be written,
    /// which is the shape of failure this whole layer exists to catch. A host that logs many small
    /// files, or runs a mail spool, reaches this first.
    RootFilesystemInodesUsed,
    /// Share of the system-wide open file descriptor limit in use, in [0.0, 1.0].
    ///
    /// The other way a machine stops accepting work while looking well: memory is fine, the disk is
    /// fine, load is fine, and nothing can open a socket. Nothing else here would notice.
    OpenFileDescriptors,
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
            Self::CertificateDaysRemaining => "certificate.days.remaining",
            Self::ServiceActive => "service.active",
            Self::BackupAgeDays => "backup.age.days",
            Self::RootFilesystemUsed => "filesystem.root.used",
            Self::RootFilesystemInodesUsed => "filesystem.root.inodes.used",
            Self::OpenFileDescriptors => "files.open",
            Self::FailedUnits => "systemd.units.failed",
        }
    }

    /// The value at which this subject is a problem regardless of what is ordinary here, and which
    /// side of it is the problem.
    ///
    /// `None` for the subjects where there is no such number: a load average of 4 is a crisis on one
    /// machine and a Tuesday on another, and only the baseline can say which.
    ///
    /// One number, read by both the detector and the projection. They used to hold their own copies,
    /// which is how a system comes to report that a disk is fine and that it reaches trouble in three
    /// days — two true statements about two different thresholds.
    ///
    /// The direction travels with the number rather than beside it. It was a separate predicate
    /// until 2026-08-24, whose only answer was *rising*, with a comment saying that a subject where
    /// a fall is the problem would need its own answer. Two functions that can disagree about one
    /// threshold is the same shape as two copies of one number, and the failure is worse: a detector
    /// watching the wrong tail reports a certificate as healthy right up to the hour it expires.
    #[must_use]
    pub const fn alarming(self) -> Option<Alarming> {
        match self {
            Self::RootFilesystemUsed | Self::MemoryUsed => Some(Alarming::AtOrAbove(0.95)),
            // Lower than the byte threshold on purpose, and the same number for both by coincidence
            // rather than by kinship: each is a resource whose exhaustion is not gradual in effect.
            // A filesystem works and then cannot create a file; swap absorbs pressure and then the
            // next allocation stalls. Neither gives the warning a slowly filling disk does, so the
            // useful warning has to come earlier.
            Self::RootFilesystemInodesUsed | Self::SwapUsed => Some(Alarming::AtOrAbove(0.90)),
            Self::OpenFileDescriptors => Some(Alarming::AtOrAbove(0.85)),
            Self::MemoryPressure | Self::IoPressure | Self::CpuPressure => {
                Some(Alarming::AtOrAbove(40.0))
            }
            Self::FailedUnits => Some(Alarming::AtOrAbove(1.0)),
            // The first subject whose problem is a low value, and the reason the direction had to
            // travel with the number. Fourteen days is what an automated renewal has already had
            // several chances to do and has not.
            Self::CertificateDaysRemaining => Some(Alarming::AtOrBelow(14.0)),
            // Inactive is the problem, and a service is active or it is not: any threshold between
            // the two values means the same thing.
            Self::ServiceActive => Some(Alarming::AtOrBelow(0.5)),
            // No universal answer, deliberately. A number here would be this system deciding an
            // operator's backup policy for them, and a wrong one is worse than none: it would report
            // a healthy backup as stale, or a stale one as fine, on every host that disagreed.
            Self::LoadAverage | Self::BackupAgeDays => None,
        }
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
    Subject::RootFilesystemInodesUsed,
    Subject::OpenFileDescriptors,
    Subject::FailedUnits,
    Subject::CertificateDaysRemaining,
    Subject::ServiceActive,
    Subject::BackupAgeDays,
];

impl Subject {
    /// Whether this subject is about one named thing rather than about the host.
    ///
    /// A host has one root filesystem and any number of certificates. The universal subjects are
    /// readable anywhere with no configuration; a named one exists only because somebody declared
    /// it, and a reading for it that carried no name would be a measurement of nothing in
    /// particular.
    #[must_use]
    pub const fn needs_naming(self) -> bool {
        matches!(
            self,
            Self::CertificateDaysRemaining | Self::ServiceActive | Self::BackupAgeDays
        )
    }
}

/// What one measurement is about: a subject, and which one.
///
/// The two halves travelled separately until 2026-08-24 and were repeatedly dropped apart. The
/// windows were keyed by both, and everything downstream — deviations, evidence, projections —
/// keyed by the subject alone. Two certificates produced two windows, two findings, and one
/// deviation, because the second overwrote the first in a map that had nowhere to put the name.
///
/// A key rather than two fields on each type, so there is one thing to pass and no way to pass half
/// of it.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricKey {
    /// What is measured.
    pub subject: Subject,
    /// Which one, for a subject about a named thing.
    ///
    /// `None` for the universal subjects, which are about the host itself.
    #[serde(default)]
    pub instance: Option<String>,
}

impl MetricKey {
    /// A key for a subject about the host itself.
    #[must_use]
    pub const fn host(subject: Subject) -> Self {
        Self {
            subject,
            instance: None,
        }
    }

    /// A key for one named thing.
    #[must_use]
    pub const fn named(subject: Subject, instance: String) -> Self {
        Self {
            subject,
            instance: Some(instance),
        }
    }

    /// How this reads to a person.
    ///
    /// A page showing four rows all called `certificate.days.remaining` is a page nobody can act on.
    #[must_use]
    pub fn label(&self) -> String {
        match &self.instance {
            Some(name) => format!("{} ({name})", self.subject.name()),
            None => self.subject.name().to_owned(),
        }
    }

    /// What an action about this would be performed on, if anything.
    ///
    /// The bridge from measurement to remediation. A finding about
    /// `service.active (postgresql.service)` can name the unit an action would restart, instead of
    /// the placeholder a proposal falls back to when it does not know which one it means.
    #[must_use]
    pub fn target(&self) -> Option<String> {
        let instance = self.instance.as_ref()?;
        Some(match self.subject {
            Subject::ServiceActive => format!("systemd:{instance}"),
            Subject::CertificateDaysRemaining | Subject::BackupAgeDays => {
                format!("path:{instance}")
            }
            _ => instance.clone(),
        })
    }
}

/// One number, observed at one instant.
///
/// Deliberately has no path into the Journal. There is no `into_contribution`, no `Kind`, and no
/// conversion anywhere in this tree — a reading is transient by construction and not by policy.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reading {
    /// What was observed, and which one.
    pub key: MetricKey,
    /// What it was.
    pub value: f64,
    /// When.
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
}

impl Reading {
    /// What was measured.
    #[must_use]
    pub const fn subject(&self) -> Subject {
        self.key.subject
    }

    /// How this reading is named to a person.
    #[must_use]
    pub fn label(&self) -> String {
        self.key.label()
    }
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

/// What is known about one thing this host was told to watch.
///
/// Four states rather than a value and its absence. A declared thing with no reading used to be
/// simply missing from every surface, which reads exactly like a thing nobody declared — and the
/// two are opposites. An operator who declared a certificate and sees nothing about it has been
/// told, by the silence, that it is fine.
///
/// The three unhappy states are kept apart because they call for different actions. Never read is a
/// probe that has not run or a path that does not exist; read failed is a file this process cannot
/// open, which is usually a permission; stale is a probe that worked and has stopped, which is
/// usually the sampler and not the thing sampled.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum Watching {
    /// Read, recently, with this value.
    #[serde(rename_all = "camelCase")]
    Observed {
        /// What it was.
        value: f64,
        /// When it was read.
        #[serde(with = "time::serde::rfc3339")]
        at: OffsetDateTime,
    },
    /// Declared, and never once read.
    NeverRead,
    /// Read attempted, and the attempt did not produce a number.
    #[serde(rename_all = "camelCase")]
    ReadFailed {
        /// When the last attempt failed.
        #[serde(with = "time::serde::rfc3339")]
        since: OffsetDateTime,
    },
    /// Read once, and not lately.
    #[serde(rename_all = "camelCase")]
    Stale {
        /// The last reading that did arrive.
        #[serde(with = "time::serde::rfc3339")]
        last_read: OffsetDateTime,
    },
}

impl Watching {
    /// The short name a surface labels this state with.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Observed { .. } => "observed",
            Self::NeverRead => "never-read",
            Self::ReadFailed { .. } => "read-failed",
            Self::Stale { .. } => "stale",
        }
    }

    /// Whether this state means the host actually knows something about the thing right now.
    #[must_use]
    pub const fn is_observed(&self) -> bool {
        matches!(self, Self::Observed { .. })
    }
}

/// One watched thing, and what is known about it.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchedResource {
    /// What is watched, and which one.
    pub key: MetricKey,
    /// What is known about it.
    pub state: Watching,
}

/// One reading behind a finding, and what it is about.
///
/// A pair of subject and deviation lost the name of the thing measured, so a finding about one
/// certificate could carry another certificate's numbers. Carrying a `Deviation` and nothing else
/// then lost something subtler and worse: a categorical finding needs no baseline, so a first
/// reading of a filesystem at 97% produced a `StorageExhaustion` of `Strong` strength citing
/// **nothing at all**. An insight that cannot show its readings is indistinguishable from one a
/// model made up, and this one was reaching that state by the ordinary route.
///
/// So the observation is the required part and the baseline is the optional one. That is also the
/// true shape of the two detectors: one asks *is this value a problem*, which needs one reading,
/// and the other asks *is this value unusual here*, which needs a window.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InsightEvidence {
    /// What the reading was about.
    pub key: MetricKey,
    /// What was observed.
    pub observed: f64,
    /// How far it sat from ordinary, when this host has watched long enough to have an ordinary.
    ///
    /// `None` is a real answer and says so: the value is a problem regardless of what is usual
    /// here, and nothing yet establishes what is usual here. Reporting a fabricated baseline, or
    /// dropping the reading for want of one, are the two ways this used to go wrong.
    #[serde(default)]
    pub deviation: Option<Deviation>,
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
    /// What this is about, for a finding about one named thing.
    ///
    /// `None` for a finding about the host itself. This is what a remediation proposal names as its
    /// target: without it a proposal about a service knows only that *a* service is down, and falls
    /// back to a placeholder unit — which is a proposal to restart whatever came to mind.
    #[serde(default)]
    pub about: Option<MetricKey>,
    /// The readings that led to it, and how far each was from ordinary.
    ///
    /// Carried rather than summarised: *why do you think that* is answered by looking, in the same
    /// way an activation path answers *why did you think of honey*. An insight that could not show
    /// its readings would be indistinguishable from one a model made up.
    pub because: Vec<InsightEvidence>,
    /// How well the evidence supports it.
    pub strength: EvidenceStrength,
    /// When the host concluded it.
    #[serde(with = "time::serde::rfc3339")]
    pub concluded_at: OffsetDateTime,
    /// When the behaviour started, as far as the window can tell.
    #[serde(with = "time::serde::rfc3339")]
    pub since: OffsetDateTime,
}

/// The namespace insight identities are derived in.
///
/// A fixed UUID, so the derivation is stable across builds and machines.
const INSIGHT_NAMESPACE: Uuid = Uuid::from_u128(0x6379_626f_755f_696e_7369_6768_745f_7631);

impl SystemInsight {
    /// The identity one ongoing condition has, for as long as it is the same condition.
    ///
    /// Derived rather than generated. A random identity per read meant that two requests a second
    /// apart described one physically identical situation with two different identities — harmless
    /// while nothing referred to them, and an architectural defect the moment an action proposal
    /// cites one as its cause.
    ///
    /// The three parts are what make a condition itself: what was concluded, what it is about, and
    /// when it began. A memory-pressure episode that ends and returns is a new episode with a new
    /// `since`, and gets a new identity, which is correct — it is not the same occurrence.
    #[must_use]
    pub fn derive_id(finding: Finding, about: Option<&MetricKey>, since: OffsetDateTime) -> Uuid {
        let about = about.map_or_else(String::new, MetricKey::label);
        let seed = format!("{}|{about}|{}", finding.name(), since.unix_timestamp());
        Uuid::new_v5(&INSIGHT_NAMESPACE, seed.as_bytes())
    }
}

/// What a host can conclude about itself.
///
/// A closed set, and short on purpose. Every entry is something with a distinct remedy; a finding
/// nobody can act on differently from another finding is not a finding, it is a synonym.
//  and  so a finding can be part of a key elsewhere without that layer restating the
// vocabulary as strings.  seeds by finding, and the first draft of that took text: the
// drift showed up immediately, with one side writing  and this one saying
// .
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
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
    /// A declared service is not running.
    ServiceInactive,
    /// A declared backup is older than the operator said it should get.
    BackupStale,
    /// A watched certificate is close to expiry, or past it.
    ///
    /// Its own finding because nothing else here is about a deadline. Every other failure is a
    /// resource under pressure that a person can relieve; this one arrives on a date regardless of
    /// what the machine is doing, and the only remedy is renewal.
    CertificateExpiring,
    /// The machine is running out of file descriptors.
    ///
    /// Its own finding rather than folded into storage, because the remedy is different: nothing is
    /// full and deleting things frees nothing. Something is holding descriptors it is not closing.
    FileDescriptorExhaustion,
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
            Self::FileDescriptorExhaustion => "file-descriptor-exhaustion",
            Self::CertificateExpiring => "certificate-expiring",
            Self::ServiceInactive => "service-inactive",
            Self::BackupStale => "backup-stale",
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
    fn the_one_subject_with_no_universal_threshold_says_so() {
        // A number here would be this system deciding an operator's backup policy for them, and a
        // wrong one is worse than none: it reports a healthy backup as stale, or a stale one as
        // fine, on every host that disagrees.
        assert_eq!(Subject::BackupAgeDays.alarming(), None);
        assert!(Subject::BackupAgeDays.needs_naming());

        // And the ones that do have a universal answer still have it.
        assert_eq!(
            Subject::ServiceActive.alarming(),
            Some(Alarming::AtOrBelow(0.5))
        );
    }

    #[test]
    fn a_named_subject_is_labelled_by_the_thing_it_is_about() {
        // A page showing four rows all called certificate.days.remaining is a page nobody can act
        // on.
        let named = Reading {
            key: MetricKey::named(
                Subject::CertificateDaysRemaining,
                "/etc/ssl/example.pem".to_owned(),
            ),
            value: 31.0,
            at: OffsetDateTime::from_unix_timestamp(1_787_000_000).expect("an instant"),
        };
        assert_eq!(
            named.label(),
            "certificate.days.remaining (/etc/ssl/example.pem)"
        );

        let universal = Reading {
            key: MetricKey::host(Subject::LoadAverage),
            value: 0.4,
            at: OffsetDateTime::from_unix_timestamp(1_787_000_000).expect("an instant"),
        };
        assert_eq!(universal.label(), "load.average");
    }

    #[test]
    fn only_the_subjects_that_are_about_one_thing_need_naming() {
        for subject in ALL_SUBJECTS {
            assert_eq!(
                subject.needs_naming(),
                matches!(
                    subject,
                    Subject::CertificateDaysRemaining
                        | Subject::ServiceActive
                        | Subject::BackupAgeDays
                ),
                "{subject:?}"
            );
        }
    }

    #[test]
    fn a_threshold_carries_which_side_of_it_is_the_problem() {
        // The failure this exists to make unrepresentable: a subject where a fall is the problem,
        // measured by a detector that only watches the rising tail, reads healthy right up to the
        // hour it expires.
        let filling = Alarming::AtOrAbove(0.95);
        assert!(filling.reached_by(0.96));
        assert!(!filling.reached_by(0.10));
        assert!(filling.approaches(0.001));
        assert!(!filling.approaches(-0.001));

        let expiring = Alarming::AtOrBelow(7.0);
        assert!(expiring.reached_by(3.0));
        assert!(!expiring.reached_by(90.0));
        assert!(expiring.approaches(-0.001));
        assert!(!expiring.approaches(0.001));

        assert!((filling.threshold() - 0.95).abs() < f64::EPSILON);
        assert!((expiring.threshold() - 7.0).abs() < f64::EPSILON);
    }

    #[test]
    fn a_flat_subject_approaches_nothing_in_either_direction() {
        // Zero slope is not movement toward a problem, whichever side the problem is on. A detector
        // that read it as approaching would find an arrival date for a machine that is not moving.
        assert!(!Alarming::AtOrAbove(0.95).approaches(0.0));
        assert!(!Alarming::AtOrBelow(7.0).approaches(0.0));
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
            about: None,
            because: vec![InsightEvidence {
                key: MetricKey::host(Subject::RootFilesystemUsed),
                observed: 0.96,
                deviation: Some(Deviation {
                    ordinary: 0.62,
                    spread: 0.01,
                    observed: 0.94,
                    spreads_away: 32.0,
                }),
            }],
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
            about: None,
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
