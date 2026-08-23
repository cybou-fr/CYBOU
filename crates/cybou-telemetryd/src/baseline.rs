// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! What is ordinary for *this* host, and how far something sits from it.
//!
//! Statistics rather than a learned model, and not as a placeholder for one. Four reasons, and
//! only the last is about effort:
//!
//! - **It adapts to the machine.** A host that idles at 45% memory and one that idles at 8% have
//!   different notions of unusual, and a model trained on a corpus has the corpus's notion.
//! - **It is explainable by looking.** *The median is 0.62, the spread is 0.01, this is 0.94* is
//!   something a person can check. A score from a network is not, and this system has spent its
//!   whole design refusing conclusions that cannot show their work.
//! - **It works on one vCPU.** ADR-0041 S5: no cognitive function may need an accelerator.
//! - It needs no corpus, and there is no corpus.
//!
//! ## Why median and MAD rather than mean and standard deviation
//!
//! Because the thing being detected contaminates the thing detecting it. A host that has been
//! swapping for ten minutes has a mean and a standard deviation *shaped by the swapping*: the mean
//! rises toward the fault and the deviation widens, so the fault makes itself look ordinary. This
//! is the failure mode that makes naive threshold-on-sigma monitors go quiet exactly as a problem
//! settles in.
//!
//! A median moves only when half the window has moved, and a median absolute deviation is not
//! widened by a tail at all. The detector stays sensitive while the episode is a minority of the
//! window, which is the whole period during which anybody can still act.

use cybou_protocol::telemetry::Deviation;

/// The scale factor that puts a median absolute deviation on the same footing as a standard
/// deviation for normally distributed data.
///
/// Included so that "spreads away" means roughly what a reader who thinks in sigmas expects. It is
/// a convenience of interpretation and not a claim that anything here is normally distributed —
/// load averages and pressure readings are emphatically not.
const MAD_TO_SIGMA: f64 = 1.4826;

/// The smallest spread that will be believed.
///
/// A host whose memory pressure sat at exactly zero for an hour has a median absolute deviation of
/// zero, and every subsequent reading is then infinitely many spreads from ordinary. That is
/// arithmetic rather than insight: the first flicker of activity on a perfectly quiet host would be
/// reported as an extreme anomaly. The floor makes a quiet host require a real movement before
/// anything is said about it.
const SMALLEST_BELIEVABLE_SPREAD: f64 = 1e-6;

/// The smallest window that will be judged at all.
///
/// Four readings can be four samples of a rising ramp, and their median says nothing about what is
/// ordinary. A detector that answered anyway would be at its most confident when it knew least,
/// which is the shape of every bad monitoring alert anybody has been woken by.
pub const SMALLEST_JUDGEABLE_WINDOW: usize = 12;

/// The middle value of a window.
///
/// Returns `None` for an empty window rather than zero. A host with no readings has no ordinary,
/// and answering zero would make every subsequent reading enormous.
#[must_use]
pub fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted: Vec<f64> = values.to_vec();
    sorted.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    let middle = sorted.len() / 2;
    if sorted.len().is_multiple_of(2) {
        Some(f64::midpoint(sorted[middle - 1], sorted[middle]))
    } else {
        Some(sorted[middle])
    }
}

/// How much this host ordinarily varies, as a median absolute deviation.
#[must_use]
pub fn median_absolute_deviation(values: &[f64]) -> Option<f64> {
    let centre = median(values)?;
    let spreads: Vec<f64> = values.iter().map(|value| (value - centre).abs()).collect();
    median(&spreads)
}

/// Where one observation sits relative to what is ordinary for this host.
///
/// Returns `None` when the window is too short to have an opinion. Saying nothing is the correct
/// answer to *is this unusual* when there is not yet a notion of usual, and it is the answer a
/// detector is least inclined to give.
#[must_use]
pub fn deviation(values: &[f64], observed: f64) -> Option<Deviation> {
    if values.len() < SMALLEST_JUDGEABLE_WINDOW {
        return None;
    }
    let ordinary = median(values)?;
    let spread = median_absolute_deviation(values)?.max(SMALLEST_BELIEVABLE_SPREAD);
    Some(Deviation {
        ordinary,
        spread,
        observed,
        spreads_away: (observed - ordinary) / (spread * MAD_TO_SIGMA),
    })
}

