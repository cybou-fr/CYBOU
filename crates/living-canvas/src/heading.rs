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

use cybou_web_contracts::{
    DeliveryProjection, FindingProjection, OfferProjection, ProjectionProjection,
    ReadingProjection, WatchedProjection,
};

/// What is ordinary for this host, in the words a person reads beside a reading.
///
/// A baseline that does not exist yet is said, not filled in. A categorical finding needs none — a
/// filesystem at 97% is a problem wherever it is — so a fresh host produces real findings with no
/// notion of ordinary behind them, and drawing `ordinary 0.00` there would put a number on the page
/// claiming the reading is enormously far from normal, about a host nobody has watched.
#[must_use]
pub fn baseline_line(reading: &ReadingProjection) -> String {
    match reading.ordinary {
        Some(ordinary) => format!("ordinary {ordinary:.2}"),
        None => "no baseline yet".to_owned(),
    }
}

/// One earlier delivery, as a line a person reads.
///
/// The counts, not the contents. What a history answers is *when did what I am given change, and by
/// how much*, and the change is what a person is looking for — a delivery that supplied more than
/// the one before it is the thing worth noticing on a page about what somebody was shown.
#[must_use]
pub fn delivery_line(delivery: &DeliveryProjection) -> String {
    // Said only when there was something. "0 withheld" on every line is noise that trains a reader
    // to skip the column that occasionally says something.
    let held_back = if delivery.withheld_count > 0 {
        format!(", {} withheld", delivery.withheld_count)
    } else {
        String::new()
    };
    format!(
        "{} — {} supplied, {} accounted for{held_back}",
        delivery.at, delivery.supplied, delivery.accounted_for
    )
}

/// The line that names one finding.
///
/// What it means, and which thing it is about. The second half is the one that took a rewrite of
/// every layer beneath this to arrive, and it used to stop one inch short: the instance reached the
/// gateway and was dropped before the wire, so a host watching four certificates drew four rows
/// reading *a watched certificate is close to expiry, or past it* and nothing else.
#[must_use]
pub fn finding_title(finding: &FindingProjection) -> String {
    match &finding.about {
        Some(about) => format!("{} — {about}", finding.means),
        None => finding.means.clone(),
    }
}

/// What an offer would act on, when that is worth showing.
///
/// Nothing for a target the proposal did not know. `systemd:<unit>` is a placeholder meaning *some
/// unit, and this host cannot say which* — drawn literally it reads as a unit called `<unit>`, and
/// a reader would take it for a real name badly formatted rather than for an admission. The two
/// cases have to stay distinguishable on screen for the same reason they stay distinguishable in
/// the proposal.
#[must_use]
pub fn offer_target(offer: &OfferProjection) -> Option<String> {
    if offer.target.contains('<') {
        return None;
    }
    Some(offer.target.clone())
}

/// The watched things this host currently cannot see, worded for a person.
///
/// Empty when everything declared was read, so the card shows nothing rather than a reassuring
/// "0 problems". What it exists to prevent is the opposite failure: a declared certificate that
/// produced no reading used to be simply absent from the page, which reads exactly like a
/// certificate nobody declared — and the operator who declared it is the one being told, by that
/// silence, that it is fine.
///
/// Each state is worded differently because they call for different actions. Never read is a path
/// that may not exist; read failed is usually a permission; stale is usually the sampler and not
/// the thing sampled.
#[must_use]
pub fn unseen_line(watched: &[WatchedProjection]) -> Option<String> {
    let unseen: Vec<String> = watched
        .iter()
        .filter_map(|resource| {
            let why = match resource.state.as_str() {
                "never-read" => "never read",
                "read-failed" => "could not be read",
                "stale" => "not read lately",
                // Observed, or a state this build does not know about. Neither is something to
                // report as unseen — inventing a complaint about an unknown state would be worse
                // than the silence this function exists to break.
                _ => return None,
            };
            // An em dash rather than a second pair of parentheses. The subject already
            // carries the thing it is about in parentheses, and "(/etc/ssl/a.pem) (never read)"
            // reads as two labels rather than one thing and its state.
            Some(format!("{} — {why}", resource.subject))
        })
        .collect();
    if unseen.is_empty() {
        return None;
    }
    Some(format!("Watched but not seen: {}", unseen.join(", ")))
}

