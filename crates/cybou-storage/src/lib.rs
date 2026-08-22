// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Fail-closed, read-only inspection of predecessor Journal databases.

pub mod inspect;
pub mod types;
pub mod verify;
pub mod writer;

pub use inspect::{decode_envelope, inspect_chain, inspect_journal, parse_millis};
pub use types::{
    JOURNAL_SCHEMA_V2, JournalCheckpoint, JournalInspection, JournalVerification,
    REQUIRED_CONTRIBUTION_COLUMNS, REQUIRED_TABLES, StorageError,
};
pub use verify::{verify_journal_from, verify_journal_page};

#[cfg(test)]
mod tests {
    use std::fs;

    use rusqlite::Connection;
    use tempfile::tempdir;

    use super::{
        JOURNAL_SCHEMA_V2, JournalCheckpoint, StorageError, inspect_journal, verify_journal_from,
        verify_journal_page,
    };

    const SCHEMA: &str = "
        CREATE TABLE contribution (
          seq INTEGER PRIMARY KEY, message_id TEXT, correlation_id TEXT, causation_id TEXT,
          origin_organ TEXT, origin_node TEXT, kind INTEGER, wall_time TEXT,
          monotonic_time INTEGER, logical_clock INTEGER, confidence REAL, evidence TEXT, payload BLOB,
          privacy INTEGER, capability TEXT, schema_version INTEGER, hash_version INTEGER,
          prev_hash BLOB, hash BLOB, commitment BLOB, payload_commitment BLOB,
          erased_at TEXT, sealed INTEGER, key_domain TEXT, key_epoch INTEGER,
          retention_class INTEGER, retention_policy INTEGER, retain_until TEXT, sensitivity INTEGER
        );
        CREATE TABLE contribution_evidence (contribution_id TEXT, evidence_id TEXT, ordinal INTEGER);
        CREATE TABLE journal_meta (id INTEGER PRIMARY KEY, erasure_epoch INTEGER, rotated_epoch INTEGER);
        INSERT INTO journal_meta VALUES (1, 4, 3);
        PRAGMA user_version=2;
    ";

    fn populate_valid_chain(connection: &Connection) {
        populate_chain(connection, 2);
    }

