// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Watching a host, and deciding when what it is doing is worth saying something about.

use std::collections::BTreeMap;
use std::sync::RwLock;

use cybou_protocol::telemetry::{
    ALL_SUBJECTS, Alarming, Deviation, EvidenceStrength, Finding, InsightEvidence, MetricKey,
    Reading, Subject, SystemInsight, WatchedResource,
};
use time::{Duration, OffsetDateTime};

use crate::baseline::{SMALLEST_JUDGEABLE_WINDOW, deviation};
use crate::series::Series;

/// How far from ordinary a reading has to be before it is worth a second look.
///
/// Six spreads is deliberately far. A monitoring system that speaks at three is a monitoring system
/// people mute, and a muted detector detects nothing — the failure mode of an alerting system is
/// almost never that it missed something, it is that everybody stopped reading it.
const NOTEWORTHY_SPREADS: f64 = 6.0;

/// How long a gap turns a window that was being filled into one that has stopped.
///
/// Six missed samples at one every ten seconds. Long enough that a slow tick or a busy moment is
/// not reported as a broken probe, short enough that an operator looking at the page is not shown a
/// four-minute-old number as if it were current.
pub const STALE_AFTER: Duration = Duration::seconds(60);

/// How full a filesystem has to be before fullness is the story regardless of statistics.
///
/// Some things do not need a baseline. A disk at 95% is a problem on a host where it has been at 95%
/// for a month, and a purely statistical detector would say nothing precisely because it is normal
/// here. Categorical facts and statistical deviations are different evidence and both are kept.
///
/// Read from the subject rather than held here. The detector and the projection used to keep their
/// own copies, which is how a system comes to report that a disk is fine and that it reaches trouble
/// in three days — two true statements about two different numbers.
fn alarming_for(subject: Subject) -> Alarming {
    // A subject with no categorical threshold is one no observation can reach, so the statistical
    // half is the only judge of it. Expressed as an unreachable threshold rather than as an early
    // return, so every caller reads the same shape.
    subject
        .alarming()
        .unwrap_or(Alarming::AtOrAbove(f64::INFINITY))
}

/// What the telemetry organ holds and concludes.
pub struct TelemetryCore {
    /// Keyed by what it is about and which one. A named subject has one window per
    /// declared thing; a universal one has a single window with no name.
    windows: RwLock<BTreeMap<MetricKey, Series>>,
    span: Duration,
    capacity: usize,
}

impl TelemetryCore {
    /// Watch every subject over a window of this span, holding at most `capacity` readings each.
    #[must_use]
    pub fn new(span: Duration, capacity: usize) -> Self {
        // Only the universal subjects. A named one exists because somebody declared it, and a
        // window created before the declaration would report a certificate nobody asked about.
        let windows = ALL_SUBJECTS
            .iter()
            .filter(|subject| !subject.needs_naming())
            .map(|subject| {
                (
                    MetricKey::host(*subject),
                    Series::new(*subject, span, capacity),
                )
            })
            .collect();
        Self {
            windows: RwLock::new(windows),
            span,
            capacity,
        }
    }

    /// The span each window covers.
    #[must_use]
    pub const fn span(&self) -> Duration {
        self.span
    }

    /// The most readings each window holds.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Begin watching one named thing.
    ///
    /// Called for each declaration. A window that already exists is left alone, so re-reading the
    /// declarations does not discard the history of everything still declared.
    pub fn watch(&self, subject: Subject, instance: String, alarming: Option<Alarming>) {
        if let Ok(mut windows) = self.windows.write() {
            let key = MetricKey::named(subject, instance);
            windows.entry(key.clone()).or_insert_with(|| {
                Series::judged(
                    key,
                    // What the declaration chose, or what the subject says by default. A named
                    // subject with neither is watched and never judged categorically, which is the
                    // honest state for something nobody has set a limit on.
                    alarming.or_else(|| subject.alarming()),
                    self.span,
                    self.capacity,
                )
            });
        }
    }

    /// Record one observation.
    ///
    /// A reading for something not being watched is dropped rather than starting a window. Windows
    /// begin by declaration; one that appeared because a reading arrived would let a probe decide
    /// what this host cares about.
    pub fn observe(&self, reading: Reading) {
        if let Ok(mut windows) = self.windows.write()
            && let Some(series) = windows.get_mut(&reading.key)
        {
            series.observe(reading);
        }
    }

    /// Record that an attempt to read one watched thing produced no number.
    ///
    /// A window that is not being watched is ignored, for the same reason a reading for one is: a
    /// probe must not be able to add to what this host cares about.
    pub fn note_unreadable(&self, key: &MetricKey, at: OffsetDateTime) {
        if let Ok(mut windows) = self.windows.write()
            && let Some(series) = windows.get_mut(key)
        {
            series.note_unreadable(at);
        }
    }

    /// What is known about every watched thing, including the ones nothing is known about.
    ///
    /// The whole list, not the part that worked. A declared thing with no reading used to be simply
    /// absent from every surface, which reads exactly like a thing nobody declared — and an operator
    /// who declared a certificate and sees nothing about it has been told, by the silence, that it
    /// is fine.
    #[must_use]
    pub fn watching(&self, now: OffsetDateTime, stale_after: Duration) -> Vec<WatchedResource> {
        let Ok(windows) = self.windows.read() else {
            return Vec::new();
        };
        windows
            .values()
            .map(|series| WatchedResource {
                key: series.key().clone(),
                state: series.state(now, stale_after),
            })
            .collect()
    }

