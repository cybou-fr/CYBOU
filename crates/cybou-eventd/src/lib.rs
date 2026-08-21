// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Canonical Journal writer and Event1 D-Bus service daemon for Cybou.
//!
//! Provides the single authoritative Event1 ownership boundary, durable-before-visible
//! transaction guarantees, origin authentication, and consumer offset tracking.

pub mod core;
pub mod erasure;
pub mod error;
pub mod offsets;
#[cfg(target_os = "linux")]
pub mod service;
pub mod verification;

pub use core::EventCore;
pub use error::{EventError, RESERVED_ORGAN_IDENTITIES, SubmitResult, is_reserved_organ};
pub use offsets::PersistedOffsets;
pub use verification::{FullSweepStep, VerificationState, decode_hex, encode_hex, format_instant};

#[cfg(test)]
mod tests {
    use cybou_protocol::{
        Kind, admission::ErasureReason, canonical::CanonicalEnvelope, unix_millis,
    };
    use tempfile::tempdir;
    use time::OffsetDateTime;
    use uuid::Uuid;

    use super::*;

    fn caused_by(cause: &CanonicalEnvelope, kind: Kind, text: &str) -> CanonicalEnvelope {
        let mut envelope = observation(text);
        envelope.kind = kind as u16;
        envelope.causation_id = cause.message_id;
        envelope
    }

    fn citing(
        evidence: &CanonicalEnvelope,
        cause: &CanonicalEnvelope,
        kind: Kind,
        text: &str,
    ) -> CanonicalEnvelope {
        let mut envelope = observation(text);
        envelope.kind = kind as u16;
        envelope.causation_id = cause.message_id;
        envelope.evidence = vec![evidence.message_id];
        envelope
    }

    fn observation(text: &str) -> CanonicalEnvelope {
        let observation = cybou_protocol::observation::ObservationV1 {
            source_id: "test".into(),
            subject: "a-subject".into(),
            value: ciborium::Value::Text(text.into()),
            acquired_at: "2026-08-21T00:00:00.000Z".into(),
            freshness_until: "2026-08-22T00:00:00.000Z".into(),
            provenance: "a fixture".into(),
        };
        let mut payload = Vec::new();
        ciborium::into_writer(&observation, &mut payload).expect("encode");
        CanonicalEnvelope {
            schema_version: 4,
            message_id: Uuid::new_v4(),
            correlation_id: Uuid::new_v4(),
            causation_id: Uuid::nil(),
            origin_organ: "testd".into(),
            origin_node: String::new(),
            kind: Kind::Observation as u16,
            wall_time_ms: 0,
            monotonic_time: 0,
            logical_clock: 1,
            confidence: 1.0,
            evidence: Vec::new(),
            payload,
            privacy: 1,
            capability_scope: String::new(),
            sealed: false,
            key_domain_id: Uuid::nil(),
            key_epoch: 0,
            retention_class: 2,
            retention_policy_version: 0,
            retain_until_ms: 0,
            sensitivity: 1,
        }
    }

    fn dummy_envelope(message_id: Uuid, origin_organ: &str) -> CanonicalEnvelope {
        CanonicalEnvelope {
            schema_version: 3,
            message_id,
            correlation_id: Uuid::new_v4(),
            causation_id: Uuid::nil(),
            origin_organ: origin_organ.to_string(),
            origin_node: String::new(),
            kind: 1, // Observation
            wall_time_ms: unix_millis(OffsetDateTime::now_utc()),
            monotonic_time: 100,
            logical_clock: 1,
            confidence: 1.0,
            evidence: vec![],
            payload: vec![1, 2, 3, 4],
            privacy: 1,
            capability_scope: String::new(),
            sealed: false,
            key_domain_id: Uuid::nil(),
            key_epoch: 0,
            retention_class: 2,
            retention_policy_version: 0,
            retain_until_ms: 0,
            sensitivity: 1,
        }
    }

    fn core_in(dir: &std::path::Path) -> EventCore {
        EventCore::open(dir.join("journal.sqlite3")).expect("a journal")
    }