    fn populate_chain(connection: &Connection, row_count: u64) {
        use cybou_protocol::canonical::{
            CanonicalEnvelope, canonical_journal_row_v3, commitment_v3, sha256,
        };
        use rusqlite::params;
        use uuid::Uuid;

        let mut previous = Vec::new();
        for sequence in 1_u64..=row_count {
            let id = Uuid::from_u128(sequence.into());
            let wall_time = "2026-08-19T00:00:00.000Z".to_owned();
            let envelope = CanonicalEnvelope {
                schema_version: 4,
                message_id: id,
                correlation_id: id,
                causation_id: Uuid::nil(),
                origin_organ: "fixture".into(),
                origin_node: String::new(),
                kind: 1,
                wall_time_ms: super::parse_millis(&wall_time).expect("fixture time"),
                monotonic_time: 0,
                logical_clock: sequence,
                confidence: 1.0,
                evidence: Vec::new(),
                payload: sequence.to_be_bytes().to_vec(),
                privacy: 0,
                capability_scope: String::new(),
                sealed: false,
                key_domain_id: Uuid::nil(),
                key_epoch: 0,
                retention_class: 2,
                retention_policy_version: 0,
                retain_until_ms: 0,
                sensitivity: 1,
            };
            let (_, payload, commitment) = commitment_v3(&envelope);
            let hash = sha256(&canonical_journal_row_v3(sequence, &previous, &commitment));
            let stored_sequence = i64::try_from(sequence).expect("fixture sequence");
            connection
                .execute(
                    "INSERT INTO contribution (
                       seq, message_id, correlation_id, origin_organ, origin_node, kind, wall_time,
                       monotonic_time, logical_clock, confidence, payload, privacy, capability,
                       schema_version, hash_version, prev_hash, hash, commitment,
                       payload_commitment, sealed, key_epoch, retention_class, retention_policy,
                       sensitivity
                     ) VALUES (
                       ?1, ?2, ?2, 'fixture', '', 1, ?3, 0, ?1, 1.0, ?8, 0, '', 4, 3,
                       ?4, ?5, ?6, ?7, 0, 0, 2, 0, 1
                     )",
                    params![
                        stored_sequence,
                        id.hyphenated().to_string(),
                        wall_time,
                        previous,
                        hash,
                        commitment,
                        payload,
                        envelope.payload
                    ],
                )
                .expect("valid contribution");
            previous = hash.to_vec();
        }
    }

    #[test]
    fn missing_database_is_not_created() {
        let root = tempdir().expect("temporary root");
        let path = root.path().join("missing.db");
        assert!(matches!(inspect_journal(&path), Err(StorageError::Open(_))));
        assert!(!path.exists());
    }

    #[test]
    fn v2_database_is_inspected_without_writing_it() {
        let root = tempdir().expect("temporary root");
        let path = root.path().join("journal.db");
        let connection = Connection::open(&path).expect("fixture database");
        connection.execute_batch(SCHEMA).expect("fixture schema");
        populate_valid_chain(&connection);
        drop(connection);
        let before = fs::read(&path).expect("database bytes");

        let inspection = inspect_journal(&path).expect("compatible journal");

        assert_eq!(inspection.schema_version, JOURNAL_SCHEMA_V2);
        assert_eq!(inspection.contribution_count, 2);
        assert_eq!(inspection.erasure_epoch, 4);
        assert_eq!(inspection.rotated_epoch, 3);
        assert_eq!(fs::read(path).expect("database bytes"), before);
    }

    #[test]
    fn future_or_partial_schema_fails_closed() {
        let root = tempdir().expect("temporary root");
        let future = root.path().join("future.db");
        let connection = Connection::open(&future).expect("future database");
        connection
            .execute_batch("CREATE TABLE contribution(seq INTEGER); PRAGMA user_version=3;")
            .expect("future schema");
        drop(connection);
        assert!(matches!(
            inspect_journal(&future),
            Err(StorageError::UnsupportedSchema { received: 3 })
        ));

        let partial = root.path().join("partial.db");
        let connection = Connection::open(&partial).expect("partial database");
        connection
            .execute_batch("CREATE TABLE contribution(seq INTEGER); PRAGMA user_version=2;")
            .expect("partial schema");
        drop(connection);
        assert!(matches!(
            inspect_journal(&partial),
            Err(StorageError::MissingSchema(_))
        ));
    }

    #[test]
    fn broken_previous_hash_fails_closed() {
        let root = tempdir().expect("temporary root");
        let path = root.path().join("broken.db");
        let connection = Connection::open(&path).expect("fixture database");
        connection.execute_batch(SCHEMA).expect("fixture schema");
        populate_valid_chain(&connection);
        connection
            .execute(
                "UPDATE contribution SET prev_hash=zeroblob(31) WHERE seq=2",
                [],
            )
            .expect("break chain link");
        drop(connection);
        assert!(matches!(
            inspect_journal(&path),
            Err(StorageError::InvalidChain { sequence: 2, .. })
        ));
    }

    #[test]
    fn canonical_hash_and_live_payload_tampering_fail_closed() {
        let root = tempdir().expect("temporary root");
        let hash_path = root.path().join("hash.db");
        let connection = Connection::open(&hash_path).expect("fixture database");
        connection.execute_batch(SCHEMA).expect("fixture schema");
        populate_valid_chain(&connection);
        connection
            .execute("UPDATE contribution SET hash=zeroblob(32) WHERE seq=1", [])
            .expect("break canonical hash");
        drop(connection);
        assert!(matches!(
            inspect_journal(&hash_path),
            Err(StorageError::InvalidChain { sequence: 1, .. })
        ));

        let payload_path = root.path().join("payload.db");
        let connection = Connection::open(&payload_path).expect("fixture database");
        connection.execute_batch(SCHEMA).expect("fixture schema");
        populate_valid_chain(&connection);
        connection
            .execute("UPDATE contribution SET payload=X'ff' WHERE seq=2", [])
            .expect("break payload commitment");
        drop(connection);
        assert!(matches!(
            inspect_journal(&payload_path),
            Err(StorageError::InvalidChain { sequence: 2, .. })
        ));
    }

    #[test]
    fn erased_payload_is_skipped_but_surviving_metadata_is_verified() {
        let root = tempdir().expect("temporary root");
        let path = root.path().join("erased.db");
        let connection = Connection::open(&path).expect("fixture database");
        connection.execute_batch(SCHEMA).expect("fixture schema");
        populate_valid_chain(&connection);
        connection
            .execute(
                "UPDATE contribution SET payload=X'', erased_at='2026-08-20T00:00:00.000Z' \
                 WHERE seq=2",
                [],
            )
            .expect("erase payload bytes");
        drop(connection);
        assert_eq!(
            inspect_journal(&path)
                .expect("verifiable erasure")
                .contribution_count,
            2
        );
    }

    #[test]
    fn checkpoint_verifies_only_the_suffix_and_refuses_a_stale_anchor() {
        let root = tempdir().expect("temporary root");
        let path = root.path().join("checkpoint.db");
        let connection = Connection::open(&path).expect("fixture database");
        connection.execute_batch(SCHEMA).expect("fixture schema");
        populate_valid_chain(&connection);
        let first_hash: Vec<u8> = connection
            .query_row("SELECT hash FROM contribution WHERE seq=1", [], |row| {
                row.get(0)
            })
            .expect("first hash");
        drop(connection);

        let full = verify_journal_from(&path, None).expect("full verification");
        assert_eq!(full.verified_from, 0);
        assert_eq!(full.verified_through, 2);
        assert_eq!(full.content_verified, 2);
        assert!(!full.has_more);

        let first_page = verify_journal_page(&path, None, 1).expect("first page");
        assert_eq!(first_page.verified_through, 1);
        assert_eq!(first_page.content_verified, 1);
        assert!(first_page.has_more);
        let second_page =
            verify_journal_page(&path, Some(&first_page.checkpoint), 1).expect("second page");
        assert_eq!(second_page.verified_from, 1);
        assert_eq!(second_page.verified_through, 2);
        assert_eq!(second_page.content_verified, 1);
        assert!(!second_page.has_more);
        assert!(matches!(
            verify_journal_page(&path, None, 0),
            Err(StorageError::InvalidPageSize)
        ));

        let suffix = verify_journal_from(
            &path,
            Some(&JournalCheckpoint {
                sequence: 1,
                hash: first_hash,
            }),
        )
        .expect("suffix verification");
        assert_eq!(suffix.verified_from, 1);
        assert_eq!(suffix.verified_through, 2);
        assert_eq!(suffix.content_verified, 1);

        let at_head = verify_journal_from(&path, Some(&full.checkpoint)).expect("head checkpoint");
        assert_eq!(at_head.verified_from, 2);
        assert_eq!(at_head.verified_through, 2);
        assert_eq!(at_head.content_verified, 0);

        let stale = JournalCheckpoint {
            sequence: 2,
            hash: vec![0; 32],
        };
        assert!(matches!(
            verify_journal_from(&path, Some(&stale)),
            Err(StorageError::CheckpointMismatch { sequence: 2 })
        ));
    }

    #[test]
    fn paged_replay_respects_its_row_budget_across_a_larger_chain() {
        const ROWS: u64 = 513;
        const PAGE_SIZE: u64 = 64;

        let root = tempdir().expect("temporary root");
        let path = root.path().join("scale.db");
        let connection = Connection::open(&path).expect("fixture database");
        connection.execute_batch(SCHEMA).expect("fixture schema");
        populate_chain(&connection, ROWS);
        drop(connection);

        let full = verify_journal_from(&path, None).expect("full verification");
        let mut checkpoint = None;
        let mut verified = 0_u64;
        let mut pages = 0_u64;
        loop {
            let page = verify_journal_page(&path, checkpoint.as_ref(), PAGE_SIZE)
                .expect("bounded verification page");
            let page_rows = page.verified_through - page.verified_from;
            assert!(page_rows <= PAGE_SIZE);
            assert_eq!(page.content_verified, page_rows);
            verified += page_rows;
            pages += 1;
            checkpoint = Some(page.checkpoint);
            if !page.has_more {
                break;
            }
        }
        assert_eq!(verified, ROWS);
        assert_eq!(pages, ROWS.div_ceil(PAGE_SIZE));
        assert_eq!(checkpoint, Some(full.checkpoint));
    }
}
