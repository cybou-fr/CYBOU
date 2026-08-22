// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Epistemic projection and belief validity engine (ADR-0027: observation != knowledge).
//!
//! Evaluates incoming observations and journal replay against historical evidence,
//! maintaining reconstructible epistemic propositions with dispute and staleness tracking.

pub mod core;
pub mod types;

#[cfg(target_os = "linux")]
pub mod service;

pub use core::EpistemicCore;
pub use types::{
    BELIEF_RULE_VERSION, EpistemicBelief, EpistemicError, EpistemicState, EpistemicStatus, as_of,
    observed_claim,
};

#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::tempdir;
    use time::OffsetDateTime;
    use uuid::Uuid;

    use super::*;

    #[test]
    fn beliefs_and_the_cursor_that_produced_them_are_written_together() {
        let dir = tempdir().expect("temp dir");
        let path = dir.path().join("epistemic-state.json");
        let core = EpistemicCore::open(&path).expect("open state");

        core.ingest_at(
            "operating-system",
            "Debian GNU/Linux 13 (trixie)",
            1.0,
            None,
            OffsetDateTime::now_utc(),
            Some(42),
        );

        let restarted = EpistemicCore::open(&path).expect("reopen state");
        assert_eq!(restarted.cursor(), 42);
        assert_eq!(restarted.projection().len(), 1);
    }

    #[test]
    fn beliefs_derived_by_an_older_rule_are_rebuilt_rather_than_trusted() {
        let dir = tempdir().expect("temp dir");
        let path = dir.path().join("epistemic-state.json");

        fs::write(
            &path,
            r#"{"cursor":42,"beliefs":{"organ.perceptiond":{"subject":"organ.perceptiond","value":"garbage","confidence":1.0,"evidence":[],"lastCorroboratedAt":"2026-08-19T12:00:00Z","status":"disputed"}}}"#,
        )
        .expect("write legacy state");

        let core = EpistemicCore::open(&path).expect("open over legacy state");
        assert_eq!(core.cursor(), 0, "replay must restart from the Journal");
        assert!(
            core.projection().is_empty(),
            "conclusions of a rule that no longer exists must not survive"
        );
    }

    #[test]
    fn a_belief_is_about_what_was_observed_not_about_who_observed_it() {
        let observation = cybou_protocol::observation::ObservationV1 {
            source_id: "linux.system".into(),
            subject: "operating-system".into(),
            value: ciborium::Value::Text("Debian GNU/Linux 13 (trixie)".into()),
            acquired_at: "2026-08-19T17:54:15.103Z".into(),
            freshness_until: "2026-08-19T17:59:15.103Z".into(),
            provenance: "os-release from /etc/os-release".into(),
        };
        let mut payload = Vec::new();
        ciborium::into_writer(&observation, &mut payload).expect("encode observation");

        let (subject, value, fresh_until) =
            observed_claim(&payload).expect("a readable observation");
        assert_eq!(subject, "operating-system");
        assert_eq!(value, "Debian GNU/Linux 13 (trixie)");
        assert_eq!(
            fresh_until.expect("the horizon was read").unix_timestamp(),
            OffsetDateTime::parse(
                "2026-08-19T17:59:15.103Z",
                &time::format_description::well_known::Rfc3339
            )
            .expect("fixture parses")
            .unix_timestamp()
        );

        assert_eq!(observed_claim(b"not cbor at all"), None);
        assert_eq!(observed_claim(&[]), None);
    }

    #[test]
    fn a_report_that_outlived_its_horizon_is_replaced_rather_than_disputed() {
        let core = EpistemicCore::new();
        let rfc = &time::format_description::well_known::Rfc3339;
        let at = |text: &str| OffsetDateTime::parse(text, rfc).expect("fixture parses");

        let observed = at("2026-08-20T12:00:00Z");
        let horizon = at("2026-08-20T12:05:00Z");
        core.ingest_at_until("battery", "80", 1.0, None, observed, None, Some(horizon));
        assert_eq!(
            core.query("battery").expect("a belief").status,
            EpistemicStatus::Stale,
            "the horizon is long past, so nothing is vouching for it now"
        );

        core.ingest_at_until(
            "battery",
            "60",
            1.0,
            None,
            at("2026-08-20T12:01:00Z"),
            None,
            Some(horizon),
        );
        assert_eq!(
            core.query("battery").expect("a belief").status,
            EpistemicStatus::Disputed
        );

        let now = OffsetDateTime::now_utc();
        core.ingest_at_until(
            "battery",
            "42",
            1.0,
            None,
            now,
            None,
            Some(now + time::Duration::hours(1)),
        );
        let replaced = core.query("battery").expect("a belief");
        assert_eq!(replaced.value, "42");
        assert_eq!(replaced.status, EpistemicStatus::Superseded);
    }

    #[test]
    fn epistemic_reconstruction_and_persistence() {
        let dir = tempdir().expect("tempdir");
        let state_path = dir.path().join("epistemic.json");

        let core = EpistemicCore::open(&state_path).expect("open");
        let now = OffsetDateTime::now_utc();
        let ev1 = Uuid::new_v4();

        core.ingest("system.os", "Debian 13", 1.0, Some(ev1), now);
        assert_eq!(
            core.query("system.os").unwrap().status,
            EpistemicStatus::Observed
        );

        core.ingest("system.os", "Fedora 40", 0.9, None, now);
        assert_eq!(
            core.query("system.os").unwrap().status,
            EpistemicStatus::Disputed
        );

        let reopened = EpistemicCore::open(&state_path).expect("reopen");
        let b = reopened.query("system.os").expect("reopened belief");
        assert_eq!(b.status, EpistemicStatus::Disputed);
    }
}
