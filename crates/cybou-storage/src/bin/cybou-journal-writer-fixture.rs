// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Write the differential fixture contributions through the Rust writer and dump every stored row.
//!
//! The Rust half of the writer oracle. It must produce output byte-identical to
//! `migration/oracles/journal_writer_fixture.cpp`, which performs the same three appends through
//! the predecessor Journal. The envelopes are restated here rather than shared, exactly as the
//! existing canonical oracles restate theirs: there is no way to share a literal across the two
//! languages, and a generated one would prove that the generator agrees with itself.
//!
//! The dump goes through `SQLite`'s own `quote()` so neither side formats anything of its own.

use std::fmt::Write as _;
use std::{env, path::PathBuf, process::ExitCode};

use cybou_protocol::admission::{Kind, Privacy, Sensitivity};
use cybou_protocol::canonical::CanonicalEnvelope;
use cybou_storage::writer::JournalWriter;
use uuid::Uuid;

/// The columns dumped, in a fixed order. Named explicitly rather than taken from the table so that
/// a column added by a future migration makes this fixture fail rather than silently widen.
const COLUMNS: &[&str] = &[
    "seq",
    "message_id",
    "correlation_id",
    "causation_id",
    "origin_organ",
    "origin_node",
    "kind",
    "wall_time",
    "monotonic_time",
    "logical_clock",
    "confidence",
    "evidence",
    "payload",
    "privacy",
    "capability",
    "schema_version",
    "hash_version",
    "prev_hash",
    "hash",
    "commitment",
    "payload_commitment",
    "erased_at",
    "sealed",
    "key_domain",
    "key_epoch",
    "retention_class",
    "retention_policy",
    "retain_until",
    "sensitivity",
];

/// 2026-08-19T08:15:30.125Z, the instant the canonical fixtures already use.
const WALL_TIME_MS: i64 = 1_787_127_330_125;
/// 2026-09-19T08:15:30.125Z.
const RETAIN_UNTIL_MS: u64 = 1_789_805_730_125;

fn id(text: &str) -> Uuid {
    Uuid::parse_str(text).expect("fixture identity")
}

/// The first observation. A root kind: no cause, no evidence, unbounded retention.
fn first() -> CanonicalEnvelope {
    CanonicalEnvelope {
        schema_version: 2,
        message_id: id("11111111-1111-4111-8111-111111111111"),
        correlation_id: id("22222222-2222-4222-8222-222222222222"),
        causation_id: Uuid::nil(),
        origin_organ: "perceptiond".into(),
        origin_node: String::new(),
        kind: Kind::Observation as u16,
        wall_time_ms: WALL_TIME_MS,
        monotonic_time: 123,
        logical_clock: 1,
        confidence: 1.0,
        evidence: Vec::new(),
        payload: vec![0xa1, 0x61, 0x78, 0x01],
        privacy: Privacy::Local as u8,
        capability_scope: String::new(),
        sealed: false,
        key_domain_id: Uuid::nil(),
        key_epoch: 0,
        retention_class: 2,
        retention_policy_version: 1,
        retain_until_ms: 0,
        sensitivity: Sensitivity::Ordinary as u8,
    }
}

/// A second observation carrying the values the first leaves at their defaults.
fn second() -> CanonicalEnvelope {
    CanonicalEnvelope {
        message_id: id("33333333-3333-4333-8333-333333333333"),
        origin_node: "local".into(),
        logical_clock: 2,
        confidence: 0.75,
        payload: vec![0xa2, 0x61, 0x78, 0x01, 0x61, 0x79, 0x02],
        privacy: Privacy::Node as u8,
        capability_scope: "mind.perception.read".into(),
        retention_class: 3,
        retention_policy_version: 2,
        retain_until_ms: RETAIN_UNTIL_MS,
        ..first()
    }
}

