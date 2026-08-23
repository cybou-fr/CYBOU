// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Watching a host, and deciding when what it is doing is worth saying something about.

use std::collections::BTreeMap;
use std::sync::RwLock;

use cybou_protocol::telemetry::{
    ALL_SUBJECTS, Deviation, EvidenceStrength, Finding, Reading, Subject, SystemInsight,
};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::baseline::{SMALLEST_JUDGEABLE_WINDOW, deviation};
use crate::series::Series;

/// How far from ordinary a reading has to be before it is worth a second look.
///
/// Six spreads is deliberately far. A monitoring system that speaks at three is a monitoring system
/// people mute, and a muted detector detects nothing — the failure mode of an alerting system is
/// almost never that it missed something, it is that everybody stopped reading it.
const NOTEWORTHY_SPREADS: f64 = 6.0;

/// How full a filesystem has to be before fullness is the story regardless of statistics.
///
/// Some things do not need a baseline. A disk at 95% is a problem on a host where it has been at 95%
/// for a month, and a purely statistical detector would say nothing precisely because it is normal
/// here. Categorical facts and statistical deviations are different evidence and both are kept.
///
/// Read from the subject rather than held here. The detector and the projection used to keep their
/// own copies, which is how a system comes to report that a disk is fine and that it reaches trouble
/// in three days — two true statements about two different numbers.
fn alarming_for(subject: Subject) -> f64 {
    subject.alarming().unwrap_or(f64::MAX)
}

/// What the telemetry organ holds and concludes.
pub struct TelemetryCore {
    windows: RwLock<BTreeMap<Subject, Series>>,
    span: Duration,
    capacity: usize,
}

