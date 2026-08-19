// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Measure the Rust Journal paths that grow with history.
//!
//! The Rust counterpart of the predecessor's `journal-scale`, against the same budgets in
//! `docs/mind/SCALE_BUDGETS.md`. Those numbers were measured through the C++ Journal; until the
//! same paths are measured through this writer, the Rust stack has budgets it has never been held
//! to.
//!
//! The fixture is deterministic: the same index always produces the same envelope, so two runs
//! build identical journals and a measured difference is a real one.
//!
//! Size is set by `CYBOU_SCALE_CONTRIBUTIONS` and defaults small enough for the ordinary checks.
//! The fixture is built through `append_batch`, which shares one commit — and therefore one fsync
//! — across a batch. Append is measured separately, one contribution per transaction, exactly as
//! Event1 accepts them: building the whole fixture at production durability would take hours at a
//! million rows and would measure nothing the append sample does not already measure honestly.

use std::time::Instant;
use std::{env, path::Path, process::ExitCode};

use cybou_protocol::admission::{Kind, Privacy, Sensitivity};
use cybou_protocol::canonical::CanonicalEnvelope;
use cybou_storage::writer::JournalWriter;
use cybou_storage::{verify_journal_from, verify_journal_page};
use uuid::Uuid;

/// Contributions per shared commit while building the fixture.
const BATCH: u64 = 1_000;
/// Rows per page for the paged-replay measurement.
const PAGE: u64 = 1_000;
/// How many contributions are appended one fsync at a time to measure real acceptance cost.
const APPEND_SAMPLE: u64 = 50;
/// Default fixture size, chosen so this runs inside the ordinary checks.
const DEFAULT_CONTRIBUTIONS: u64 = 10_000;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("cybou-journal-scale: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let contributions = match env::var("CYBOU_SCALE_CONTRIBUTIONS") {
        Ok(value) => value
            .parse::<u64>()
            .map_err(|_| format!("CYBOU_SCALE_CONTRIBUTIONS is not a count: {value}"))?,
        Err(_) => DEFAULT_CONTRIBUTIONS,
    };
    if contributions == 0 {
        return Err("a scale run needs at least one contribution".into());
    }

    let arguments = env::args_os().skip(1).collect::<Vec<_>>();
    if arguments.len() != 1 {
        return Err("usage: cybou-journal-scale DIRECTORY".into());
    }
    let path = Path::new(&arguments[0]).join("journal.db");
    if path.exists() {
        return Err(format!("{} already exists", path.display()));
    }
    let measured = measure(&path, contributions)?;

    println!("contributions={contributions}");
    for (name, value) in measured {
        println!("{name}={value}");
    }
    Ok(())
}

