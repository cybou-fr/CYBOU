// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! A bounded window of what a host has been doing.
//!
//! Bounded two ways, and both are needed for the same reason the dialogue memory needs both. A count
//! alone means a host sampled every second remembers a different span than one sampled every
//! minute, so the same detector sees a different amount of history depending on a configuration
//! nobody thought of as history. A duration alone means a host that sampled ten thousand times in a
//! burst holds ten thousand readings, and the memory bound this window exists to provide is gone.
//!
//! Nothing here can be written anywhere. The window holds `Reading`s, and a `Reading` has no path
//! into the Journal — the boundary ADR-0041 S7 asks for is that a reading is transient by
//! construction, and this is the construction.

use std::collections::VecDeque;

use cybou_protocol::telemetry::{Alarming, Reading, Subject};
use time::{Duration, OffsetDateTime};

/// One subject's recent history.
#[derive(Clone, Debug)]
pub struct Series {
    subject: Subject,
    /// Which one, for a subject about a named thing.
    instance: Option<String>,
    span: Duration,
    capacity: usize,
    readings: VecDeque<Reading>,
}

impl Series {
    /// A window over one subject, holding at most `capacity` readings and at most `span` of time.
    #[must_use]
    pub fn new(subject: Subject, span: Duration, capacity: usize) -> Self {
        Self::named(subject, None, span, capacity)
    }

    /// A window over one named thing.
    #[must_use]
    pub fn named(
        subject: Subject,
        instance: Option<String>,
        span: Duration,
        capacity: usize,
    ) -> Self {
        Self {
            subject,
            instance,
            span,
            capacity: capacity.max(1),
            readings: VecDeque::new(),
        }
    }

    /// Which one this window is about, for a named subject.
    #[must_use]
    pub fn instance(&self) -> Option<&str> {
        self.instance.as_deref()
    }

    /// How this window is named to a person.
    #[must_use]
    pub fn label(&self) -> String {
        match &self.instance {
            Some(name) => format!("{} ({name})", self.subject.name()),
            None => self.subject.name().to_owned(),
        }
    }

    /// What this window is about.
    #[must_use]
    pub const fn subject(&self) -> Subject {
        self.subject
    }

