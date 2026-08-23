// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Where something is heading, and when it gets there.
//!
//! The detector answers *is this unusual right now*. This answers the other question an operator
//! actually asks, which no amount of current-state monitoring reaches: **at this rate, when does it
//! become a problem?** A disk at 71% is not interesting. A disk at 71% that was at 62% yesterday is
//! the thing worth being told about, and the difference between them is a slope.
//!
//! ## Why the median of pairwise slopes rather than a least-squares fit
//!
//! Least squares minimises squared error, so one spike moves the line by the square of how far out
//! it was. Operational series are full of spikes that mean nothing — a backup ran, a log rotated, a
//! build filled `/tmp` and emptied it — and a fit that swung on each of them would produce a
//! confident date that changed every time somebody compiled something.
//!
//! The Theil–Sen estimator takes the median of the slopes between every pair of points. Half the
//! points have to move before it does, so it describes the trend rather than the last excursion. It
//! is also explainable: the answer is one of the observed pairwise slopes, and a person can find it.
//!
//! ## The horizon
//!
//! A linear trend is a description of what was observed. Projected far enough past the window, it
//! stops being a description and becomes an assumption about the future, and the failure mode is
//! confident nonsense — six hours of watching turned into a date three months out, stated with the
//! same wording as a date three hours out.
//!
//! Refusing to answer would be worse. *At this rate `/var` fills in three days* is exactly what the
//! operator needs even on a young window, and silence there loses them the disk. So the answer is
//! given and the reach is stated: a projection knows whether it is looking further ahead than it has
//! watched, and says so.

use cybou_protocol::telemetry::Alarming;
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

use crate::series::Series;

/// The smallest number of readings a slope will be estimated from.
///
/// Below this the pairwise medians are dominated by whichever two readings happened to be furthest
/// apart, and a "trend" from four points is a line through noise.
pub const SMALLEST_TRENDABLE_WINDOW: usize = 16;

/// How much of the window's own spread the trend has to explain before it is called a direction.
///
/// A fraction of the spread rather than an absolute, because the subjects are on wildly different
/// scales: a load average and a filesystem share have no common epsilon. And compared against the
/// movement *across the whole window* rather than against the per-second slope, because those are
/// different units — a slope is value-per-second and a spread is a value, and comparing them
/// directly makes the flatness threshold depend on how the clock is counted. That mistake reported a
/// steadily filling disk as flat, which is the one answer that loses the disk.
///
/// The spread it is measured against is a median absolute deviation and not a range. A range moves
/// with one outlier, so a robust slope compared against a non-robust yardstick is silenced by
/// exactly the spike it was chosen to survive — the same mistake as a least-squares fit, made one
/// line further down. Both of these were caught by the same test.
const FLAT_FRACTION_OF_SPREAD: f64 = 0.05;

/// Where a subject is heading.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Trend {
    /// Going up, by this much per second.
    Rising(f64),
    /// Going down, by this much per second.
    Falling(f64),
    /// Not moving in a way this window can distinguish from noise.
    Flat,
}

/// When a subject reaches a threshold, if it does.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Reaching {
    /// It is already there.
    Already,
    /// At the current rate it arrives after this long.
    AtThisRate {
        /// How long from the most recent reading.
        #[serde(with = "seconds")]
        after: Duration,
        /// Whether this looks further ahead than the window has watched.
        ///
        /// Stated rather than used to refuse. A young window projecting three days out is the most
        /// useful thing this module produces and the least certain, and a reader is entitled to
        /// both facts.
        beyond_what_was_watched: bool,
    },
    /// It is flat, or moving away from the threshold. Not a very large number: a different answer.
    NotAtThisRate,
    /// Too few readings to estimate a slope from.
    NotEnoughHistory {
        /// How many readings there are.
        have: usize,
        /// How many are needed.
        need: usize,
    },
}

/// One subject's direction, and when it becomes a problem.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Projection {
    /// Where it is heading.
    pub trend: Trend,
    /// The most recent reading.
    pub current: f64,
    /// The threshold this was projected against.
    pub threshold: f64,
    /// When it arrives.
    pub reaching: Reaching,
    /// How long the window has actually watched.
    #[serde(with = "seconds")]
    pub watched: Duration,
}

/// A duration on the wire, in whole seconds.
///
/// The bus carries a number rather than a struct, so a reader in another language does not need to
/// know how this one represents a span. Whole seconds because a projection measured finer than that
/// is already more precise than the claim it supports.
mod seconds {
    use serde::{Deserialize, Deserializer, Serializer};
    use time::Duration;