    #[test]
    fn forgetting_something_forgets_what_was_derived_from_it() {
        let dir = tempfile::tempdir().expect("temp dir");
        let core = core_in(dir.path());

        let root = observation("the thing to forget");
        core.submit(&root, None).expect("root accepted");
        let derived = caused_by(&root, Kind::Hypothesis, "because of the thing to forget");
        core.submit(&derived, None).expect("derived accepted");
        let unrelated = observation("nothing to do with it");
        core.submit(&unrelated, None).expect("unrelated accepted");
        let citing_it = citing(
            &root,
            &unrelated,
            Kind::Learning,
            "learned from the thing to forget",
        );
        core.submit(&citing_it, None).expect("citing accepted");

        let outcome = core
            .request_erasure(&root.message_id, ErasureReason::UserRequested)
            .expect("the erasure runs");

        assert!(outcome.closure.contains(&root.message_id));
        assert!(
            outcome.closure.contains(&derived.message_id),
            "a contribution caused by the target is part of what must be forgotten"
        );
        assert!(
            outcome.closure.contains(&citing_it.message_id),
            "a contribution citing the target as evidence is a dependent too"
        );
        assert!(
            !outcome.closure.contains(&unrelated.message_id),
            "something that merely happened afterwards is not a descendant"
        );

        for id in [root.message_id, derived.message_id, citing_it.message_id] {
            let envelope = core.find_by_message_id(&id).expect("the row survives");
            assert!(envelope.payload.is_empty(), "the payload must be redacted");
            assert_eq!(envelope.origin_organ, "testd", "provenance must survive");
        }
        let untouched = core
            .find_by_message_id(&unrelated.message_id)
            .expect("the row survives");
        assert!(!untouched.payload.is_empty());
    }

    #[test]
    fn an_erasure_is_recorded_at_both_ends_and_says_why_without_saying_what() {
        let dir = tempfile::tempdir().expect("temp dir");
        let core = core_in(dir.path());
        let root = observation("something private");
        core.submit(&root, None).expect("accepted");

        let before = core.erasure_epoch();
        core.request_erasure(&root.message_id, ErasureReason::ConsentWithdrawn)
            .expect("the erasure runs");
        assert!(core.erasure_epoch() > before, "the epoch has to advance");

        let all = core.replay(0, 128);
        let requested: Vec<_> = all
            .iter()
            .filter(|e| Kind::from_u16(e.kind) == Some(Kind::ErasureRequested))
            .collect();
        let applied: Vec<_> = all
            .iter()
            .filter(|e| Kind::from_u16(e.kind) == Some(Kind::ErasureApplied))
            .collect();
        assert_eq!(requested.len(), 1);
        assert_eq!(applied.len(), 1);
        assert_eq!(
            applied[0].causation_id, requested[0].message_id,
            "the applied record has to name the request it completes"
        );

        let (target, reason) =
            erasure::decode_erasure_record(requested[0]).expect("a readable record");
        assert_eq!(target, root.message_id);
        assert_eq!(reason, ErasureReason::ConsentWithdrawn);
        let text = String::from_utf8_lossy(&requested[0].payload).to_string();
        assert!(
            !text.contains("something private"),
            "an erasure record must never restate what it erased"
        );
    }

    #[test]
    fn an_erasure_record_cannot_itself_be_erased() {
        let dir = tempfile::tempdir().expect("temp dir");
        let core = core_in(dir.path());
        let root = observation("something private");
        core.submit(&root, None).expect("accepted");
        core.request_erasure(&root.message_id, ErasureReason::UserRequested)
            .expect("the first erasure runs");

        let record = core
            .replay(0, 128)
            .into_iter()
            .find(|e| Kind::from_u16(e.kind) == Some(Kind::ErasureRequested))
            .expect("an erasure record");

        core.request_erasure(&record.message_id, ErasureReason::UserRequested)
            .expect("the request is accepted");
        let after = core
            .find_by_message_id(&record.message_id)
            .expect("the record survives");
        assert!(
            !after.payload.is_empty(),
            "an erasure record keeps its payload however often it is targeted"
        );
    }

    #[test]
    fn an_erasure_interrupted_before_it_finished_is_finished_on_the_next_start() {
        let dir = tempfile::tempdir().expect("temp dir");
        let core = core_in(dir.path());
        let root = observation("something private");
        core.submit(&root, None).expect("accepted");

        core.record_erasure_step(
            Kind::ErasureRequested,
            &root.message_id,
            ErasureReason::UserRequested,
            None,
        )
        .expect("the request is recorded");
        assert!(
            !core
                .find_by_message_id(&root.message_id)
                .expect("the row")
                .payload
                .is_empty(),
            "nothing has been erased yet"
        );

        assert_eq!(core.resume_erasures().expect("resumption runs"), 1);
        assert!(
            core.find_by_message_id(&root.message_id)
                .expect("the row")
                .payload
                .is_empty(),
            "the interrupted erasure has to complete"
        );

        assert_eq!(core.resume_erasures().expect("resumption runs"), 0);
    }