/// One deterministic contribution. Root kind, so no reference resolution is measured with it: this
/// gauges the write path, not the admission graph.
fn envelope(index: u64) -> CanonicalEnvelope {
    let mut bytes = [0_u8; 16];
    bytes[..8].copy_from_slice(&index.to_be_bytes());
    bytes[8] = 0x40; // A stable version nibble; the writer only needs the identity to be non-nil.
    CanonicalEnvelope {
        schema_version: 2,
        message_id: Uuid::from_bytes(bytes),
        correlation_id: Uuid::from_bytes([0x22; 16]),
        causation_id: Uuid::nil(),
        origin_organ: "perceptiond".into(),
        origin_node: String::new(),
        kind: Kind::Observation as u16,
        wall_time_ms: 1_787_127_330_125 + i64::try_from(index).unwrap_or(0),
        monotonic_time: index,
        logical_clock: index,
        confidence: 1.0,
        evidence: Vec::new(),
        // Roughly the shape of a real observation payload rather than a token byte: row size is
        // one of the measured quantities, and a two-byte payload would flatter it.
        payload: vec![0xa1, 0x61, 0x78, 0x1a]
            .into_iter()
            .chain(index.to_be_bytes())
            .chain(std::iter::repeat_n(0x20, 96))
            .collect(),
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

fn measure(path: &Path, contributions: u64) -> Result<Vec<(String, String)>, String> {
    let mut writer = JournalWriter::open(path).map_err(|error| error.to_string())?;

    let started = Instant::now();
    let mut written = 0_u64;
    while written < contributions {
        let end = (written + BATCH).min(contributions);
        let batch: Vec<CanonicalEnvelope> = (written..end).map(envelope).collect();
        writer
            .append_batch(&batch)
            .map_err(|error| format!("fixture build refused: {error}"))?;
        written = end;
    }
    let build = started.elapsed();

    // Acceptance cost, one commit each, on top of the fixture that already exists: this is what an
    // Event1 Submit actually pays once the history is large.
    let started = Instant::now();
    for index in contributions..contributions + APPEND_SAMPLE {
        writer
            .append(&envelope(index))
            .map_err(|error| format!("append refused: {error}"))?;
    }
    let append = started.elapsed();
    let total = contributions + APPEND_SAMPLE;
    drop(writer);

    let started = Instant::now();
    let full = verify_journal_from(path, None).map_err(|error| error.to_string())?;
    let verify = started.elapsed();
    if full.verified_through != total {
        return Err(format!(
            "full verification stopped at {} of {total}",
            full.verified_through
        ));
    }

    let started = Instant::now();
    let mut checkpoint = None;
    let mut pages = 0_u64;
    loop {
        let page =
            verify_journal_page(path, checkpoint.as_ref(), PAGE).map_err(|e| e.to_string())?;
        pages += 1;
        if !page.has_more {
            break;
        }
        checkpoint = Some(page.checkpoint);
    }
    let paged_elapsed = started.elapsed();

    let bytes = std::fs::metadata(path)
        .map_err(|error| error.to_string())?
        .len();

    Ok(vec![
        ("build_ms".into(), build.as_millis().to_string()),
        (
            "build_us_per_contribution".into(),
            per(build, contributions),
        ),
        ("append_sample".into(), APPEND_SAMPLE.to_string()),
        ("append_us_each".into(), per(append, APPEND_SAMPLE)),
        ("verify_ms".into(), verify.as_millis().to_string()),
        ("verify_us_per_row".into(), per(verify, total)),
        (
            "paged_verify_ms".into(),
            paged_elapsed.as_millis().to_string(),
        ),
        ("paged_verify_pages".into(), pages.to_string()),
        ("bytes".into(), bytes.to_string()),
        ("bytes_per_contribution".into(), (bytes / total).to_string()),
    ])
}

fn per(elapsed: std::time::Duration, count: u64) -> String {
    if count == 0 {
        return "0".into();
    }
    (elapsed.as_micros() / u128::from(count)).to_string()
}

#[cfg(test)]
mod tests {
    use super::{envelope, measure};
    use cybou_protocol::canonical::canonical_envelope_v2;

    /// A small run, so the measurement path itself is covered by the ordinary suite.
    ///
    /// Deliberately no assertion about how long anything took. A timing threshold in a unit test
    /// fails on a loaded machine and teaches everyone to rerun until it passes; the budgets in
    /// `docs/mind/SCALE_BUDGETS.md` are compared by a person reading a scale run, not by this.
    #[test]
    fn a_small_scale_run_measures_every_path() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let measured = measure(&directory.path().join("journal.db"), 200).expect("measure");
        let names: Vec<&str> = measured.iter().map(|(name, _)| name.as_str()).collect();
        assert!(names.contains(&"append_us_each"));
        assert!(names.contains(&"verify_us_per_row"));
        assert!(names.contains(&"bytes_per_contribution"));
    }

    #[test]
    fn the_fixture_is_deterministic() {
        assert_eq!(
            canonical_envelope_v2(&envelope(7)),
            canonical_envelope_v2(&envelope(7))
        );
        assert_ne!(envelope(7).message_id, envelope(8).message_id);
    }
}