    pub fn serialize<S: Serializer>(value: &Duration, out: S) -> Result<S::Ok, S::Error> {
        out.serialize_i64(value.whole_seconds())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(input: D) -> Result<Duration, D::Error> {
        Ok(Duration::seconds(i64::deserialize(input)?))
    }
}

/// The median of the slopes between every pair of points.
///
/// `None` when there are too few points, or when every point shares one instant — a set of readings
/// taken at the same moment has no slope, and dividing by the zero time between them would produce
/// an infinity that later reads as an imminent arrival.
#[must_use]
pub fn theil_sen(points: &[(f64, f64)]) -> Option<f64> {
    if points.len() < SMALLEST_TRENDABLE_WINDOW {
        return None;
    }
    let mut slopes = Vec::new();
    for (index, (earlier_at, earlier)) in points.iter().enumerate() {
        for (later_at, later) in &points[index + 1..] {
            let elapsed = later_at - earlier_at;
            if elapsed.abs() > f64::EPSILON {
                slopes.push((later - earlier) / elapsed);
            }
        }
    }
    if slopes.is_empty() {
        return None;
    }
    slopes.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    let middle = slopes.len() / 2;
    Some(if slopes.len().is_multiple_of(2) {
        f64::midpoint(slopes[middle - 1], slopes[middle])
    } else {
        slopes[middle]
    })
}

/// Project one series against a threshold.
///
/// `now` is the instant the projection is made from, so a stale window does not report a date
/// measured from a reading taken an hour ago.
#[must_use]
pub fn project(series: &Series, alarming: Alarming, now: OffsetDateTime) -> Option<Projection> {
    let latest = series.latest()?;
    let oldest = series.sees_back_to()?;
    let watched = latest.at - oldest;

    let points: Vec<(f64, f64)> = series.timed_values(oldest);
    let current = latest.value;

    let Some(slope) = theil_sen(&points) else {
        return Some(Projection {
            trend: Trend::Flat,
            current,
            threshold: alarming.threshold(),
            reaching: Reaching::NotEnoughHistory {
                have: points.len(),
                need: SMALLEST_TRENDABLE_WINDOW,
            },
            watched,
        });
    };

    // What counts as movement is how much of this window's own spread the trend accounts for. A
    // steady ramp explains nearly all of it; noise around a constant explains almost none.
    let spread = robust_spread(&points);
    let across_the_window = slope * watched.as_seconds_f64();
    let explains_enough = (spread * FLAT_FRACTION_OF_SPREAD).max(f64::EPSILON);
    let trend = if across_the_window > explains_enough {
        Trend::Rising(slope)
    } else if across_the_window < -explains_enough {
        Trend::Falling(slope)
    } else {
        Trend::Flat
    };

    let reaching = if alarming.reached_by(current) {
        Reaching::Already
    } else {
        // Whether this is movement *toward* the problem, which depends on which side the problem is
        // on. A certificate losing a day a day is approaching; a filesystem losing a percent a day
        // is retreating, and the same slope means opposite things.
        let rate = match trend {
            Trend::Rising(rate) | Trend::Falling(rate) => rate,
            Trend::Flat => 0.0,
        };
        let seconds = (alarming.threshold() - current) / rate;
        // Flat, or heading away: not "in a very long time". A series moving away does not arrive at
        // all, and a large number would be read as a date.
        if alarming.approaches(rate) && seconds.is_finite() && seconds >= 0.0 {
            #[allow(
                clippy::cast_possible_truncation,
                reason = "a projection measured to the second is already more precise than the claim"
            )]
            let from_the_last_reading = Duration::seconds(seconds as i64);
            // Measured from now, not from the reading. A window that stopped being fed ten minutes
            // ago would otherwise keep reporting the same three hours, counting down from an instant
            // that is receding — and the closer the arrival, the larger the error as a share of what
            // is left.
            let stale_by = now - latest.at;
            let after = (from_the_last_reading - stale_by).max(Duration::ZERO);
            Reaching::AtThisRate {
                after,
                beyond_what_was_watched: after > watched,
            }
        } else {
            Reaching::NotAtThisRate
        }
    };

    Some(Projection {
        trend,
        current,
        threshold: alarming.threshold(),
        reaching,
        watched,
    })
}

