// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Turning a projection into the line a person reads.
//!
//! Here rather than in the card for the reason this tree has learned four times: `components` is
//! compiled only for `wasm32`, so anything living there is invisible to `cargo test` and stays
//! invisible until somebody looks at a screen. Arithmetic and string work belong in a module the
//! native test run can reach; a component only draws.
//!
//! Two decisions are made here rather than left to whoever writes the markup, and both are about
//! not overstating an estimate.

use cybou_web_contracts::ProjectionProjection;

/// How long until something, in the words a person would use.
///
/// Rounded hard and deliberately. A projection is an extrapolation from a slope; rendering it as
/// `2 days 7 hours 14 minutes` would give it a precision the estimate does not have, and a reader
/// would compare two of them as though the difference meant something.
#[must_use]
pub fn roughly(seconds: i64) -> String {
    match seconds {
        ..=0 => "now".to_owned(),
        1..=5400 => format!("~{} min", (seconds + 30) / 60),
        5401..=172_800 => format!("~{} h", (seconds + 1800) / 3600),
        _ => format!("~{} days", (seconds + 43_200) / 86_400),
    }
}

/// One line about where a subject is heading, or nothing.
///
/// The subjects that are flat, falling, or have too little history are not drawn. A page listing
/// eight rows of *not at this rate* would bury the one row that matters, and the reader who most
/// needs to see it is the one skimming.
#[must_use]
pub fn heading_line(projection: &ProjectionProjection) -> Option<String> {
    match projection.reaching.as_str() {
        "at-this-rate" => {
            let after = roughly(projection.after_seconds?);
            let reach = if projection.beyond_what_was_watched {
                // Said, not hidden. The most useful projection is usually the least certain, and a
                // reader deciding whether to act tonight is entitled to know which one they hold.
                " — further ahead than I have watched"
            } else {
                ""
            };
            Some(format!(
                "{} reaches {:.2} in {after}{reach}",
                projection.subject, projection.threshold
            ))
        }
        "already" => Some(format!(
            "{} is past {:.2}",
            projection.subject, projection.threshold
        )),
        // Includes a verdict this build does not recognise. A line that guessed at one would put a
        // sentence on the page that no layer below it produced.
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn projection(reaching: &str, after: Option<i64>, beyond: bool) -> ProjectionProjection {
        ProjectionProjection {
            subject: "filesystem.root.used".to_owned(),
            trend: "rising".to_owned(),
            current: 0.71,
            threshold: 0.95,
            reaching: reaching.to_owned(),
            after_seconds: after,
            beyond_what_was_watched: beyond,
            watched_seconds: 21_600,
        }
    }

    #[test]
    fn a_duration_is_rounded_to_the_precision_the_estimate_has() {
        // Rendered finer, a reader would compare two projections as though the difference meant
        // something. It is an extrapolation from a slope.
        assert_eq!(roughly(0), "now");
        assert_eq!(roughly(-90), "now");
        assert_eq!(roughly(600), "~10 min");
        assert_eq!(roughly(7200), "~2 h");
        assert_eq!(roughly(3 * 86_400), "~3 days");
    }

    #[test]
    fn a_projection_further_ahead_than_the_window_says_so_on_the_line() {
        // The most useful projection is usually the least certain, and a reader deciding whether to
        // act tonight is entitled to know which of the two they are holding.
        let young = heading_line(&projection("at-this-rate", Some(3 * 86_400), true))
            .expect("a rising subject is drawn");
        assert!(young.contains("~3 days"), "{young}");
        assert!(
            young.contains("further ahead than I have watched"),
            "{young}"
        );

        let grounded = heading_line(&projection("at-this-rate", Some(3600), false))
            .expect("a rising subject is drawn");
        assert!(!grounded.contains("further ahead"), "{grounded}");
    }

    #[test]
    fn nothing_is_drawn_for_a_subject_that_does_not_arrive() {
        assert!(heading_line(&projection("not-at-this-rate", None, false)).is_none());
        assert!(heading_line(&projection("not-enough-history", None, false)).is_none());
        assert!(heading_line(&projection("at-this-rate", None, false)).is_none());
        assert!(heading_line(&projection("something-new", None, false)).is_none());
    }

    #[test]
    fn something_already_past_its_threshold_is_drawn_without_a_time() {
        let past = heading_line(&projection("already", None, false)).expect("drawn");
        assert!(past.contains("is past 0.95"), "{past}");
        assert!(!past.contains(" in "), "{past}");
    }
}
