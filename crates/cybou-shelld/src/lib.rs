// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! CYBOU Bounded Body Shell capability engine and daemon.
//!
//! Exposes a bounded, sandboxed execution environment on the Debian 13 Body host.
//! Commands are strictly typed and parsed into explicit capabilities isolated within
//! a [`cybou_jailfs::JailFs`] sandbox root.

pub mod engine;
pub mod types;

pub use engine::ShellEngine;
pub use types::{MAX_FILE_PAYLOAD_BYTES, MAX_OUTPUT_BYTES, ShellError, ShellOutput};

#[cfg(test)]
mod tests {
    use cybou_jailfs::JailFs;
    use std::path::PathBuf;

    use super::*;

    fn test_engine() -> (ShellEngine, PathBuf) {
        let unique = format!(
            "cybou_shell_test_{}_{}",
            std::process::id(),
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        );
        let path = std::env::temp_dir().join(unique);
        let jail = JailFs::new(&path).expect("create test jail");
        let _ = jail.write_bytes("/welcome.txt", b"Welcome to CYBOU Bounded Shell", 1024);
        (ShellEngine::new(jail), path)
    }

    #[test]
    fn shell_executes_basic_workflow() {
        let (mut engine, dir) = test_engine();

        // 1. pwd
        let out = engine.execute("pwd");
        assert_eq!(out.exit_code, 0);
        assert_eq!(out.stdout.trim(), "/");

        // 2. ls
        let out = engine.execute("ls");
        assert_eq!(out.exit_code, 0);
        assert!(out.stdout.contains("welcome.txt"));

        // 3. cat
        let out = engine.execute("cat welcome.txt");
        assert_eq!(out.exit_code, 0);
        assert_eq!(out.stdout.trim(), "Welcome to CYBOU Bounded Shell");

        // 4. echo
        let out = engine.execute("echo Hello Cybou");
        assert_eq!(out.exit_code, 0);
        assert_eq!(out.stdout.trim(), "Hello Cybou");

        // 5. stat
        let out = engine.execute("stat welcome.txt");
        assert_eq!(out.exit_code, 0);
        assert!(out.stdout.contains("regular file"));

        // 6. head & tail
        let out = engine.execute("head -n 1 welcome.txt");
        assert_eq!(out.exit_code, 0);
        assert!(out.stdout.contains("Welcome"));

        let out = engine.execute("tail -n 1 welcome.txt");
        assert_eq!(out.exit_code, 0);
        assert!(out.stdout.contains("Welcome"));

        // 7. grep
        let out = engine.execute("grep -i Bounded welcome.txt");
        assert_eq!(out.exit_code, 0);
        assert!(out.stdout.contains("Welcome to CYBOU Bounded Shell"));

        // 8. help
        let out = engine.execute("help");
        assert_eq!(out.exit_code, 0);
        assert!(out.stdout.contains("pwd"));
        assert!(out.stdout.contains("grep"));

        // 9. clear
        let out = engine.execute("clear");
        assert_eq!(out.exit_code, 0);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn counting_reports_what_the_file_holds() {
        let (mut engine, dir) = test_engine();
        let out = engine.execute("wc welcome.txt");
        assert_eq!(out.exit_code, 0);
        // "Welcome to CYBOU Bounded Shell" — one line, five words, thirty bytes.
        let fields: Vec<&str> = out.stdout.split_whitespace().collect();
        assert_eq!(fields[0], "1");
        assert_eq!(fields[1], "5");
        assert_eq!(fields[2], "30");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn file_says_what_it_checked_and_nothing_more() {
        // Two answers, because two things were checked: whether it is a directory, and whether the
        // bytes are UTF-8. No format guessed from a name or a magic number.
        let (mut engine, dir) = test_engine();
        assert!(
            engine
                .execute("file welcome.txt")
                .stdout
                .contains("UTF-8 text")
        );
        assert!(engine.execute("file /").stdout.contains("directory"));
        assert_eq!(engine.execute("file nothing-here").exit_code, 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_walk_that_stopped_early_says_so() {
        // Partial is not a smaller directory. `find` and `du` are bounded, and a bounded answer
        // that read like a complete one would be the surface stating something it did not
        // establish.
        let (mut engine, dir) = test_engine();
        let deep = (0..crate::types::MAX_WALK_DEPTH + 2)
            .map(|level| format!("level{level}"))
            .collect::<Vec<_>>()
            .join("/");
        std::fs::create_dir_all(dir.join(&deep)).expect("a deep tree");

        let out = engine.execute("find /");
        assert_eq!(out.exit_code, 0);
        assert!(
            out.stdout.contains("partial"),
            "a truncated walk did not say so: {}",
            out.stdout
        );

        let total = engine.execute("du /");
        assert!(total.stdout.contains("partial"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_clock_is_read_rather_than_recited() {
        // `uname` was withdrawn for answering with something compiled in. A date that did the same
        // would be the same fault under another name.
        let (mut engine, dir) = test_engine();
        let first = engine.execute("date");
        assert_eq!(first.exit_code, 0);
        let year = time::OffsetDateTime::now_utc().year().to_string();
        assert!(
            first.stdout.starts_with(&year),
            "date did not report the current year: {}",
            first.stdout
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_builtin_that_could_only_lie_is_not_offered_at_all() {
        // `whoami` answered "cybou" and `uname -a` answered a fixed kernel string, on every host,
        // regardless of who was asking or what the machine was. In a terminal those read as
        // observations of the Body. Removing them is the honest fix; a bounded surface may be
        // small, but nothing it prints may be invented.
        let (mut engine, dir) = test_engine();

        for pretending in ["whoami", "uname", "uname -a"] {
            let out = engine.execute(pretending);
            assert_eq!(
                out.exit_code, 127,
                "{pretending} answered instead of refusing"
            );
        }

        let help = engine.execute("help");
        assert!(!help.stdout.contains("whoami"));
        assert!(!help.stdout.contains("uname"));

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_long_listing_states_only_what_the_sandbox_established() {
        // The old column was `-rwxr-xr-x 1 cybou cybou` for every entry: a mode nobody read and an
        // owner nobody looked up, in the position a person reads as fact.
        let (mut engine, dir) = test_engine();

        let out = engine.execute("ls -l");
        assert_eq!(out.exit_code, 0);
        assert!(out.stdout.contains("welcome.txt"));
        assert!(
            !out.stdout.contains("rwx"),
            "a permission bit nobody established was printed: {}",
            out.stdout
        );
        assert!(
            !out.stdout.contains("cybou cybou"),
            "an owner nobody looked up was printed: {}",
            out.stdout
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn stat_reports_the_size_it_read_and_no_mode_it_did_not() {
        let (mut engine, dir) = test_engine();

        let out = engine.execute("stat welcome.txt");
        assert_eq!(out.exit_code, 0);
        assert!(out.stdout.contains("regular file"));
        assert!(
            !out.stdout.contains("Access:"),
            "a constant access mode was printed as a fact: {}",
            out.stdout
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn shell_blocks_unauthorized_commands_and_traversals() {
        let (mut engine, dir) = test_engine();

        // Disallowed / unauthorized commands (write, rm, sudo, curl)
        let out = engine.execute("touch test.txt");
        assert_eq!(out.exit_code, 127);
        assert!(out.stderr.contains("command not found"));

        let out = engine.execute("write test.txt hello");
        assert_eq!(out.exit_code, 127);
        assert!(out.stderr.contains("command not found"));

        let out = engine.execute("sudo rm -rf /");
        assert_eq!(out.exit_code, 127);
        assert!(out.stderr.contains("command not found"));

        let out = engine.execute("curl https://example.com");
        assert_eq!(out.exit_code, 127);
        assert!(out.stderr.contains("command not found"));

        // Path traversal in cat
        let out = engine.execute("cat ../../etc/shadow");
        assert_eq!(out.exit_code, 1);
        assert!(out.stderr.contains("sandbox path traversal"));

        // Path traversal in stat
        let out = engine.execute("stat ../../etc/passwd");
        assert_eq!(out.exit_code, 1);
        assert!(out.stderr.contains("sandbox path traversal"));

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn tokenize_handles_quotes_and_escapes() {
        let tokens = ShellEngine::tokenize("cat \"hello world.txt\" 'foo bar' simple\\ name");
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0], "cat");
        assert_eq!(tokens[1], "hello world.txt");
        assert_eq!(tokens[2], "foo bar");
        assert_eq!(tokens[3], "simple name");
    }
}