    /// How many readings it currently holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.readings.len()
    }

    /// Whether it holds nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.readings.is_empty()
    }

    /// Record one observation, dropping whatever has fallen outside either bound.
    ///
    /// A reading for the wrong subject is refused rather than stored: a window that accepted
    /// anything would silently mix two things a detector is about to compare against one baseline.
    pub fn observe(&mut self, reading: Reading) -> bool {
        // Both halves of the key. Two certificates in one window would produce a baseline for a
        // thing that does not exist, and a detector comparing one against the other's ordinary.
        if reading.subject != self.subject || reading.instance != self.instance {
            return false;
        }
        let at = reading.at;
        self.readings.push_back(reading);
        self.expire(at);
        true
    }

    /// Drop what has fallen outside either bound.
    fn expire(&mut self, now: OffsetDateTime) {
        while self.readings.len() > self.capacity {
            self.readings.pop_front();
        }
        while let Some(oldest) = self.readings.front() {
            if now - oldest.at > self.span {
                self.readings.pop_front();
            } else {
                break;
            }
        }
    }

    /// Every value held, oldest first.
    #[must_use]
    pub fn values(&self) -> Vec<f64> {
        self.readings.iter().map(|reading| reading.value).collect()
    }

    /// Every reading as seconds since `origin` and its value, oldest first.
    ///
    /// The shape a slope is estimated from. Seconds rather than instants because the estimator
    /// divides by elapsed time, and `origin` rather than the epoch because the difference of two
    /// large timestamps loses precision exactly where the slope is small — which is the case that
    /// matters, since a fast-moving subject needs no projection to notice.
    #[must_use]
    pub fn timed_values(&self, origin: OffsetDateTime) -> Vec<(f64, f64)> {
        self.readings
            .iter()
            .map(|reading| ((reading.at - origin).as_seconds_f64(), reading.value))
            .collect()
    }

    /// The most recent reading, if there is one.
    #[must_use]
    pub fn latest(&self) -> Option<Reading> {
        self.readings.back().cloned()
    }

    /// When the oldest held reading was taken.
    ///
    /// How far back the window can actually see, which is not the same as how far back it was
    /// configured to see. A detector that reported *since* from the configured span would claim to
    /// have watched something for an hour after four minutes of uptime.
    #[must_use]
    pub fn sees_back_to(&self) -> Option<OffsetDateTime> {
        self.readings.front().map(|reading| reading.at)
    }

    /// The earliest instant at which the value was continuously at or beyond `threshold`.
    ///
    /// Answers *since when* for a finding, from the readings rather than from a guess. Walks
    /// backwards from the most recent reading and stops at the first one below the threshold, so an
    /// episode that recovered and started again reports the current episode and not both.
    #[must_use]
    pub fn continuously_since(&self, alarming: Alarming) -> Option<OffsetDateTime> {
        let mut since = None;
        for reading in self.readings.iter().rev() {
            if alarming.reached_by(reading.value) {
                since = Some(reading.at);
            } else {
                break;
            }
        }
        since
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(offset: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_787_000_000 + offset).expect("a fixed instant")
    }

    fn reading(subject: Subject, value: f64, offset: i64) -> Reading {
        Reading {
            subject,
            instance: None,
            value,
            at: at(offset),
        }
    }

    fn named_reading(subject: Subject, name: &str, value: f64, offset: i64) -> Reading {
        Reading {
            subject,
            instance: Some(name.to_owned()),
            value,
            at: at(offset),
        }
    }

    fn window() -> Series {
        Series::new(Subject::MemoryPressure, Duration::minutes(10), 64)
    }

    #[test]
    fn a_burst_cannot_outgrow_the_memory_bound() {
        // The failure a duration bound alone allows: ten thousand readings inside one minute is
        // within any time window and is exactly the memory the window exists to bound.
        let mut series = window();
        for index in 0..10_000 {
            series.observe(reading(Subject::MemoryPressure, 1.0, index % 60));
        }
        assert_eq!(series.len(), 64);
    }

    #[test]
    fn a_slow_sampler_does_not_remember_further_back_than_it_was_told_to() {
        // The failure a count bound alone allows: sixty-four readings a minute apart is an hour of
        // history in a window configured for ten minutes, and the detector silently sees six times
        // what anyone intended.
        let mut series = window();
        for minute in 0..64 {
            series.observe(reading(Subject::MemoryPressure, 1.0, minute * 60));
        }
        assert!(series.len() <= 11, "held {} readings", series.len());
        let oldest = series.sees_back_to().expect("something is held");
        assert!(at(63 * 60) - oldest <= Duration::minutes(10));
    }

    #[test]
    fn a_reading_for_another_subject_is_refused_rather_than_mixed_in() {
        // A window that accepted anything would put two different things under one baseline, and
        // the detector comparing against it would be comparing memory against disk.
        let mut series = window();
        assert!(series.observe(reading(Subject::MemoryPressure, 1.0, 0)));
        assert!(!series.observe(reading(Subject::IoPressure, 90.0, 1)));
        assert_eq!(series.len(), 1);
    }

    #[test]
    fn two_named_things_of_the_same_kind_do_not_share_a_window() {
        // Two certificates in one window would produce a baseline for a thing that does not exist,
        // and one would be judged against the other's notion of ordinary.
        let mut series = Series::named(
            Subject::CertificateDaysRemaining,
            Some("/etc/ssl/a.pem".to_owned()),
            Duration::minutes(10),
            64,
        );
        assert!(series.observe(named_reading(
            Subject::CertificateDaysRemaining,
            "/etc/ssl/a.pem",
            30.0,
            0
        )));
        assert!(!series.observe(named_reading(
            Subject::CertificateDaysRemaining,
            "/etc/ssl/b.pem",
            2.0,
            1
        )));
        assert_eq!(series.len(), 1);
        assert_eq!(series.instance(), Some("/etc/ssl/a.pem"));
        assert_eq!(
            series.label(),
            "certificate.days.remaining (/etc/ssl/a.pem)"
        );
    }

    #[test]
    fn a_universal_reading_does_not_land_in_a_named_window() {
        let mut series = Series::named(
            Subject::CertificateDaysRemaining,
            Some("/etc/ssl/a.pem".to_owned()),
            Duration::minutes(10),
            64,
        );
        assert!(!series.observe(reading(Subject::CertificateDaysRemaining, 30.0, 0)));
    }

    #[test]
    fn how_far_back_it_can_see_is_what_it_holds_and_not_what_it_was_configured_for() {
        // A detector reporting `since` from the configured span would claim to have watched
        // something for ten minutes after four readings of uptime.
        let mut series = window();
        assert_eq!(series.sees_back_to(), None);
        series.observe(reading(Subject::MemoryPressure, 1.0, 0));
        series.observe(reading(Subject::MemoryPressure, 1.0, 30));
        assert_eq!(series.sees_back_to(), Some(at(0)));
    }

    #[test]
    fn since_when_is_read_from_the_readings_and_covers_the_current_episode_only() {
        // An episode that recovered and started again is one episode now, not one long one. A
        // detector reporting the earlier start would tell an operator the problem has been going on
        // for an hour when it came back two minutes ago.
        let mut series = window();
        for (offset, value) in [(0, 90.0), (30, 90.0), (60, 2.0), (90, 80.0), (120, 85.0)] {
            series.observe(reading(Subject::MemoryPressure, value, offset));
        }
        assert_eq!(
            series.continuously_since(Alarming::AtOrAbove(50.0)),
            Some(at(90))
        );
    }

    #[test]
    fn a_subject_whose_problem_is_a_low_value_reports_since_from_the_low_side() {
        // The same episode logic, watching the other tail. A window that only ever compared upward
        // would report a certificate as never having entered trouble.
        let mut series = Series::new(Subject::LoadAverage, Duration::minutes(10), 64);
        for (offset, value) in [(0, 40.0), (30, 30.0), (60, 5.0), (90, 3.0)] {
            series.observe(reading(Subject::LoadAverage, value, offset));
        }
        assert_eq!(
            series.continuously_since(Alarming::AtOrBelow(7.0)),
            Some(at(60))
        );
    }

    #[test]
    fn nothing_beyond_the_threshold_has_no_since() {
        let mut series = window();
        series.observe(reading(Subject::MemoryPressure, 1.0, 0));
        assert_eq!(series.continuously_since(Alarming::AtOrAbove(50.0)), None);
    }

    #[test]
    fn an_empty_window_answers_nothing_rather_than_zero() {
        let series = window();
        assert!(series.is_empty());
        assert!(series.latest().is_none());
        assert!(series.values().is_empty());
        assert_eq!(series.continuously_since(Alarming::AtOrAbove(0.0)), None);
    }
}