    /// The most recent reading for each subject that has one.
    ///
    /// A subject with no reading is absent rather than zero. A host without pressure accounting, or
    /// without swap, has nothing to say about them, and a surface showing `0.0` would be showing a
    /// perfectly calm machine where there is in fact no measurement.
    #[must_use]
    pub fn latest(&self) -> Vec<Reading> {
        self.windows
            .read()
            .map(|windows| windows.values().filter_map(Series::latest).collect())
            .unwrap_or_default()
    }

    /// How each subject currently sits relative to what is ordinary for this host.
    ///
    /// Subjects whose window is too short to have an opinion are absent, not neutral.
    #[must_use]
    pub fn deviations(&self) -> Vec<(MetricKey, Deviation)> {
        let Ok(windows) = self.windows.read() else {
            return Vec::new();
        };
        windows
            .values()
            .filter_map(|series| {
                let latest = series.latest()?;
                Some((
                    series.key().clone(),
                    deviation(&series.values(), latest.value)?,
                ))
            })
            .collect()
    }

    /// Whether enough has been watched to say anything at all.
    ///
    /// The honest answer to *how is this host* during the first minutes after a restart is that
    /// Cybou has not been watching long enough to know, and a surface needs to be able to say that
    /// rather than report a confident all-clear built on four readings.
    #[must_use]
    pub fn has_watched_enough(&self) -> bool {
        self.windows.read().is_ok_and(|windows| {
            windows
                .values()
                .any(|series| series.len() >= SMALLEST_JUDGEABLE_WINDOW)
        })
    }

    /// Where each watched subject is heading, and when it becomes a problem.
    ///
    /// Only the subjects that have a threshold at all. A load average has none — 4 is a crisis on
    /// one machine and a Tuesday on another — so projecting it against a number would be inventing
    /// the number.
    #[must_use]
    pub fn projections(&self, now: OffsetDateTime) -> Vec<(MetricKey, crate::trend::Projection)> {
        let Ok(windows) = self.windows.read() else {
            return Vec::new();
        };
        windows
            .values()
            .filter_map(|series| {
                let alarming = series.alarming()?;
                Some((
                    // The whole key, or a page of rows all called the same thing.
                    series.key().clone(),
                    crate::trend::project(series, alarming, now)?,
                ))
            })
            .collect()
    }

    /// What this host currently concludes about itself.
    ///
    /// Every conclusion carries the readings that produced it and is a hypothesis, never a fact.
    /// Findings are returned in a fixed order so two runs over the same windows compare.
    #[must_use]
    pub fn insights(&self, now: OffsetDateTime) -> Vec<SystemInsight> {
        let Ok(windows) = self.windows.read() else {
            return Vec::new();
        };
        // Keyed by the whole key. Keyed by subject alone, two declared certificates produced two
        // windows, two findings and one deviation: the second overwrote the first, and whichever
        // finding was built second cited the other certificate numbers as its evidence.
        let deviations: BTreeMap<MetricKey, Deviation> = windows
            .values()
            .filter_map(|series| {
                let latest = series.latest()?;
                Some((
                    series.key().clone(),
                    deviation(&series.values(), latest.value)?,
                ))
            })
            .collect();

        let mut found = Vec::new();
        let mut explained: Vec<MetricKey> = Vec::new();

        categorical(&windows, &deviations, now, &mut found, &mut explained);
        pressures(&windows, &deviations, now, &mut found, &mut explained);
        unexplained(&windows, &deviations, now, &mut found, &explained);
        found
    }
}