/// Five-stage self-healing lifecycle for an action proposal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelfHealingStage {
    /// Stage name.
    pub name: &'static str,
    /// Whether this stage is currently in progress.
    pub active: bool,
    /// Whether this stage has completed.
    pub completed: bool,
}

/// Compute the self-healing timeline stages from authorization and execution status.
#[must_use]
pub fn self_healing_timeline(verdict: &str, executed: bool, relieved: bool) -> Vec<SelfHealingStage> {
    let decided = verdict == "granted" || verdict.contains("pre-authorized");
    vec![
        SelfHealingStage {
            name: "Detected",
            active: false,
            completed: true,
        },
        SelfHealingStage {
            name: "Decided",
            active: !decided && !executed,
            completed: decided || executed || relieved,
        },
        SelfHealingStage {
            name: "Acting",
            active: decided && !executed && !relieved,
            completed: executed || relieved,
        },
        SelfHealingStage {
            name: "Re-observed",
            active: executed && !relieved,
            completed: relieved,
        },
        SelfHealingStage {
            name: "Relieved",
            active: relieved,
            completed: relieved,
        },
    ]
}

/// Explain why a reading constitutes a finding by comparing observation to baseline.
#[must_use]
pub fn why_explanation(observed: f64, ordinary: Option<f64>, spread: Option<f64>) -> String {
    match (ordinary, spread) {
        (Some(ord), Some(spr)) if ord > 0.0 => {
            let ratio = (observed - ord) / ord * 100.0;
            if ratio.abs() >= 1.0 {
                format!("{ratio:+.0}% vs ordinary ({ord:.2} ± {spr:.2})")
            } else {
                format!("at ordinary ({ord:.2} ± {spr:.2})")
            }
        }
        (Some(ord), _) if ord > 0.0 => {
            let ratio = (observed - ord) / ord * 100.0;
            format!("{ratio:+.0}% vs baseline ({ord:.2})")
        }
        (None, Some(spr)) => format!("observed {observed:.2} (spread {spr:.2})"),
        _ => format!("observed reading: {observed:.2}"),
    }
}

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
    use cybou_web_contracts::WatchedProjection;

    #[test]
    fn a_baseline_nobody_has_established_is_said_rather_than_filled_in() {
        // A categorical finding needs no baseline, so a fresh host produces real findings with none
        // behind them. Drawing "ordinary 0.00" there would put a number on the page claiming the
        // reading is enormously far from normal, about a host nobody has watched.
        let reading = |ordinary: Option<f64>| ReadingProjection {
            subject: "filesystem.root.used".to_owned(),
            observed: 0.97,
            ordinary,
            spread: ordinary.map(|_| 0.01),
        };
        assert_eq!(super::baseline_line(&reading(None)), "no baseline yet");
        assert_eq!(super::baseline_line(&reading(Some(0.62))), "ordinary 0.62");
    }

    fn delivery(supplied: u32, withheld_count: u32) -> DeliveryProjection {
        DeliveryProjection {
            at: "2026-08-24T11:00:00Z".to_owned(),
            supplied,
            accounted_for: supplied,
            provenance_count: supplied,
            withheld_count,
        }
    }

    #[test]
    fn a_delivery_that_held_nothing_back_does_not_say_so() {
        // "0 withheld" on every line is noise that trains a reader to skip the column that
        // occasionally says something.
        let line = super::delivery_line(&delivery(4, 0));
        assert!(!line.contains("withheld"), "{line}");
        assert!(line.contains("4 supplied"), "{line}");
    }

    #[test]
    fn a_delivery_that_held_something_back_says_how_much() {
        let line = super::delivery_line(&delivery(4, 2));
        assert!(line.contains("2 withheld"), "{line}");
    }

    fn finding(about: Option<&str>) -> cybou_web_contracts::FindingProjection {
        cybou_web_contracts::FindingProjection {
            finding: "certificate.expiring".to_owned(),
            about: about.map(ToOwned::to_owned),
            means: "a watched certificate is close to expiry, or past it".to_owned(),
            strength: "strong".to_owned(),
            since: String::new(),
            readings: Vec::new(),
            offers: Vec::new(),
        }
    }

    fn offer(target: &str) -> cybou_web_contracts::OfferProjection {
        cybou_web_contracts::OfferProjection {
            operation: "service.restart".to_owned(),
            target: target.to_owned(),
            risk: "medium".to_owned(),
            reversible: true,
            verdict: "requires-confirmation".to_owned(),
            reason: String::new(),
        }
    }

    #[test]
    fn two_findings_about_two_things_are_two_different_lines() {
        // The whole point of carrying the instance from the window to the reader. Two certificates
        // close to expiry share a `means` word for word, and two identical rows are two rows a
        // person cannot act on.
        let first = super::finding_title(&finding(Some("/etc/ssl/a.pem")));
        let second = super::finding_title(&finding(Some("/etc/ssl/b.pem")));
        assert_ne!(first, second);
        assert!(first.contains("/etc/ssl/a.pem"), "{first}");
        assert!(first.contains("close to expiry"), "{first}");
    }

    #[test]
    fn a_finding_about_the_host_itself_is_not_given_a_name_it_does_not_have() {
        // A trailing dash with nothing after it, or the word "None", would both be this surface
        // inventing a subject for something that is about the machine.
        let title = super::finding_title(&finding(None));
        assert_eq!(
            title,
            "a watched certificate is close to expiry, or past it"
        );
    }

    #[test]
    fn a_target_the_proposal_did_not_know_is_not_drawn_as_one_it_did() {
        // `systemd:<unit>` is this host saying it does not know which unit it means. Drawn
        // literally it reads as a real name badly formatted, which is the opposite of an admission.
        assert_eq!(super::offer_target(&offer("systemd:<unit>")), None);
        assert_eq!(super::offer_target(&offer("filesystem:<device>")), None);
        assert_eq!(
            super::offer_target(&offer("systemd:postgresql.service")),
            Some("systemd:postgresql.service".to_owned())
        );
    }

    fn watched(subject: &str, state: &str) -> WatchedProjection {
        WatchedProjection {
            subject: subject.to_owned(),
            state: state.to_owned(),
            at: None,
            value: None,
        }
    }

    #[test]
    fn a_host_that_read_everything_it_watches_says_nothing_about_it() {
        // Not "0 problems". A reassurance nobody asked for is one more line between the reader and
        // the line that matters.
        let all_read = [
            watched("certificate.days.remaining (/etc/ssl/a.pem)", "observed"),
            watched("service.active (caddy.service)", "observed"),
        ];
        assert_eq!(super::unseen_line(&all_read), None);
    }

    #[test]
    fn each_way_of_not_seeing_something_is_worded_as_itself() {
        // They call for different actions: a path that may not exist, a permission, and a sampler
        // that stopped. One word for all three would send an operator to the wrong place.
        let line = super::unseen_line(&[
            watched("certificate.days.remaining (/etc/ssl/a.pem)", "never-read"),
            watched("service.active (caddy.service)", "read-failed"),
            watched("backup.age.days (/var/backups/db)", "stale"),
            watched("load.average", "observed"),
        ])
        .expect("three of the four are not seen");

        assert!(line.contains("(/etc/ssl/a.pem) — never read"), "{line}");
        assert!(
            line.contains("(caddy.service) — could not be read"),
            "{line}"
        );
        assert!(
            line.contains("(/var/backups/db) — not read lately"),
            "{line}"
        );
        assert!(!line.contains("load.average"), "{line}");
    }

    #[test]
    fn a_state_this_build_does_not_know_is_not_invented_into_a_complaint() {
        // A newer organ sending a state this canvas has not been taught must not produce a line
        // saying something is wrong with a thing that may be perfectly fine.
        assert_eq!(
            super::unseen_line(&[watched("load.average", "shimmering")]),
            None
        );
    }

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
        // A probe that stopped has no heading either. Where it *was* going is not where it is
        // going, and the staleness is already said on its own line — a heading here would be the
        // one place on the page still claiming a live trend for a metric nobody is reading.
        assert!(heading_line(&projection("readings-stopped", None, false)).is_none());
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