    #[test]
    fn erasing_something_the_journal_does_not_hold_is_refused() {
        let dir = tempfile::tempdir().expect("temp dir");
        let core = core_in(dir.path());
        assert!(
            core.request_erasure(&Uuid::new_v4(), ErasureReason::UserRequested)
                .is_err(),
            "there is nothing to forget, and saying otherwise would be a false record"
        );
    }

    #[test]
    fn a_full_sweep_never_moves_the_trusted_checkpoint() {
        let dir = tempfile::tempdir().expect("temp dir");
        let db_path = dir.path().join("journal.sqlite3");
        let core = EventCore::open(&db_path).expect("open event core");

        let checkpoint_path = core.checkpoint_path().to_path_buf();
        let step = core.verify_fully_step(8);
        assert!(step.broken_at.is_none());
        assert!(
            !checkpoint_path.exists(),
            "a full sweep must not write the checkpoint the incremental pass trusts"
        );

        let _ = core.verify_page(8, time::OffsetDateTime::now_utc());
        assert!(checkpoint_path.exists());
    }

    #[test]
    fn hex_round_trips_the_checkpoint_hash() {
        let hash = vec![0x00, 0x0f, 0xa5, 0xff];
        let text = encode_hex(&hash);
        assert_eq!(text, "000fa5ff");
        assert_eq!(decode_hex(&text), Some(hash));
        assert_eq!(decode_hex("abc"), None);
        assert_eq!(decode_hex("zz"), None);
    }

    #[test]
    fn submit_and_query_lifecycle() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("journal.sqlite3");
        let core = EventCore::open(&db_path).expect("open event core");

        assert_eq!(core.count(), 0);
        assert_eq!(core.head(), None);

        let env1 = dummy_envelope(Uuid::new_v4(), "unreserved_client");
        let res = core.submit(&env1, None).expect("submit");
        assert_eq!(res.sequence, 1);

        assert_eq!(core.count(), 1);
        let head = core.head().expect("head exists");
        assert_eq!(head.message_id, env1.message_id);

        let retrieved = core.at_sequence(1).expect("at sequence 1");
        assert_eq!(retrieved.message_id, env1.message_id);

        let found = core
            .find_by_message_id(&env1.message_id)
            .expect("found by id");
        assert_eq!(found.message_id, env1.message_id);
    }

    #[test]
    fn unauthenticated_reserved_origin_is_refused() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("journal.sqlite3");
        let core = EventCore::open(&db_path).expect("open event core");

        let env = dummy_envelope(Uuid::new_v4(), "identityd");
        let err = core.submit(&env, None).expect_err("should refuse");
        assert!(matches!(err, EventError::OriginUnauthentic(_)));
    }

    #[test]
    fn self_assessment_and_learning_are_permitted_but_erasure_is_refused() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("journal.sqlite3");
        let core = EventCore::open(&db_path).expect("open event core");

        let root_obs = dummy_envelope(Uuid::new_v4(), "unreserved");
        let root_res = core.submit(&root_obs, None).expect("root observation");
        assert_eq!(root_res.sequence, 1);

        let mut env13 = dummy_envelope(Uuid::new_v4(), "selfd");
        env13.kind = Kind::SelfAssessment as u16;
        env13.evidence = vec![root_obs.message_id];
        let res13 = core
            .submit(&env13, Some("selfd"))
            .expect("SelfAssessment permitted");
        assert_eq!(res13.sequence, 2);

        let mut env14 = dummy_envelope(Uuid::new_v4(), "learning_organ");
        env14.kind = Kind::Learning as u16;
        env14.causation_id = env13.message_id;
        let res14 = core.submit(&env14, None).expect("Learning permitted");
        assert_eq!(res14.sequence, 3);

        let mut env15 = dummy_envelope(Uuid::new_v4(), "admin");
        env15.kind = Kind::ErasureRequested as u16;
        let err15 = core
            .submit(&env15, None)
            .expect_err("ErasureRequested must be refused");
        assert!(matches!(err15, EventError::ErasureRefused));

        let mut env16 = dummy_envelope(Uuid::new_v4(), "admin");
        env16.kind = Kind::ErasureApplied as u16;
        let err16 = core
            .submit(&env16, None)
            .expect_err("ErasureApplied must be refused");
        assert!(matches!(err16, EventError::ErasureRefused));
    }
}