/// The findings that do not need a baseline.
///
/// Some things are a problem on a host where they have always been the case, and a purely
/// statistical detector says nothing about them precisely because they are normal here.
fn categorical(
    windows: &BTreeMap<MetricKey, Series>,
    deviations: &BTreeMap<MetricKey, Deviation>,
    now: OffsetDateTime,
    found: &mut Vec<SystemInsight>,
    explained: &mut Vec<MetricKey>,
) {
    // Categorical first. Some things do not need a baseline: a filesystem at 95% is a problem on
    // a host where it has been at 95% for a month, and a purely statistical detector would say
    // nothing precisely because it is normal here.
    // Both ways a filesystem stops working, and they are one finding because they have one family
    // of remedies. A host out of inodes has free bytes: every byte-based measure reads healthy while
    // nothing can be created, which is why the inode share is watched at all.
    for subject in [
        Subject::RootFilesystemUsed,
        Subject::RootFilesystemInodesUsed,
    ] {
        let key = MetricKey::host(subject);
        if let Some(series) = windows.get(&key)
            && let Some(latest) = series.latest()
            && alarming_for(subject).reached_by(latest.value)
        {
            explained.push(key.clone());
            let since = series
                .continuously_since(alarming_for(subject))
                .unwrap_or(latest.at);
            found.push(SystemInsight {
                insight_id: SystemInsight::derive_id(Finding::StorageExhaustion, None, since),
                finding: Finding::StorageExhaustion,
                about: None,
                because: evidence(deviations, &[key]),
                strength: EvidenceStrength::Strong,
                concluded_at: now,
                since,
            });
        }
    }

    // One finding per declared thing rather than one naming a count: an operator with four
    // certificates needs to know which. Written once over every named subject, because three copies
    // of this loop is three places for the next named subject to be forgotten.
    for series in windows.values() {
        if !series.subject().needs_naming() {
            continue;
        }
        let Some(latest) = series.latest() else {
            continue;
        };
        // A named subject may have no threshold at all — a backup nobody set a staleness policy for
        // is watched and never judged, which is the honest state rather than a default nobody chose.
        let Some(alarming) = series.alarming() else {
            continue;
        };
        if !alarming.reached_by(latest.value) {
            continue;
        }
        let Some(finding) = named_finding(series.subject()) else {
            continue;
        };
        let key = series.key().clone();
        explained.push(key.clone());
        let since = series.continuously_since(alarming).unwrap_or(latest.at);
        found.push(SystemInsight {
            insight_id: SystemInsight::derive_id(finding, Some(&key), since),
            finding,
            // Which certificate, which unit, which backup. Without this a proposal to act on the
            // finding would know only that something of this kind is wrong.
            about: Some(key.clone()),
            because: evidence(deviations, &[key]),
            strength: EvidenceStrength::Strong,
            concluded_at: now,
            since,
        });
    }

    let descriptors = MetricKey::host(Subject::OpenFileDescriptors);
    if let Some(series) = windows.get(&descriptors)
        && let Some(latest) = series.latest()
        && alarming_for(Subject::OpenFileDescriptors).reached_by(latest.value)
    {
        // The other way a machine stops accepting work while looking well: memory fine, disk fine,
        // load fine, and nothing can open a socket.
        explained.push(descriptors.clone());
        let since = series
            .continuously_since(alarming_for(Subject::OpenFileDescriptors))
            .unwrap_or(latest.at);
        found.push(SystemInsight {
            insight_id: SystemInsight::derive_id(Finding::FileDescriptorExhaustion, None, since),
            finding: Finding::FileDescriptorExhaustion,
            about: None,
            because: evidence(deviations, &[descriptors]),
            strength: EvidenceStrength::Strong,
            concluded_at: now,
            since,
        });
    }

    let failed = MetricKey::host(Subject::FailedUnits);
    if let Some(series) = windows.get(&failed)
        && let Some(latest) = series.latest()
        && latest.value >= 1.0
    {
        explained.push(failed.clone());
        let since = series
            .continuously_since(alarming_for(Subject::FailedUnits))
            .unwrap_or(latest.at);
        found.push(SystemInsight {
            insight_id: SystemInsight::derive_id(Finding::ServiceFailure, None, since),
            finding: Finding::ServiceFailure,
            // The count says something is wrong, not which unit. Nothing to act on by name.
            about: None,
            because: evidence(deviations, &[failed]),
            strength: EvidenceStrength::Strong,
            concluded_at: now,
            since,
        });
    }
}

/// The findings that are a matter of degree, each with the subject that corroborates it.
///
/// Memory pressure alone is weak; memory pressure with swap growing is the same story told twice,
/// which is what makes it stronger.
fn pressures(
    windows: &BTreeMap<MetricKey, Series>,
    deviations: &BTreeMap<MetricKey, Deviation>,
    now: OffsetDateTime,
    found: &mut Vec<SystemInsight>,
    explained: &mut Vec<MetricKey>,
) {
    for (subject, corroborator, finding) in [
        (
            Subject::MemoryPressure,
            Some(Subject::SwapUsed),
            Finding::MemoryPressure,
        ),
        (Subject::IoPressure, None, Finding::IoSaturation),
        (
            Subject::CpuPressure,
            Some(Subject::LoadAverage),
            Finding::CpuSaturation,
        ),
    ] {
        let key = MetricKey::host(subject);
        let Some(series) = windows.get(&key) else {
            continue;
        };
        let Some(latest) = series.latest() else {
            continue;
        };
        let unusual = deviations
            .get(&key)
            .is_some_and(|found| found.spreads_away >= NOTEWORTHY_SPREADS);
        if !unusual && !alarming_for(subject).reached_by(latest.value) {
            continue;
        }
        let corroborator = corroborator.map(MetricKey::host);
        let corroborated = corroborator.as_ref().is_some_and(|other| {
            deviations
                .get(other)
                .is_some_and(|found| found.spreads_away >= NOTEWORTHY_SPREADS)
        });
        explained.push(key.clone());
        let mut cited = vec![key];
        if corroborated && let Some(other) = corroborator {
            explained.push(other.clone());
            cited.push(other);
        }
        let since = series
            .continuously_since(alarming_for(subject))
            .unwrap_or(latest.at);
        found.push(SystemInsight {
            insight_id: SystemInsight::derive_id(finding, None, since),
            finding,
            about: None,
            because: evidence(deviations, &cited),
            strength: if corroborated {
                EvidenceStrength::Moderate
            } else {
                EvidenceStrength::Weak
            },
            concluded_at: now,
            since,
        });
    }
}