/// An exponentially weighted moving average, for a value that should follow recent history closely.
///
/// Kept beside the robust statistics rather than instead of them. It is the right tool for tracking
/// where something is heading and the wrong one for deciding whether something is unusual, for
/// exactly the reason above: it follows the fault.
#[must_use]
pub fn exponentially_weighted(values: &[f64], smoothing: f64) -> Option<f64> {
    let smoothing = smoothing.clamp(0.0, 1.0);
    let mut iter = values.iter();
    let mut average = *iter.next()?;
    for value in iter {
        average = smoothing.mul_add(*value, (1.0 - smoothing) * average);
    }
    Some(average)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A quiet host: memory pressure hovering around 2, with ordinary jitter.
    fn quiet() -> Vec<f64> {
        vec![
            2.0, 2.4, 1.8, 2.1, 2.3, 1.9, 2.2, 2.0, 2.5, 1.7, 2.1, 2.2, 2.0, 1.9, 2.3, 2.1,
        ]
    }

    #[test]
    fn a_fault_that_has_been_running_for_a_while_does_not_make_itself_look_ordinary() {
        // The reason median and MAD are here rather than mean and standard deviation, as an
        // executable comparison. A third of the window is a fault; a sigma-based detector has
        // already widened enough to shrug at it, and a MAD-based one has not.
        let mut window = quiet();
        window.extend([90.0, 92.0, 88.0, 91.0, 89.0, 93.0]);
        let observed = 91.0;

        #[allow(
            clippy::cast_precision_loss,
            reason = "a window is tens of readings; the count is exact in f64"
        )]
        let count = window.len() as f64;
        let mean = window.iter().sum::<f64>() / count;
        let variance = window
            .iter()
            .map(|value| (value - mean).powi(2))
            .sum::<f64>()
            / count;
        let sigmas_away = (observed - mean) / variance.sqrt();

        let robust = deviation(&window, observed).expect("the window is long enough");

        assert!(
            sigmas_away < 2.0,
            "a sigma detector already considers the fault unremarkable: {sigmas_away:.2}"
        );
        assert!(
            robust.spreads_away > 50.0,
            "the robust detector lost sensitivity too: {:.2}",
            robust.spreads_away
        );
    }

    #[test]
    fn a_window_too_short_to_have_an_opinion_says_nothing() {
        // The answer a detector is least inclined to give, and the correct one: four readings of a
        // rising ramp have a median that says nothing about what is ordinary.
        let ramp = vec![1.0, 4.0, 9.0, 16.0];
        assert!(deviation(&ramp, 25.0).is_none());
        assert!(deviation(&[], 1.0).is_none());
    }

    #[test]
    fn a_perfectly_quiet_host_does_not_report_its_first_flicker_as_an_extreme() {
        // A window of identical values has a MAD of zero, and without a floor every subsequent
        // reading is infinitely many spreads away. That is arithmetic, not insight.
        let flat = vec![0.0; 32];
        let flicker = deviation(&flat, 0.4).expect("long enough");
        assert!(flicker.spreads_away.is_finite());
        assert!(flicker.spread >= SMALLEST_BELIEVABLE_SPREAD);
    }

    #[test]
    fn an_ordinary_reading_on_a_quiet_host_is_not_reported_as_unusual() {
        // The control. Every test above passes on a detector that calls everything an anomaly.
        let window = quiet();
        let ordinary = deviation(&window, 2.1).expect("long enough");
        assert!(
            ordinary.spreads_away.abs() < 3.0,
            "an ordinary reading was {:.2} spreads out",
            ordinary.spreads_away
        );
    }

    #[test]
    fn what_is_ordinary_is_this_host_and_not_a_corpus() {
        // Two hosts, both perfectly well, with medians an order of magnitude apart. The same
        // observation is unremarkable on one and extreme on the other, which is the property a
        // model trained elsewhere cannot have.
        let busy: Vec<f64> = (0..32)
            .map(|index| 45.0 + f64::from(index % 5) * 0.4)
            .collect();
        let idle: Vec<f64> = (0..32)
            .map(|index| 4.0 + f64::from(index % 5) * 0.4)
            .collect();

        let on_busy = deviation(&busy, 46.0).expect("long enough");
        let on_idle = deviation(&idle, 46.0).expect("long enough");

        assert!(on_busy.spreads_away.abs() < 5.0, "{on_busy:?}");
        assert!(on_idle.spreads_away > 50.0, "{on_idle:?}");
    }

    #[test]
    fn a_deviation_can_be_checked_by_hand_from_what_it_carries() {
        // Explainability is not a property of the prose around a number; it is whether a reader
        // handed the fields can arrive at the same answer.
        let window = quiet();
        let found = deviation(&window, 10.0).expect("long enough");
        let by_hand = (found.observed - found.ordinary) / (found.spread * MAD_TO_SIGMA);
        assert!((found.spreads_away - by_hand).abs() < 1e-12);
    }

    #[test]
    fn the_middle_of_an_even_window_is_between_the_two_middle_values() {
        assert_eq!(median(&[1.0, 2.0, 3.0, 4.0]), Some(2.5));
        assert_eq!(median(&[3.0, 1.0, 2.0]), Some(2.0));
        assert_eq!(median(&[]), None);
    }

    #[test]
    fn a_weighted_average_follows_where_something_is_heading() {
        let rising: Vec<f64> = (0..20).map(f64::from).collect();
        let followed = exponentially_weighted(&rising, 0.5).expect("not empty");
        let middle = median(&rising).expect("not empty");
        assert!(
            followed > middle,
            "the average did not follow the rise: {followed} vs {middle}"
        );
        assert_eq!(exponentially_weighted(&[], 0.5), None);
    }
}
