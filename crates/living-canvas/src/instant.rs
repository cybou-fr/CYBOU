// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! One way to show an instant, everywhere.
//!
//! The owners record time as RFC 3339 with whatever precision they had —
//! `2026-08-22T16:33:54.409963793Z`. That is the right thing to store and the wrong thing to put on
//! a card: nine subsecond digits and two format characters are noise to the person reading, and a
//! surface covered in them reads as one built for whoever wrote it.
//!
//! Shortening is not the same as rounding away the truth. Nothing here changes the value; the exact
//! string stays available beside every display through a tooltip, so a person who wants the
//! nanosecond can still have it. What is dropped is only the part nobody was reading.
//!
//! This lives outside `components` on purpose: it is string arithmetic, so it is tested natively
//! rather than only by looking at a screen.

/// An instant as a person reads it: `2026-08-22 16:33:54 UTC`.
///
/// Returns the input unchanged when it is not the shape this understands. A formatter that
/// invented a value for something it could not parse would be doing exactly what this file exists
/// to stop.
#[must_use]
pub fn instant_label(rfc3339: &str) -> String {
    let trimmed = rfc3339.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let Some((date, rest)) = trimmed.split_once('T') else {
        return trimmed.to_owned();
    };
    // The time, up to seconds. Everything after is subsecond precision or a zone marker.
    let time: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == ':')
        .collect();
    if date.len() != 10 || time.len() < 8 {
        return trimmed.to_owned();
    }
    // Only UTC is labelled, because only UTC is what the owners write. An offset this does not
    // recognise is left alone rather than relabelled.
    let zone = if rest.ends_with('Z') { " UTC" } else { "" };
    format!("{date} {}{zone}", &time[..8])
}

/// Just the clock part: `16:33:54 UTC`.
///
/// For places already saying which day they mean — an event stream whose rows are all from the last
/// few minutes does not need the date on every line.
#[must_use]
pub fn time_label(rfc3339: &str) -> String {
    let full = instant_label(rfc3339);
    match full.split_once(' ') {
        Some((_, rest)) if rest.len() >= 8 => rest.to_owned(),
        _ => full,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nanoseconds_are_dropped_and_the_instant_is_not() {
        assert_eq!(
            instant_label("2026-08-22T16:33:54.409963793Z"),
            "2026-08-22 16:33:54 UTC"
        );
        assert_eq!(
            instant_label("2026-08-20T11:29:59Z"),
            "2026-08-20 11:29:59 UTC"
        );
    }

    #[test]
    fn something_this_does_not_understand_is_returned_untouched() {
        // A formatter that invented a value for an input it could not read would be the failure
        // this file exists to stop, in the file that exists to stop it.
        for odd in ["", "not a time", "2026-08-22", "20260822T163354Z"] {
            assert_eq!(instant_label(odd), odd);
        }
    }

    #[test]
    fn an_offset_that_is_not_utc_is_not_labelled_utc() {
        let shifted = "2026-08-22T16:33:54+02:00";
        assert!(!instant_label(shifted).contains("UTC"));
    }

    #[test]
    fn the_clock_alone_keeps_the_zone() {
        assert_eq!(time_label("2026-08-22T16:33:54.409963793Z"), "16:33:54 UTC");
    }

    #[test]
    fn surrounding_space_does_not_change_the_answer() {
        assert_eq!(
            instant_label("  2026-08-22T16:33:54Z  "),
            "2026-08-22 16:33:54 UTC"
        );
    }
}