/// Whatever is still out of range and unaccounted for.
///
/// A detector that only reported what it had a name for would be silent exactly when a host is
/// doing something nobody anticipated, which is the case an operator most wants to hear about.
fn unexplained(
    windows: &BTreeMap<MetricKey, Series>,
    deviations: &BTreeMap<MetricKey, Deviation>,
    now: OffsetDateTime,
    found: &mut Vec<SystemInsight>,
    explained: &[MetricKey],
) {
    for (key, found_deviation) in deviations {
        if explained.contains(key) || found_deviation.spreads_away < NOTEWORTHY_SPREADS {
            continue;
        }
        let since = windows
            .get(key)
            .and_then(Series::latest)
            .map_or(now, |latest| latest.at);
        found.push(SystemInsight {
            insight_id: SystemInsight::derive_id(Finding::UnexplainedDeviation, Some(key), since),
            finding: Finding::UnexplainedDeviation,
            // Whatever it was about. An unexplained deviation on one of two certificates is about
            // that certificate, and a reader who cannot tell which is not much better off.
            about: key.instance.as_ref().map(|_| key.clone()),
            because: evidence(deviations, std::slice::from_ref(key)),
            strength: EvidenceStrength::Weak,
            concluded_at: now,
            since,
        });
    }
}

/// What a named subject reaching its threshold means.
///
/// `None` for a named subject this build watches and has no finding for, which is a gap rather than
/// a decision — the compiler cannot force this the way it forces a match arm, so the loop above
/// skips what it cannot name rather than inventing a finding for it.
const fn named_finding(subject: Subject) -> Option<Finding> {
    match subject {
        Subject::CertificateDaysRemaining => Some(Finding::CertificateExpiring),
        Subject::ServiceActive => Some(Finding::ServiceInactive),
        Subject::BackupAgeDays => Some(Finding::BackupStale),
        _ => None,
    }
}