/// How much the observed values vary, without being moved by one of them.
fn robust_spread(points: &[(f64, f64)]) -> f64 {
    let values: Vec<f64> = points.iter().map(|(_, value)| *value).collect();
    crate::baseline::median_absolute_deviation(&values).unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use cybou_protocol::telemetry::{Reading, Subject};

    use super::*;

    fn at(offset: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_787_000_000 + offset).expect("a fixed instant")
    }

    /// A window over one subject, filled from a function of the tick.
    fn window(count: i64, value: impl Fn(i64) -> f64) -> Series {
        let mut series = Series::new(Subject::RootFilesystemUsed, Duration::days(30), 10_000);
        for tick in 0..count {
            series.observe(Reading {
                subject: Subject::RootFilesystemUsed,
                value: value(tick),
                at: at(tick * 60),
            });
        }
        series
    }

    #[test]
    fn one_spike_does_not_move_the_date() {
        // The reason this is a median of pairwise slopes rather than a least-squares fit. A build
        // filled /tmp and emptied it; a fit that swung on that would produce a confident date that
        // changed every time somebody compiled something.
        let steady = window(60, |tick| {
            #[allow(clippy::cast_precision_loss, reason = "a test fixture of sixty ticks")]
            let tick = tick as f64;
            0.60 + tick * 0.0001
        });
        let spiked = window(60, |tick| {
            #[allow(clippy::cast_precision_loss, reason = "a test fixture of sixty ticks")]
            let float = tick as f64;
            let base = 0.60 + float * 0.0001;
            if tick == 30 { base + 0.30 } else { base }
        });

        let clean = project(&steady, Alarming::AtOrAbove(0.95), at(3600)).expect("a projection");
        let disturbed =
            project(&spiked, Alarming::AtOrAbove(0.95), at(3600)).expect("a projection");

        let (Reaching::AtThisRate { after: a, .. }, Reaching::AtThisRate { after: b, .. }) =
            (clean.reaching, disturbed.reaching)
        else {
            panic!("{clean:?} / {disturbed:?}");
        };
        let drift = (a - b).abs();
        assert!(
            drift < a / 10,
            "one spike moved the date by {drift:?} out of {a:?}"
        );
    }

    #[test]
    fn something_moving_away_does_not_arrive_in_a_very_long_time() {
        // "Never at this rate" and "in nine thousand years" are different answers, and only one of
        // them is not a date. A reader shown the second has been given a number to compare.
        let shrinking = window(60, |tick| {
            #[allow(clippy::cast_precision_loss, reason = "a test fixture of sixty ticks")]
            let tick = tick as f64;
            0.80 - tick * 0.0005
        });
        let projected =
            project(&shrinking, Alarming::AtOrAbove(0.95), at(3600)).expect("a projection");
        assert!(matches!(projected.trend, Trend::Falling(_)));
        assert_eq!(projected.reaching, Reaching::NotAtThisRate);
    }

    #[test]
    fn a_subject_whose_problem_is_a_low_value_is_projected_toward_its_floor() {
        // The case the old code could not express. Days remaining on a certificate falls toward
        // zero, and a projection that only watched the rising tail would report it as never
        // arriving — healthy right up to the hour it expires.
        let expiring = window(60, |tick| {
            #[allow(clippy::cast_precision_loss, reason = "a test fixture of sixty ticks")]
            let tick = tick as f64;
            30.0 - tick * 0.01
        });
        let projected = project(&expiring, Alarming::AtOrBelow(7.0), at(3600)).expect("projected");

        assert!(matches!(projected.trend, Trend::Falling(_)));
        let Reaching::AtThisRate { after, .. } = projected.reaching else {
            panic!("{projected:?}");
        };
        assert!(after > Duration::ZERO);
    }

    #[test]
    fn a_low_threshold_subject_moving_upward_does_not_arrive() {
        // The control for the direction. A certificate that was just renewed is going the right way,
        // and the same slope on a filling disk would be an arrival.
        let renewed = window(60, |tick| {
            #[allow(clippy::cast_precision_loss, reason = "a test fixture of sixty ticks")]
            let tick = tick as f64;
            10.0 + tick * 0.5
        });
        let projected = project(&renewed, Alarming::AtOrBelow(7.0), at(3600)).expect("projected");
        assert_eq!(projected.reaching, Reaching::NotAtThisRate);
    }

    #[test]
    fn something_already_below_a_low_threshold_says_so() {
        let expired = window(60, |_| 2.0);
        let projected = project(&expired, Alarming::AtOrBelow(7.0), at(3600)).expect("projected");
        assert_eq!(projected.reaching, Reaching::Already);
    }

    #[test]
    fn a_flat_series_is_flat_rather_than_very_slowly_rising() {
        let steady = window(60, |_| 0.42);
        let projected =
            project(&steady, Alarming::AtOrAbove(0.95), at(3600)).expect("a projection");
        assert_eq!(projected.trend, Trend::Flat);
        assert_eq!(projected.reaching, Reaching::NotAtThisRate);
    }

    #[test]
    fn a_projection_says_when_it_is_looking_further_ahead_than_it_has_watched() {
        // The most useful answer this module produces and the least certain. Refusing it would lose
        // the operator the disk; stating it without the reach would be a young window sounding like
        // an old one.
        let slow = window(60, |tick| {
            #[allow(clippy::cast_precision_loss, reason = "a test fixture of sixty ticks")]
            let tick = tick as f64;
            0.60 + tick * 0.00002
        });
        let projected = project(&slow, Alarming::AtOrAbove(0.95), at(3600)).expect("a projection");
        let Reaching::AtThisRate {
            after,
            beyond_what_was_watched,
        } = projected.reaching
        else {
            panic!("{projected:?}");
        };
        assert!(after > projected.watched);
        assert!(beyond_what_was_watched);
    }

    #[test]
    fn a_window_that_stopped_being_fed_counts_down_rather_than_repeating_itself() {
        // A projection measured from the last reading would report the same three hours for as long
        // as nobody looked, counting down from an instant that is receding — and the closer the
        // arrival, the larger that error is as a share of what is left.
        let filling = window(60, |tick| {
            #[allow(clippy::cast_precision_loss, reason = "a test fixture of sixty ticks")]
            let tick = tick as f64;
            0.60 + tick * 0.005
        });

        // Staled by less than the arrival is away, so the countdown is visible rather than clamped.
        let fresh = project(&filling, Alarming::AtOrAbove(0.95), at(3540)).expect("a projection");
        let stale =
            project(&filling, Alarming::AtOrAbove(0.95), at(3540 + 300)).expect("a projection");

        let (Reaching::AtThisRate { after: soon, .. }, Reaching::AtThisRate { after: sooner, .. }) =
            (fresh.reaching, stale.reaching)
        else {
            panic!("{fresh:?} / {stale:?}");
        };
        assert_eq!(soon - sooner, Duration::seconds(300));
    }

    #[test]
    fn a_window_stale_past_its_own_projection_says_now_rather_than_a_negative_time() {
        let filling = window(60, |tick| {
            #[allow(clippy::cast_precision_loss, reason = "a test fixture of sixty ticks")]
            let tick = tick as f64;
            0.60 + tick * 0.005
        });
        let forgotten =
            project(&filling, Alarming::AtOrAbove(0.95), at(3540 + 100_000)).expect("a projection");
        let Reaching::AtThisRate { after, .. } = forgotten.reaching else {
            panic!("{forgotten:?}");
        };
        assert_eq!(after, Duration::ZERO);
    }

    #[test]
    fn a_projection_within_what_was_watched_says_so_too() {
        // The control. Without it the flag above could be hard-coded true and every test would pass.
        let fast = window(60, |tick| {
            #[allow(clippy::cast_precision_loss, reason = "a test fixture of sixty ticks")]
            let tick = tick as f64;
            0.60 + tick * 0.005
        });
        let projected = project(&fast, Alarming::AtOrAbove(0.95), at(3600)).expect("a projection");
        let Reaching::AtThisRate {
            beyond_what_was_watched,
            ..
        } = projected.reaching
        else {
            panic!("{projected:?}");
        };
        assert!(!beyond_what_was_watched);
    }

    #[test]
    fn too_few_readings_produce_no_slope_rather_than_a_line_through_noise() {
        let barely = window(5, |tick| {
            #[allow(clippy::cast_precision_loss, reason = "a test fixture of five ticks")]
            let tick = tick as f64;
            0.60 + tick * 0.01
        });
        let projected = project(&barely, Alarming::AtOrAbove(0.95), at(600)).expect("a projection");
        assert!(matches!(
            projected.reaching,
            Reaching::NotEnoughHistory { .. }
        ));
    }

    #[test]
    fn something_already_past_the_threshold_says_so_rather_than_projecting_to_it() {
        let full = window(60, |_| 0.97);
        let projected = project(&full, Alarming::AtOrAbove(0.95), at(3600)).expect("a projection");
        assert_eq!(projected.reaching, Reaching::Already);
    }

    #[test]
    fn readings_that_share_one_instant_have_no_slope() {
        // Dividing by the zero time between them would produce an infinity, which later reads as an
        // imminent arrival — the most alarming possible answer from the least informative data.
        let same_instant: Vec<(f64, f64)> = (0..40)
            .map(|index| {
                #[allow(clippy::cast_precision_loss, reason = "a test fixture")]
                let value = f64::from(index) * 0.01;
                (0.0, value)
            })
            .collect();
        assert_eq!(theil_sen(&same_instant), None);
    }

    #[test]
    fn the_slope_is_one_a_person_can_check() {
        // A clean ramp of 0.01 per minute is 0.01/60 per second, and the estimator returns exactly
        // that rather than something near it.
        let ramp: Vec<(f64, f64)> = (0..40)
            .map(|index| {
                let seconds = f64::from(index) * 60.0;
                (seconds, 0.10 + f64::from(index) * 0.01)
            })
            .collect();
        let slope = theil_sen(&ramp).expect("a slope");
        assert!((slope - 0.01 / 60.0).abs() < 1e-12, "{slope}");
    }

    #[test]
    fn an_empty_window_has_nothing_to_project() {
        let empty = Series::new(Subject::RootFilesystemUsed, Duration::hours(1), 100);
        assert!(project(&empty, Alarming::AtOrAbove(0.95), at(0)).is_none());
    }
}
