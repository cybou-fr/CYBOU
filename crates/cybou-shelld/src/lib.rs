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
    use std::path::PathBuf;
    use cybou_jailfs::JailFs;

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

        // 5. whoami
        let out = engine.execute("whoami");
        assert_eq!(out.exit_code, 0);
        assert_eq!(out.stdout.trim(), "cybou");

        // 6. uname
        let out = engine.execute("uname -a");
        assert_eq!(out.exit_code, 0);
        assert!(out.stdout.contains("Linux"));
        assert!(out.stdout.contains("x86_64"));

        // 7. stat
        let out = engine.execute("stat welcome.txt");
        assert_eq!(out.exit_code, 0);
        assert!(out.stdout.contains("regular file"));

        // 8. head & tail
        let out = engine.execute("head -n 1 welcome.txt");
        assert_eq!(out.exit_code, 0);
        assert!(out.stdout.contains("Welcome"));

        let out = engine.execute("tail -n 1 welcome.txt");
        assert_eq!(out.exit_code, 0);
        assert!(out.stdout.contains("Welcome"));

        // 9. grep
        let out = engine.execute("grep -i Bounded welcome.txt");
        assert_eq!(out.exit_code, 0);
        assert!(out.stdout.contains("Welcome to CYBOU Bounded Shell"));

        // 10. help
        let out = engine.execute("help");
        assert_eq!(out.exit_code, 0);
        assert!(out.stdout.contains("pwd"));
        assert!(out.stdout.contains("grep"));

        // 11. clear
        let out = engine.execute("clear");
        assert_eq!(out.exit_code, 0);

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
