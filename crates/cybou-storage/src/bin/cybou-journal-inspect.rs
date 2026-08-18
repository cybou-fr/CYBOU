// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! One-page, read-only verifier for an existing predecessor Journal.

use std::{env, ffi::OsString, path::PathBuf, process::ExitCode};

use cybou_storage::{JournalCheckpoint, verify_journal_page};

fn main() -> ExitCode {
    let arguments = env::args_os().skip(1).collect::<Vec<_>>();
    match run(&arguments) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("cybou-journal-inspect: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: &[OsString]) -> Result<(), String> {
    if !matches!(arguments.len(), 2 | 4) {
        return Err(usage());
    }
    let path = PathBuf::from(&arguments[0]);
    let max_rows = parse_u64(&arguments[1], "MAX_ROWS")?;
    let checkpoint = if arguments.len() == 4 {
        Some(JournalCheckpoint {
            sequence: parse_u64(&arguments[2], "SEQUENCE")?,
            hash: parse_hash(&arguments[3])?,
        })
    } else {
        None
    };
    let verification = verify_journal_page(&path, checkpoint.as_ref(), max_rows)
        .map_err(|error| error.to_string())?;
    println!("verified_from={}", verification.verified_from);
    println!("verified_through={}", verification.verified_through);
    println!("content_verified={}", verification.content_verified);
    println!("content_skipped={}", verification.content_skipped);
    println!("has_more={}", verification.has_more);
    println!("checkpoint_sequence={}", verification.checkpoint.sequence);
    println!(
        "checkpoint_hash={}",
        encode_hex(&verification.checkpoint.hash)
    );
    Ok(())
}

fn parse_u64(value: &OsString, name: &str) -> Result<u64, String> {
    value
        .to_str()
        .ok_or_else(|| format!("{name} is not UTF-8"))?
        .parse()
        .map_err(|_| format!("{name} is not an unsigned integer"))
}

fn parse_hash(value: &OsString) -> Result<Vec<u8>, String> {
    let value = value
        .to_str()
        .ok_or_else(|| "HASH_HEX is not UTF-8".to_owned())?;
    if value.len() != 64 || !value.is_ascii() {
        return Err("HASH_HEX must contain exactly 64 hexadecimal characters".into());
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            std::str::from_utf8(pair)
                .ok()
                .and_then(|pair| u8::from_str_radix(pair, 16).ok())
                .ok_or_else(|| "HASH_HEX contains a non-hexadecimal character".to_owned())
        })
        .collect()
}

fn encode_hex(value: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(value.len() * 2);
    for byte in value {
        encoded.push(char::from(DIGITS[usize::from(byte >> 4)]));
        encoded.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    encoded
}

fn usage() -> String {
    "usage: cybou-journal-inspect PATH MAX_ROWS [SEQUENCE HASH_HEX]".into()
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::{encode_hex, parse_hash};

    #[test]
    fn checkpoint_hash_round_trips_and_malformed_input_is_rejected() {
        let bytes = [0x5a; 32];
        let encoded = encode_hex(&bytes);
        assert_eq!(parse_hash(&OsString::from(encoded)).unwrap(), bytes);
        assert!(parse_hash(&OsString::from("00")).is_err());
        assert!(parse_hash(&OsString::from("z".repeat(64))).is_err());
    }
}