/// A derived contribution citing both, so the evidence join table and its ordinals are exercised.
fn third() -> CanonicalEnvelope {
    CanonicalEnvelope {
        message_id: id("44444444-4444-4444-8444-444444444444"),
        kind: Kind::Learning as u16,
        causation_id: id("11111111-1111-4111-8111-111111111111"),
        evidence: vec![id("33333333-3333-4333-8333-333333333333")],
        origin_organ: "selfd".into(),
        logical_clock: 3,
        confidence: 0.5,
        payload: vec![0xa1, 0x61, 0x7a, 0x03],
        privacy: Privacy::Local as u8,
        retention_class: 3,
        retention_policy_version: 2,
        retain_until_ms: RETAIN_UNTIL_MS,
        ..first()
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(dump) => {
            print!("{dump}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("cybou-journal-writer-fixture: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, String> {
    let arguments = env::args_os().skip(1).collect::<Vec<_>>();
    if arguments.len() != 1 {
        return Err("usage: cybou-journal-writer-fixture DIRECTORY".into());
    }
    let path = PathBuf::from(&arguments[0]).join("journal.db");
    if path.exists() {
        return Err(format!("{} already exists", path.display()));
    }

    let mut writer = JournalWriter::open(&path).map_err(|error| error.to_string())?;
    for envelope in [first(), second(), third()] {
        writer
            .append(&envelope)
            .map_err(|error| format!("append refused: {error}"))?;
    }

    dump(&path)
}

fn dump(path: &std::path::Path) -> Result<String, String> {
    let connection = rusqlite::Connection::open(path).map_err(|error| error.to_string())?;
    let quoted = COLUMNS
        .iter()
        .map(|column| format!("quote({column})"))
        .collect::<Vec<_>>()
        .join(", ");

    let mut out = String::new();
    let mut statement = connection
        .prepare(&format!("SELECT {quoted} FROM contribution ORDER BY seq"))
        .map_err(|error| error.to_string())?;
    let mut rows = statement.query([]).map_err(|error| error.to_string())?;
    let mut index = 0_usize;
    while let Some(row) = rows.next().map_err(|error| error.to_string())? {
        index += 1;
        for (column, name) in COLUMNS.iter().enumerate() {
            let value: String = row.get(column).map_err(|error| error.to_string())?;
            writeln!(out, "row.{index}.{name}={value}").map_err(|error| error.to_string())?;
        }
    }

    let mut statement = connection
        .prepare(
            "SELECT quote(contribution_id), quote(evidence_id), quote(ordinal) \
             FROM contribution_evidence ORDER BY contribution_id, ordinal",
        )
        .map_err(|error| error.to_string())?;
    let mut rows = statement.query([]).map_err(|error| error.to_string())?;
    let mut link = 0_usize;
    while let Some(row) = rows.next().map_err(|error| error.to_string())? {
        link += 1;
        let contribution: String = row.get(0).map_err(|error| error.to_string())?;
        let evidence: String = row.get(1).map_err(|error| error.to_string())?;
        let ordinal: String = row.get(2).map_err(|error| error.to_string())?;
        writeln!(out, "evidence.{link}={contribution} {evidence} {ordinal}")
            .map_err(|error| error.to_string())?;
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{dump, first, second, third};
    use cybou_storage::writer::JournalWriter;

    /// The Rust half of the writer differential, guarded where every developer runs it.
    ///
    /// The Debian gate compares this dump against the predecessor's. That gate needs Qt, SQL
    /// drivers, and libsodium, so it runs in one place; without this test a change to what Rust
    /// stores would stay invisible until it reached that host. Here it fails immediately, and the
    /// recorded fixture makes the change visible in review rather than only in a red gate.
    #[test]
    fn the_recorded_dump_is_what_the_writer_stores() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("journal.db");
        {
            let mut writer = JournalWriter::open(&path).expect("open");
            for envelope in [first(), second(), third()] {
                writer.append(&envelope).expect("append");
            }
        }

        let produced = dump(&path).expect("dump");
        let recorded = include_str!("../../../../fixtures/storage/journal-writer-v3.txt");
        assert_eq!(
            produced.replace("\r\n", "\n"),
            recorded.replace("\r\n", "\n")
        );
    }
}