/// The deviations behind a finding, in the order the finding cited them.
fn evidence(
    deviations: &BTreeMap<MetricKey, Deviation>,
    cited: &[MetricKey],
) -> Vec<InsightEvidence> {
    cited
        .iter()
        .filter_map(|key| {
            deviations.get(key).map(|found| InsightEvidence {
                key: key.clone(),
                deviation: *found,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use cybou_protocol::telemetry::Watching;

    use super::*;

    fn at(offset: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_787_000_000 + offset).expect("a fixed instant")
    }

    fn core() -> TelemetryCore {
        TelemetryCore::new(Duration::hours(1), 240)
    }

    /// Feed a quiet history, then whatever comes after it.
    fn history(core: &TelemetryCore, subject: Subject, quiet: f64, then: &[f64]) {
        for index in 0..24 {
            core.observe(Reading {
                key: MetricKey::host(subject),
                value: quiet + f64::from(u8::try_from(index % 3).unwrap_or(0)) * 0.01,
                at: at(index * 10),
            });
        }
        for (index, value) in then.iter().enumerate() {
            #[allow(clippy::cast_possible_wrap, reason = "a handful of test readings")]
            let offset = 240 + (index as i64) * 10;
            core.observe(Reading {
                key: MetricKey::host(subject),
                value: *value,
                at: at(offset),
            });
        }
    }

    #[test]
    fn a_host_that_has_only_just_started_says_it_has_not_watched_enough() {
        // The honest answer during the first minutes after a restart. A surface reporting a
        // confident all-clear built on four readings is worse than one saying it does not know yet.
        let core = core();
        assert!(!core.has_watched_enough());
        for index in 0..4 {
            core.observe(Reading {
                key: MetricKey {
                    subject: Subject::LoadAverage,
                    instance: None,
                },
                value: 0.4,
                at: at(index),
            });
        }
        assert!(!core.has_watched_enough());
        assert!(core.insights(at(100)).is_empty());
    }

    #[test]
    fn a_full_filesystem_is_a_finding_even_where_it_has_always_been_full() {
        // Some things do not need a baseline. A purely statistical detector would say nothing here
        // precisely because 96% is perfectly normal for this host.
        let core = core();
        history(&core, Subject::RootFilesystemUsed, 0.96, &[]);

        let insights = core.insights(at(300));
        let storage = insights
            .iter()
            .find(|insight| insight.finding == Finding::StorageExhaustion)
            .expect("a full disk is a finding regardless of statistics");
        assert_eq!(storage.strength, EvidenceStrength::Strong);
        assert!(!storage.because.is_empty(), "the finding cites nothing");
    }

    #[test]
    fn a_full_window_of_every_subject_is_projected_in_a_blink() {
        // The number that matters on a one-vCPU host: a page load projects every watched subject,
        // and the honest way to know it is affordable is to measure it rather than to reason about
        // the exponent. Unbounded, this same fixture is eleven subjects times 2.33 million pairwise
        // slopes.
        let core = TelemetryCore::new(Duration::hours(6), 2160);
        for name in ["/etc/ssl/a.pem", "/etc/ssl/b.pem", "/etc/ssl/c.pem"] {
            core.watch(Subject::CertificateDaysRemaining, name.to_owned(), None);
        }
        for tick in 0..2160i64 {
            #[allow(clippy::cast_precision_loss, reason = "a benchmark fixture")]
            let drift = tick as f64;
            for subject in [
                Subject::LoadAverage,
                Subject::MemoryUsed,
                Subject::MemoryPressure,
                Subject::IoPressure,
                Subject::CpuPressure,
                Subject::RootFilesystemUsed,
                Subject::RootFilesystemInodesUsed,
                Subject::OpenFileDescriptors,
            ] {
                core.observe(Reading {
                    key: MetricKey::host(subject),
                    value: 0.40 + drift * 0.0001,
                    at: at(tick * 10),
                });
            }
            for name in ["/etc/ssl/a.pem", "/etc/ssl/b.pem", "/etc/ssl/c.pem"] {
                core.observe(Reading {
                    key: MetricKey {
                        subject: Subject::CertificateDaysRemaining,
                        instance: Some(name.to_owned()),
                    },
                    value: 90.0 - drift * 0.001,
                    at: at(tick * 10),
                });
            }
        }

        let started = std::time::Instant::now();
        let projected = core.projections(at(2160 * 10));
        let took = started.elapsed();

        assert!(
            projected.len() >= 10,
            "{} subjects projected",
            projected.len()
        );
        assert!(
            took < std::time::Duration::from_millis(250),
            "projecting a full window of every subject took {took:?}"
        );
    }

    #[test]
    fn a_declared_certificate_is_watched_projected_and_found() {
        // The whole named-subject path, from declaration to finding. A certificate losing a day a
        // day is approaching its floor, which is the direction the threshold now carries.
        let core = core();
        core.watch(
            Subject::CertificateDaysRemaining,
            "/etc/ssl/example.pem".to_owned(),
            None,
        );
        for tick in 0..40 {
            core.observe(Reading {
                key: MetricKey {
                    subject: Subject::CertificateDaysRemaining,
                    instance: Some("/etc/ssl/example.pem".to_owned()),
                },
                #[allow(clippy::cast_precision_loss, reason = "a test fixture of forty ticks")]
                value: 40.0 - (tick as f64) * 0.5,
                // A minute apart: the window in this fixture spans an hour, and readings an hour
                // apart would leave two of them in it.
                at: at(tick * 60),
            });
        }

        let latest = core.latest();
        let certificate = latest
            .iter()
            .find(|reading| reading.subject() == Subject::CertificateDaysRemaining)
            .expect("the declared certificate is watched");
        assert_eq!(
            certificate.key.instance.as_deref(),
            Some("/etc/ssl/example.pem")
        );

        let projected = core.projections(at(40 * 60));
        let (key, heading) = projected
            .iter()
            .find(|(key, _)| key.subject == Subject::CertificateDaysRemaining)
            .expect("a certificate is projected");
        assert_eq!(key.instance.as_deref(), Some("/etc/ssl/example.pem"));
        assert!(
            matches!(heading.reaching, crate::trend::Reaching::AtThisRate { .. }),
            "{heading:?}"
        );
    }

    #[test]
    fn each_expiring_certificate_is_its_own_finding() {
        // An operator with four certificates needs to know which. A single finding naming a count
        // would send them looking through the four themselves.
        let core = core();
        for name in ["/etc/ssl/a.pem", "/etc/ssl/b.pem"] {
            core.watch(Subject::CertificateDaysRemaining, name.to_owned(), None);
            for tick in 0..20 {
                core.observe(Reading {
                    key: MetricKey {
                        subject: Subject::CertificateDaysRemaining,
                        instance: Some(name.to_owned()),
                    },
                    value: 3.0,
                    at: at(tick * 60),
                });
            }
        }

        let expiring = core
            .insights(at(2000))
            .into_iter()
            .filter(|insight| insight.finding == Finding::CertificateExpiring)
            .count();
        assert_eq!(expiring, 2);
    }

    #[test]
    fn a_declared_thing_that_was_never_read_is_not_silence() {
        // The failure this closes: a declared certificate with no reading was simply absent from
        // every surface, which reads exactly like a certificate nobody declared. The operator who
        // declared it is the one being told, by that silence, that it is fine.
        let core = core();
        core.watch(
            Subject::CertificateDaysRemaining,
            "/etc/ssl/never.pem".to_owned(),
            None,
        );

        let watched = core.watching(at(100), Duration::seconds(60));
        let never = watched
            .iter()
            .find(|resource| resource.key.instance.as_deref() == Some("/etc/ssl/never.pem"))
            .expect("a declared thing is listed even with nothing known about it");
        assert_eq!(never.state, Watching::NeverRead);
    }

    #[test]
    fn two_services_one_unreadable_are_told_apart() {
        // The two are opposites and used to render identically. One is a unit this host looked at
        // and found running; the other is a unit it could not ask about at all, and reporting the
        // second as absent is reporting it as fine.
        let core = core();
        for unit in ["postgresql.service", "caddy.service"] {
            core.watch(Subject::ServiceActive, unit.to_owned(), None);
        }
        for tick in 0..24 {
            core.observe(Reading {
                key: MetricKey::named(Subject::ServiceActive, "postgresql.service".to_owned()),
                value: 1.0,
                at: at(tick * 60),
            });
        }
        core.note_unreadable(
            &MetricKey::named(Subject::ServiceActive, "caddy.service".to_owned()),
            at(1400),
        );

        let watched = core.watching(at(1440), Duration::seconds(60));
        let state = |unit: &str| {
            watched
                .iter()
                .find(|resource| resource.key.instance.as_deref() == Some(unit))
                .map(|resource| resource.state.clone())
                .expect("both are listed")
        };
        assert!(
            state("postgresql.service").is_observed(),
            "{:?}",
            state("postgresql.service")
        );
        assert_eq!(
            state("caddy.service"),
            Watching::ReadFailed { since: at(1400) },
            "an unreadable unit was reported as something other than unreadable"
        );
    }

    #[test]
    fn a_probe_that_stopped_is_not_a_probe_that_failed() {
        // Different remedies. A file this process cannot open is usually a permission; a window
        // that stopped being filled is usually the sampler, and is not about the file at all.
        let core = core();
        core.watch(
            Subject::CertificateDaysRemaining,
            "/etc/ssl/quiet.pem".to_owned(),
            None,
        );
        core.observe(Reading {
            key: MetricKey::named(
                Subject::CertificateDaysRemaining,
                "/etc/ssl/quiet.pem".to_owned(),
            ),
            value: 90.0,
            at: at(0),
        });

        let state = |now| {
            core.watching(now, Duration::seconds(60))
                .into_iter()
                .find(|resource| resource.key.instance.as_deref() == Some("/etc/ssl/quiet.pem"))
                .expect("listed")
                .state
        };
        assert!(state(at(30)).is_observed(), "a fresh reading is fresh");
        assert_eq!(state(at(600)), Watching::Stale { last_read: at(0) });
    }

    #[test]
    fn a_reading_that_arrives_clears_the_failure_before_it() {
        // Otherwise a permission fixed at noon is still reported as broken at midnight, and the
        // page that exists to say what is wrong becomes the page nobody believes.
        let core = core();
        core.watch(
            Subject::CertificateDaysRemaining,
            "/etc/ssl/fixed.pem".to_owned(),
            None,
        );
        let key = MetricKey::named(
            Subject::CertificateDaysRemaining,
            "/etc/ssl/fixed.pem".to_owned(),
        );
        core.note_unreadable(&key, at(0));
        core.observe(Reading {
            key: key.clone(),
            value: 90.0,
            at: at(10),
        });

        let state = core
            .watching(at(20), Duration::seconds(60))
            .into_iter()
            .find(|resource| resource.key == key)
            .expect("listed")
            .state;
        assert!(state.is_observed(), "{state:?}");
    }

    #[test]
    fn two_certificates_in_trouble_are_two_findings_carrying_their_own_numbers() {
        // The defect this closes: everything downstream of the windows was keyed by subject alone,
        // so two certificates produced two windows, two findings, and one deviation — the second
        // overwrote the first, and one of the two findings cited the other certificate readings as
        // the evidence for itself.
        let core = core();
        core.watch(
            Subject::CertificateDaysRemaining,
            "/etc/ssl/soon.pem".to_owned(),
            None,
        );
        core.watch(
            Subject::CertificateDaysRemaining,
            "/etc/ssl/sooner.pem".to_owned(),
            None,
        );
        for tick in 0..24i32 {
            let drift = f64::from(tick) * 0.01;
            core.observe(Reading {
                key: MetricKey::named(
                    Subject::CertificateDaysRemaining,
                    "/etc/ssl/soon.pem".to_owned(),
                ),
                value: 12.0 - drift,
                at: at(i64::from(tick) * 60),
            });
            core.observe(Reading {
                key: MetricKey::named(
                    Subject::CertificateDaysRemaining,
                    "/etc/ssl/sooner.pem".to_owned(),
                ),
                value: 3.0 - drift,
                at: at(i64::from(tick) * 60),
            });
        }

        let found = core.insights(at(2000));
        let expiring: Vec<&SystemInsight> = found
            .iter()
            .filter(|insight| insight.finding == Finding::CertificateExpiring)
            .collect();
        assert_eq!(expiring.len(), 2, "two certificates, two findings");

        // Each names which one it is about...
        let about: Vec<Option<&str>> = expiring
            .iter()
            .map(|insight| {
                insight
                    .about
                    .as_ref()
                    .and_then(|key| key.instance.as_deref())
            })
            .collect();
        assert!(about.contains(&Some("/etc/ssl/soon.pem")), "{about:?}");
        assert!(about.contains(&Some("/etc/ssl/sooner.pem")), "{about:?}");

        // ...and each cites its own readings, not the other one numbers.
        for insight in &expiring {
            let its_own = insight.about.as_ref().expect("a named finding");
            assert!(
                insight
                    .because
                    .iter()
                    .all(|evidence| &evidence.key == its_own),
                "a finding about {} cited {:?}",
                its_own.label(),
                insight
                    .because
                    .iter()
                    .map(|evidence| evidence.key.label())
                    .collect::<Vec<_>>()
            );
        }

        // ...and the two are told apart by identity, not only by their text.
        assert_ne!(
            expiring[0].insight_id, expiring[1].insight_id,
            "two conditions on two different files shared one identity"
        );
    }

    #[test]
    fn one_ongoing_condition_keeps_one_identity_across_reads() {
        // A fresh identity per read meant two requests a second apart described one physically
        // identical situation with two different identities. Harmless while nothing referred to
        // them, and an architectural defect the moment a proposal cites one as its cause: the cause
        // it names would not exist by the time anybody looked.
        let core = core();
        core.watch(
            Subject::CertificateDaysRemaining,
            "/etc/ssl/steady.pem".to_owned(),
            None,
        );
        for tick in 0..24 {
            core.observe(Reading {
                key: MetricKey::named(
                    Subject::CertificateDaysRemaining,
                    "/etc/ssl/steady.pem".to_owned(),
                ),
                value: 4.0,
                at: at(tick * 60),
            });
        }

        let first = core.insights(at(2000));
        // A second read, at a different instant, of an unchanged host.
        let second = core.insights(at(2600));
        assert_eq!(first.len(), second.len());
        for (before, after) in first.iter().zip(second.iter()) {
            assert_eq!(
                before.insight_id,
                after.insight_id,
                "{} changed identity while nothing about it changed",
                before.finding.name()
            );
        }
    }

    #[test]
    fn a_condition_that_ends_and_returns_is_a_new_occurrence() {
        // The other half of the same property. Identity that never changed would merge two separate
        // episodes into one, and "since" would then describe a stretch of time the host was fine
        // for part of.
        let key = MetricKey::named(
            Subject::CertificateDaysRemaining,
            "/etc/ssl/renewed.pem".to_owned(),
        );
        let episode = |values: &[f64], from: i64| {
            let core = core();
            core.watch(
                Subject::CertificateDaysRemaining,
                "/etc/ssl/renewed.pem".to_owned(),
                None,
            );
            for (index, value) in (0i64..).zip(values.iter()) {
                core.observe(Reading {
                    key: key.clone(),
                    value: *value,
                    at: at(from + index * 60),
                });
            }
            core.insights(at(from + 4000))
                .into_iter()
                .find(|insight| insight.finding == Finding::CertificateExpiring)
                .expect("a finding")
        };

        let earlier = episode(&[4.0; 24], 0);
        let later = episode(&[4.0; 24], 100_000);
        assert_ne!(
            earlier.insight_id, later.insight_id,
            "two separate episodes were reported as one occurrence"
        );
    }

    #[test]
    fn a_declared_service_that_is_not_running_is_found() {
        // Distinct from the count of failed units, which says something is wrong and not what. A
        // service can be inactive without having failed.
        let core = core();
        core.watch(
            Subject::ServiceActive,
            "postgresql.service".to_owned(),
            None,
        );
        for tick in 0..20 {
            core.observe(Reading {
                key: MetricKey {
                    subject: Subject::ServiceActive,
                    instance: Some("postgresql.service".to_owned()),
                },
                value: 0.0,
                at: at(tick * 60),
            });
        }
        assert!(
            core.insights(at(2000))
                .iter()
                .any(|insight| insight.finding == Finding::ServiceInactive)
        );
    }

    #[test]
    fn a_backup_is_judged_against_the_operators_number_and_not_one_of_ours() {
        // Two backups on one host can honestly disagree about how stale is too stale. The same age
        // is a finding under one declaration and not under the other.
        let core = core();
        core.watch(
            Subject::BackupAgeDays,
            "/var/backups/strict".to_owned(),
            Some(Alarming::AtOrAbove(1.0)),
        );
        core.watch(
            Subject::BackupAgeDays,
            "/var/backups/relaxed".to_owned(),
            Some(Alarming::AtOrAbove(30.0)),
        );
        for name in ["/var/backups/strict", "/var/backups/relaxed"] {
            for tick in 0..20 {
                core.observe(Reading {
                    key: MetricKey {
                        subject: Subject::BackupAgeDays,
                        instance: Some(name.to_owned()),
                    },
                    value: 3.0,
                    at: at(tick * 60),
                });
            }
        }

        let stale = core
            .insights(at(2000))
            .into_iter()
            .filter(|insight| insight.finding == Finding::BackupStale)
            .count();
        assert_eq!(stale, 1, "both were judged against one number");
    }

    #[test]
    fn a_backup_nobody_set_a_policy_for_is_watched_and_never_judged() {
        // The honest state for something with no universal threshold and no declared one: watched,
        // and not reported against a number nobody chose.
        let core = core();
        core.watch(
            Subject::BackupAgeDays,
            "/var/backups/quiet".to_owned(),
            None,
        );
        for tick in 0..20 {
            core.observe(Reading {
                key: MetricKey {
                    subject: Subject::BackupAgeDays,
                    instance: Some("/var/backups/quiet".to_owned()),
                },
                value: 400.0,
                at: at(tick * 60),
            });
        }
        assert!(
            !core
                .insights(at(2000))
                .iter()
                .any(|insight| insight.finding == Finding::BackupStale)
        );
        assert!(
            core.latest()
                .iter()
                .any(|reading| reading.subject() == Subject::BackupAgeDays),
            "and it is still watched"
        );
    }

    #[test]
    fn a_certificate_with_months_left_is_not_a_finding() {
        // The control. Every assertion above passes on a detector that reports every watched
        // certificate.
        let core = core();
        core.watch(
            Subject::CertificateDaysRemaining,
            "/etc/ssl/a.pem".to_owned(),
            None,
        );
        for tick in 0..20 {
            core.observe(Reading {
                key: MetricKey {
                    subject: Subject::CertificateDaysRemaining,
                    instance: Some("/etc/ssl/a.pem".to_owned()),
                },
                value: 88.0,
                at: at(tick * 60),
            });
        }
        assert!(
            !core
                .insights(at(2000))
                .iter()
                .any(|insight| insight.finding == Finding::CertificateExpiring)
        );
    }

    #[test]
    fn a_certificate_nobody_declared_is_not_watched_because_a_reading_arrived() {
        // Windows begin by declaration. One that appeared because a probe reported something would
        // let the probe decide what this host cares about.
        let core = core();
        core.observe(Reading {
            key: MetricKey {
                subject: Subject::CertificateDaysRemaining,
                instance: Some("/etc/ssl/undeclared.pem".to_owned()),
            },
            value: 3.0,
            at: at(0),
        });
        assert!(
            !core
                .latest()
                .iter()
                .any(|reading| reading.subject() == Subject::CertificateDaysRemaining),
            "an undeclared certificate started a window"
        );
    }

    #[test]
    fn a_filesystem_out_of_inodes_is_found_while_every_byte_measure_reads_healthy() {
        // The blind spot the subject exists for. Forty percent of the bytes are used and nothing can
        // be created; without this the host would report that storage is fine.
        let core = core();
        history(&core, Subject::RootFilesystemUsed, 0.40, &[]);
        history(&core, Subject::RootFilesystemInodesUsed, 0.995, &[]);

        let insights = core.insights(at(300));
        assert!(
            insights
                .iter()
                .any(|insight| insight.finding == Finding::StorageExhaustion),
            "a full inode table produced no finding: {insights:?}"
        );
    }

    #[test]
    fn a_machine_out_of_file_descriptors_is_its_own_finding() {
        // Nothing is full and deleting things frees nothing, so folding it into storage would offer
        // the wrong remedy.
        let core = core();
        history(&core, Subject::OpenFileDescriptors, 0.97, &[]);

        let insights = core.insights(at(300));
        assert!(
            insights
                .iter()
                .any(|insight| insight.finding == Finding::FileDescriptorExhaustion),
            "{insights:?}"
        );
    }

    #[test]
    fn memory_pressure_with_swap_growing_is_stronger_than_memory_pressure_alone() {
        // The same story told twice. Corroboration is what separates a moderate conclusion from a
        // weak one, and it is derived from the readings rather than asserted.
        let alone = core();
        history(&alone, Subject::MemoryPressure, 2.0, &[85.0, 88.0, 90.0]);
        let weak = alone
            .insights(at(400))
            .into_iter()
            .find(|insight| insight.finding == Finding::MemoryPressure)
            .expect("pressure alone is still a finding");
        assert_eq!(weak.strength, EvidenceStrength::Weak);
        assert_eq!(weak.because.len(), 1);

        let together = core();
        history(&together, Subject::MemoryPressure, 2.0, &[85.0, 88.0, 90.0]);
        history(&together, Subject::SwapUsed, 0.02, &[0.4, 0.6, 0.8]);
        let moderate = together
            .insights(at(400))
            .into_iter()
            .find(|insight| insight.finding == Finding::MemoryPressure)
            .expect("a finding");
        assert_eq!(moderate.strength, EvidenceStrength::Moderate);
        assert_eq!(moderate.because.len(), 2, "the corroborator is not cited");
    }

    #[test]
    fn something_out_of_range_that_nothing_explains_is_still_reported() {
        // A detector that only reported what it had a name for would be silent exactly when a host
        // is doing something nobody anticipated.
        let core = core();
        history(&core, Subject::LoadAverage, 0.4, &[19.0, 21.0, 20.0]);

        let insights = core.insights(at(400));
        assert!(
            insights
                .iter()
                .any(|insight| insight.finding == Finding::UnexplainedDeviation),
            "{insights:?}"
        );
    }

    #[test]
    fn a_quiet_host_concludes_nothing_about_itself() {
        // The control. Every test above passes on a detector that always finds something.
        let core = core();
        for subject in [
            Subject::LoadAverage,
            Subject::MemoryPressure,
            Subject::IoPressure,
            Subject::CpuPressure,
        ] {
            history(&core, subject, 1.0, &[1.01, 0.99, 1.0]);
        }
        history(&core, Subject::RootFilesystemUsed, 0.41, &[0.41, 0.41]);
        history(&core, Subject::FailedUnits, 0.0, &[0.0, 0.0]);

        assert!(core.has_watched_enough());
        assert_eq!(core.insights(at(400)), Vec::new());
    }

    #[test]
    fn a_finding_says_since_when_from_the_readings() {
        let core = core();
        history(&core, Subject::MemoryPressure, 2.0, &[3.0, 85.0, 88.0]);
        let insight = core
            .insights(at(400))
            .into_iter()
            .find(|insight| insight.finding == Finding::MemoryPressure)
            .expect("a finding");
        assert_eq!(insight.since, at(250), "since was not read from the window");
        assert_eq!(insight.concluded_at, at(400));
    }

    #[test]
    fn a_subject_nothing_ever_reported_is_absent_rather_than_zero() {
        // A host without pressure accounting, or without swap, has nothing to say about them. A
        // surface showing 0.0 would be showing a perfectly calm machine where there is in fact no
        // measurement at all.
        let core = core();
        history(&core, Subject::LoadAverage, 0.4, &[]);

        let latest = core.latest();
        assert_eq!(latest.len(), 1);
        assert_eq!(latest[0].subject(), Subject::LoadAverage);
        assert!(
            !core
                .deviations()
                .iter()
                .any(|(key, _)| key.subject == Subject::SwapUsed)
        );
    }

    #[test]
    fn the_same_windows_always_conclude_the_same_things() {
        let core = core();
        history(&core, Subject::MemoryPressure, 2.0, &[85.0, 88.0, 90.0]);
        history(&core, Subject::SwapUsed, 0.02, &[0.4, 0.6, 0.8]);
        let first = core.insights(at(400));
        for _ in 0..8 {
            assert_eq!(core.insights(at(400)), first);
        }
    }
}