impl TelemetryCore {
    /// Watch every subject over a window of this span, holding at most `capacity` readings each.
    #[must_use]
    pub fn new(span: Duration, capacity: usize) -> Self {
        let windows = ALL_SUBJECTS
            .iter()
            .map(|subject| (*subject, Series::new(*subject, span, capacity)))
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

    /// Record one observation.
    pub fn observe(&self, reading: Reading) {
        if let Ok(mut windows) = self.windows.write()
            && let Some(series) = windows.get_mut(&reading.subject)
        {
            series.observe(reading);
        }
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
    pub fn deviations(&self) -> Vec<(Subject, Deviation)> {
        let Ok(windows) = self.windows.read() else {
            return Vec::new();
        };
        windows
            .values()
            .filter_map(|series| {
                let latest = series.latest()?;
                Some((series.subject(), deviation(&series.values(), latest.value)?))
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
    pub fn projections(&self, now: OffsetDateTime) -> Vec<(Subject, crate::trend::Projection)> {
        let Ok(windows) = self.windows.read() else {
            return Vec::new();
        };
        windows
            .values()
            .filter_map(|series| {
                let threshold = series.subject().alarming()?;
                Some((
                    series.subject(),
                    crate::trend::project(series, threshold, now)?,
                ))
            })
            .collect()
    }

    /// What this host currently concludes about itself.
    ///
    /// Every conclusion carries the readings that produced it and is a hypothesis, never a fact.
    /// Findings are returned in a fixed order so two runs over the same windows compare.
    #[must_use]
    pub fn insights(
        &self,
        now: OffsetDateTime,
        id: impl Fn(Finding) -> Uuid,
    ) -> Vec<SystemInsight> {
        let Ok(windows) = self.windows.read() else {
            return Vec::new();
        };
        let deviations: BTreeMap<Subject, Deviation> = windows
            .values()
            .filter_map(|series| {
                let latest = series.latest()?;
                Some((series.subject(), deviation(&series.values(), latest.value)?))
            })
            .collect();

        let mut found = Vec::new();
        let mut explained: Vec<Subject> = Vec::new();

        categorical(&windows, &deviations, now, &id, &mut found, &mut explained);
        pressures(&windows, &deviations, now, &id, &mut found, &mut explained);
        unexplained(&windows, &deviations, now, &id, &mut found, &explained);
        found
    }
}

/// The findings that do not need a baseline.
///
/// Some things are a problem on a host where they have always been the case, and a purely
/// statistical detector says nothing about them precisely because they are normal here.
fn categorical(
    windows: &BTreeMap<Subject, Series>,
    deviations: &BTreeMap<Subject, Deviation>,
    now: OffsetDateTime,
    id: &impl Fn(Finding) -> Uuid,
    found: &mut Vec<SystemInsight>,
    explained: &mut Vec<Subject>,
) {
    // Categorical first. Some things do not need a baseline: a filesystem at 95% is a problem on
    // a host where it has been at 95% for a month, and a purely statistical detector would say
    // nothing precisely because it is normal here.
    if let Some(series) = windows.get(&Subject::RootFilesystemUsed)
        && let Some(latest) = series.latest()
        && latest.value >= alarming_for(Subject::RootFilesystemUsed)
    {
        explained.push(Subject::RootFilesystemUsed);
        found.push(SystemInsight {
            insight_id: id(Finding::StorageExhaustion),
            finding: Finding::StorageExhaustion,
            because: evidence(deviations, &[Subject::RootFilesystemUsed]),
            strength: EvidenceStrength::Strong,
            concluded_at: now,
            since: series
                .continuously_since(alarming_for(Subject::RootFilesystemUsed))
                .unwrap_or(latest.at),
        });
    }

    if let Some(series) = windows.get(&Subject::FailedUnits)
        && let Some(latest) = series.latest()
        && latest.value >= 1.0
    {
        explained.push(Subject::FailedUnits);
        found.push(SystemInsight {
            insight_id: id(Finding::ServiceFailure),
            finding: Finding::ServiceFailure,
            because: evidence(deviations, &[Subject::FailedUnits]),
            strength: EvidenceStrength::Strong,
            concluded_at: now,
            since: series.continuously_since(1.0).unwrap_or(latest.at),
        });
    }
}

/// The findings that are a matter of degree, each with the subject that corroborates it.
///
/// Memory pressure alone is weak; memory pressure with swap growing is the same story told twice,
/// which is what makes it stronger.
fn pressures(
    windows: &BTreeMap<Subject, Series>,
    deviations: &BTreeMap<Subject, Deviation>,
    now: OffsetDateTime,
    id: &impl Fn(Finding) -> Uuid,
    found: &mut Vec<SystemInsight>,
    explained: &mut Vec<Subject>,
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
        let Some(series) = windows.get(&subject) else {
            continue;
        };
        let Some(latest) = series.latest() else {
            continue;
        };
        let unusual = deviations
            .get(&subject)
            .is_some_and(|found| found.spreads_away >= NOTEWORTHY_SPREADS);
        if !unusual && latest.value < alarming_for(subject) {
            continue;
        }
        let corroborated = corroborator.is_some_and(|other| {
            deviations
                .get(&other)
                .is_some_and(|found| found.spreads_away >= NOTEWORTHY_SPREADS)
        });
        explained.push(subject);
        let mut cited = vec![subject];
        if corroborated && let Some(other) = corroborator {
            explained.push(other);
            cited.push(other);
        }
        found.push(SystemInsight {
            insight_id: id(finding),
            finding,
            because: evidence(deviations, &cited),
            strength: if corroborated {
                EvidenceStrength::Moderate
            } else {
                EvidenceStrength::Weak
            },
            concluded_at: now,
            since: series
                .continuously_since(alarming_for(subject))
                .unwrap_or(latest.at),
        });
    }
}

/// Whatever is still out of range and unaccounted for.
///
/// A detector that only reported what it had a name for would be silent exactly when a host is
/// doing something nobody anticipated, which is the case an operator most wants to hear about.
fn unexplained(
    windows: &BTreeMap<Subject, Series>,
    deviations: &BTreeMap<Subject, Deviation>,
    now: OffsetDateTime,
    id: &impl Fn(Finding) -> Uuid,
    found: &mut Vec<SystemInsight>,
    explained: &[Subject],
) {
    for (subject, found_deviation) in deviations {
        if explained.contains(subject) || found_deviation.spreads_away < NOTEWORTHY_SPREADS {
            continue;
        }
        let since = windows
            .get(subject)
            .and_then(Series::latest)
            .map_or(now, |latest| latest.at);
        found.push(SystemInsight {
            insight_id: id(Finding::UnexplainedDeviation),
            finding: Finding::UnexplainedDeviation,
            because: evidence(deviations, &[*subject]),
            strength: EvidenceStrength::Weak,
            concluded_at: now,
            since,
        });
    }
}

/// The deviations behind a finding, in the order the finding cited them.
fn evidence(
    deviations: &BTreeMap<Subject, Deviation>,
    cited: &[Subject],
) -> Vec<(Subject, Deviation)> {
    cited
        .iter()
        .filter_map(|subject| deviations.get(subject).map(|found| (*subject, *found)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(offset: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_787_000_000 + offset).expect("a fixed instant")
    }

    fn ids() -> impl Fn(Finding) -> Uuid {
        |finding| Uuid::from_u128(finding.name().len() as u128)
    }

    fn core() -> TelemetryCore {
        TelemetryCore::new(Duration::hours(1), 240)
    }

    /// Feed a quiet history, then whatever comes after it.
    fn history(core: &TelemetryCore, subject: Subject, quiet: f64, then: &[f64]) {
        for index in 0..24 {
            core.observe(Reading {
                subject,
                value: quiet + f64::from(u8::try_from(index % 3).unwrap_or(0)) * 0.01,
                at: at(index * 10),
            });
        }
        for (index, value) in then.iter().enumerate() {
            #[allow(clippy::cast_possible_wrap, reason = "a handful of test readings")]
            let offset = 240 + (index as i64) * 10;
            core.observe(Reading {
                subject,
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
                subject: Subject::LoadAverage,
                value: 0.4,
                at: at(index),
            });
        }
        assert!(!core.has_watched_enough());
        assert!(core.insights(at(100), ids()).is_empty());
    }

    #[test]
    fn a_full_filesystem_is_a_finding_even_where_it_has_always_been_full() {
        // Some things do not need a baseline. A purely statistical detector would say nothing here
        // precisely because 96% is perfectly normal for this host.
        let core = core();
        history(&core, Subject::RootFilesystemUsed, 0.96, &[]);

        let insights = core.insights(at(300), ids());
        let storage = insights
            .iter()
            .find(|insight| insight.finding == Finding::StorageExhaustion)
            .expect("a full disk is a finding regardless of statistics");
        assert_eq!(storage.strength, EvidenceStrength::Strong);
        assert!(!storage.because.is_empty(), "the finding cites nothing");
    }

    #[test]
    fn memory_pressure_with_swap_growing_is_stronger_than_memory_pressure_alone() {
        // The same story told twice. Corroboration is what separates a moderate conclusion from a
        // weak one, and it is derived from the readings rather than asserted.
        let alone = core();
        history(&alone, Subject::MemoryPressure, 2.0, &[85.0, 88.0, 90.0]);
        let weak = alone
            .insights(at(400), ids())
            .into_iter()
            .find(|insight| insight.finding == Finding::MemoryPressure)
            .expect("pressure alone is still a finding");
        assert_eq!(weak.strength, EvidenceStrength::Weak);
        assert_eq!(weak.because.len(), 1);

        let together = core();
        history(&together, Subject::MemoryPressure, 2.0, &[85.0, 88.0, 90.0]);
        history(&together, Subject::SwapUsed, 0.02, &[0.4, 0.6, 0.8]);
        let moderate = together
            .insights(at(400), ids())
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

        let insights = core.insights(at(400), ids());
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
        assert_eq!(core.insights(at(400), ids()), Vec::new());
    }

    #[test]
    fn a_finding_says_since_when_from_the_readings() {
        let core = core();
        history(&core, Subject::MemoryPressure, 2.0, &[3.0, 85.0, 88.0]);
        let insight = core
            .insights(at(400), ids())
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
        assert_eq!(latest[0].subject, Subject::LoadAverage);
        assert!(
            !core
                .deviations()
                .iter()
                .any(|(subject, _)| *subject == Subject::SwapUsed)
        );
    }

    #[test]
    fn the_same_windows_always_conclude_the_same_things() {
        let core = core();
        history(&core, Subject::MemoryPressure, 2.0, &[85.0, 88.0, 90.0]);
        history(&core, Subject::SwapUsed, 0.02, &[0.4, 0.6, 0.8]);
        let first = core.insights(at(400), ids());
        for _ in 0..8 {
            assert_eq!(core.insights(at(400), ids()), first);
        }
    }
}
