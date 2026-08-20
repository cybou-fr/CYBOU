// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! CYBOU Bounded Body Shell capability engine and daemon.
//!
//! Exposes a bounded, sandboxed execution environment on the Debian 13 Body host.
//! Commands are strictly typed and parsed into explicit capabilities isolated within
//! a [`cybou_jailfs::JailFs`] sandbox root.

use std::fmt::Write as _;

use cybou_jailfs::{JailError, JailFs};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Maximum stdout buffer size (64 KB).
pub const MAX_OUTPUT_BYTES: usize = 65_536;

/// Maximum file read/write payload size in single command (1 MB).
pub const MAX_FILE_PAYLOAD_BYTES: usize = 1_048_576;

/// Error returned by Shell capability execution.
#[derive(Clone, Debug, Eq, Error, PartialEq, Serialize, Deserialize)]
pub enum ShellError {
    /// Command syntax or arguments are invalid.
    #[error("command error: {0}")]
    InvalidCommand(String),
    /// Command is unrecognized or forbidden.
    #[error("command not found: '{0}'. Type 'help' for available commands.")]
    CommandNotFound(String),
    /// Sandboxed filesystem violation.
    #[error("filesystem sandbox violation: {0}")]
    Sandbox(String),
    /// Execution output exceeded capacity limit.
    #[error("output limit exceeded: {0}")]
    OutputLimitExceeded(String),
}

/// Result of executing a bounded Shell command.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ShellOutput {
    /// Standard exit code (0 for success).
    pub exit_code: i32,
    /// Standard output text.
    pub stdout: String,
    /// Standard error text.
    pub stderr: String,
    /// Current working directory inside the sandbox after command execution.
    pub cwd: String,
}

impl ShellOutput {
    /// Create a successful output result.
    #[must_use]
    pub fn success(stdout: impl Into<String>, cwd: impl Into<String>) -> Self {
        Self {
            exit_code: 0,
            stdout: stdout.into(),
            stderr: String::new(),
            cwd: cwd.into(),
        }
    }

    /// Create an error output result.
    #[must_use]
    pub fn error(exit_code: i32, stderr: impl Into<String>, cwd: impl Into<String>) -> Self {
        Self {
            exit_code,
            stdout: String::new(),
            stderr: stderr.into(),
            cwd: cwd.into(),
        }
    }
}

/// Interactive Shell state with virtual working directory and sandboxed filesystem.
#[derive(Clone, Debug)]
pub struct ShellEngine {
    jail: JailFs,
    virtual_cwd: String,
}

impl ShellEngine {
    /// Initialize a new shell engine bound to a sandbox root.
    #[must_use]
    pub const fn new(jail: JailFs) -> Self {
        Self {
            jail,
            virtual_cwd: String::new(),
        }
    }

    /// Return the current virtual working directory inside the sandbox.
    #[must_use]
    pub fn cwd(&self) -> &str {
        if self.virtual_cwd.is_empty() {
            "/"
        } else {
            &self.virtual_cwd
        }
    }

    /// Return a reference to the underlying sandbox filesystem.
    #[must_use]
    pub const fn jail(&self) -> &JailFs {
        &self.jail
    }

    /// Parse and execute a command string.
    #[must_use]
    pub fn execute(&mut self, command_line: &str) -> ShellOutput {
        let trimmed = command_line.trim();
        if trimmed.is_empty() {
            return ShellOutput::success(String::new(), self.cwd());
        }

        let tokens = Self::tokenize(trimmed);
        if tokens.is_empty() {
            return ShellOutput::success(String::new(), self.cwd());
        }

        let cmd = tokens[0].to_ascii_lowercase();
        let args = &tokens[1..];

        match cmd.as_str() {
            "pwd" => self.exec_pwd(),
            "cd" => self.exec_cd(args),
            "ls" => self.exec_ls(args),
            "cat" => self.exec_cat(args),
            "echo" => self.exec_echo(args),
            "grep" => self.exec_grep(args),
            "mkdir" => self.exec_mkdir(args),
            "touch" => self.exec_touch(args),
            "write" => self.exec_write(args),
            "status" => self.exec_status(),
            "help" => self.exec_help(args),
            "clear" => ShellOutput::success("\x1b[2J\x1b[H", self.cwd()),
            unknown => ShellOutput::error(
                127,
                format!(
                    "cybou: command not found: '{unknown}'. Type 'help' for available capabilities.\n"
                ),
                self.cwd(),
            ),
        }
    }

    fn resolve_path(&self, rel_or_abs: &str) -> String {
        if rel_or_abs.starts_with('/') {
            rel_or_abs.to_owned()
        } else if self.virtual_cwd.is_empty() || self.virtual_cwd == "/" {
            format!("/{rel_or_abs}")
        } else {
            format!("{}/{rel_or_abs}", self.virtual_cwd)
        }
    }

    fn exec_pwd(&self) -> ShellOutput {
        ShellOutput::success(format!("{}\n", self.cwd()), self.cwd())
    }

    fn exec_cd(&mut self, args: &[String]) -> ShellOutput {
        let target = if args.is_empty() || args[0] == "~" {
            "/".to_owned()
        } else {
            args[0].clone()
        };

        if target == ".." || target.contains("..") {
            // Check if user is navigating up
            if self.virtual_cwd.is_empty() || self.virtual_cwd == "/" {
                return ShellOutput::success(String::new(), "/");
            }
            let mut parts: Vec<&str> = self
                .virtual_cwd
                .trim_start_matches('/')
                .split('/')
                .collect();
            parts.pop();
            let new_cwd = if parts.is_empty() || parts[0].is_empty() {
                "/".to_owned()
            } else {
                format!("/{}", parts.join("/"))
            };
            self.virtual_cwd = new_cwd;
            return ShellOutput::success(String::new(), self.cwd());
        }

        let full_path = self.resolve_path(&target);
        match self.jail.resolve(&full_path) {
            Ok(resolved) => {
                if resolved.is_dir() {
                    let clean = full_path.replace('\\', "/");
                    self.virtual_cwd = if clean.is_empty() {
                        "/".to_owned()
                    } else {
                        clean
                    };
                    ShellOutput::success(String::new(), self.cwd())
                } else if resolved.exists() {
                    ShellOutput::error(1, format!("cd: not a directory: {target}\n"), self.cwd())
                } else {
                    ShellOutput::error(
                        1,
                        format!("cd: no such file or directory: {target}\n"),
                        self.cwd(),
                    )
                }
            }
            Err(JailError::TraversalAttempt(_)) => {
                ShellOutput::error(1, "cd: sandbox path traversal violation\n", self.cwd())
            }
            Err(e) => ShellOutput::error(1, format!("cd: {e}\n"), self.cwd()),
        }
    }

    fn exec_ls(&self, args: &[String]) -> ShellOutput {
        let mut show_long = false;
        let mut target_dir = self.cwd().to_owned();

        for arg in args {
            if arg == "-l" || arg == "-la" || arg == "-al" {
                show_long = true;
            } else if !arg.starts_with('-') {
                target_dir = self.resolve_path(arg);
            }
        }

        match self.jail.list_dir(&target_dir) {
            Ok(entries) => {
                let mut out = String::new();
                for entry in entries {
                    if show_long {
                        let kind = if entry.is_dir { "d" } else { "-" };
                        let size = if entry.is_dir { 4096 } else { entry.size_bytes };
                        let _ = writeln!(
                            out,
                            "{kind}rwxr-xr-x 1 cybou cybou {size:>8} {}",
                            entry.name
                        );
                    } else {
                        let suffix = if entry.is_dir { "/" } else { "" };
                        let _ = write!(out, "{}{suffix}  ", entry.name);
                    }
                }
                if !show_long && !out.is_empty() {
                    out.push('\n');
                }
                ShellOutput::success(out, self.cwd())
            }
            Err(JailError::NotFound(p)) => ShellOutput::error(
                1,
                format!("ls: cannot access '{p}': No such file or directory\n"),
                self.cwd(),
            ),
            Err(e) => ShellOutput::error(1, format!("ls: {e}\n"), self.cwd()),
        }
    }

    fn exec_cat(&self, args: &[String]) -> ShellOutput {
        if args.is_empty() {
            return ShellOutput::error(1, "cat: missing file operand\n", self.cwd());
        }
        let full_path = self.resolve_path(&args[0]);
        match self.jail.read_to_string(&full_path, MAX_FILE_PAYLOAD_BYTES) {
            Ok(content) => ShellOutput::success(content, self.cwd()),
            Err(JailError::NotFound(p)) => ShellOutput::error(
                1,
                format!("cat: {p}: No such file or directory\n"),
                self.cwd(),
            ),
            Err(e) => ShellOutput::error(1, format!("cat: {e}\n"), self.cwd()),
        }
    }

    fn exec_echo(&self, args: &[String]) -> ShellOutput {
        let joined = args.join(" ");
        ShellOutput::success(format!("{joined}\n"), self.cwd())
    }

    fn exec_grep(&self, args: &[String]) -> ShellOutput {
        if args.len() < 2 {
            return ShellOutput::error(1, "grep: usage: grep <pattern> <file>\n", self.cwd());
        }
        let pattern = &args[0];
        let file_path = self.resolve_path(&args[1]);

        match self.jail.read_to_string(&file_path, MAX_FILE_PAYLOAD_BYTES) {
            Ok(content) => {
                let mut out = String::new();
                for (idx, line) in content.lines().enumerate() {
                    if line.contains(pattern) {
                        let _ = writeln!(out, "{}:{line}", idx + 1);
                    }
                }
                ShellOutput::success(out, self.cwd())
            }
            Err(e) => ShellOutput::error(1, format!("grep: {e}\n"), self.cwd()),
        }
    }

    fn exec_mkdir(&self, args: &[String]) -> ShellOutput {
        if args.is_empty() {
            return ShellOutput::error(1, "mkdir: missing directory operand\n", self.cwd());
        }
        let full_path = self.resolve_path(&args[0]);
        match self.jail.create_dir_all(&full_path) {
            Ok(()) => ShellOutput::success(String::new(), self.cwd()),
            Err(e) => ShellOutput::error(1, format!("mkdir: {e}\n"), self.cwd()),
        }
    }

    fn exec_touch(&self, args: &[String]) -> ShellOutput {
        if args.is_empty() {
            return ShellOutput::error(1, "touch: missing file operand\n", self.cwd());
        }
        let full_path = self.resolve_path(&args[0]);
        if self.jail.exists(&full_path) {
            return ShellOutput::success(String::new(), self.cwd());
        }
        match self
            .jail
            .write_bytes(&full_path, b"", MAX_FILE_PAYLOAD_BYTES)
        {
            Ok(()) => ShellOutput::success(String::new(), self.cwd()),
            Err(e) => ShellOutput::error(1, format!("touch: {e}\n"), self.cwd()),
        }
    }

    fn exec_write(&self, args: &[String]) -> ShellOutput {
        if args.len() < 2 {
            return ShellOutput::error(1, "write: usage: write <file> <content...>\n", self.cwd());
        }
        let file_path = self.resolve_path(&args[0]);
        let content = args[1..].join(" ");
        match self
            .jail
            .write_bytes(&file_path, content.as_bytes(), MAX_FILE_PAYLOAD_BYTES)
        {
            Ok(()) => ShellOutput::success(String::new(), self.cwd()),
            Err(e) => ShellOutput::error(1, format!("write: {e}\n"), self.cwd()),
        }
    }

    fn exec_status(&self) -> ShellOutput {
        let status = format!(
            "CYBOU Body Host: Debian 13 (Trixie)\n\
             Mind Architecture: Rust-First (ADR-0038/0039)\n\
             Desktop: CYBOU Desktop Spatial Cards (ADR-0040)\n\
             Sandbox Root: {}\n\
             Virtual CWD: {}\n\
             Security Zone: Zone 3 (Bounded Body Capabilities)\n",
            self.jail.root().display(),
            self.cwd()
        );
        ShellOutput::success(status, self.cwd())
    }

    fn exec_help(&self, args: &[String]) -> ShellOutput {
        if let Some(cmd) = args.first() {
            let desc = match cmd.as_str() {
                "pwd" => "pwd - print name of current virtual directory",
                "cd" => "cd <dir> - change virtual working directory inside sandbox",
                "ls" => "ls [-l] [dir] - list directory contents",
                "cat" => "cat <file> - concatenate and display file content",
                "echo" => "echo [text...] - display a line of text",
                "grep" => "grep <pattern> <file> - print lines matching a pattern",
                "mkdir" => "mkdir <dir> - create directory inside sandbox",
                "touch" => "touch <file> - change file timestamp or create empty file",
                "write" => "write <file> <content...> - write text into a sandboxed file",
                "status" => "status - show CYBOU Body host and sandbox status",
                "clear" => "clear - clear terminal screen buffer",
                "help" => "help [command] - display information about builtin commands",
                _ => "Unknown command. Type 'help' to list all commands.",
            };
            return ShellOutput::success(format!("{desc}\n"), self.cwd());
        }

        let help = "\
CYBOU Bounded Body Shell (Zone 3 capability exploration)
These shell commands are defined internally. Type 'help <command>' for details.

  pwd             Print working directory
  cd <dir>        Change directory inside sandbox
  ls [-l] [dir]   List directory contents
  cat <file>      Display file contents
  echo [text...]  Print text to output
  grep <pat> <f>  Search pattern in file
  mkdir <dir>     Create directory
  touch <file>    Create empty file
  write <f> <txt> Write text to file
  status          Display Body and Mind status
  clear           Clear screen
  help [cmd]      Display command help
";
        ShellOutput::success(help, self.cwd())
    }

    /// Split a command line string into tokens respecting single/double quotes.
    #[must_use]
    pub fn tokenize(input: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut in_single_quote = false;
        let mut in_double_quote = false;
        let mut chars = input.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '\'' if !in_double_quote => {
                    in_single_quote = !in_single_quote;
                }
                '"' if !in_single_quote => {
                    in_double_quote = !in_double_quote;
                }
                ' ' | '\t' if !in_single_quote && !in_double_quote => {
                    if !current.is_empty() {
                        tokens.push(current);
                        current = String::new();
                    }
                }
                '\\' if !in_single_quote => {
                    if let Some(next_ch) = chars.next() {
                        current.push(next_ch);
                    }
                }
                c => current.push(c),
            }
        }

        if !current.is_empty() {
            tokens.push(current);
        }

        tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_engine() -> (ShellEngine, PathBuf) {
        let unique = format!(
            "cybou_shell_test_{}_{}",
            std::process::id(),
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        );
        let path = std::env::temp_dir().join(unique);
        let jail = JailFs::new(&path).expect("create test jail");
        (ShellEngine::new(jail), path)
    }

    #[test]
    fn shell_executes_basic_workflow() {
        let (mut engine, dir) = test_engine();

        // 1. pwd
        let out = engine.execute("pwd");
        assert_eq!(out.exit_code, 0);
        assert_eq!(out.stdout.trim(), "/");

        // 2. mkdir and cd
        let out = engine.execute("mkdir workspace");
        assert_eq!(out.exit_code, 0);

        let out = engine.execute("cd workspace");
        assert_eq!(out.exit_code, 0);
        assert_eq!(engine.cwd(), "/workspace");

        // 3. write and cat
        let out = engine.execute("write notes.txt \"System check passed\"");
        assert_eq!(out.exit_code, 0);

        let out = engine.execute("cat notes.txt");
        assert_eq!(out.exit_code, 0);
        assert_eq!(out.stdout.trim(), "System check passed");

        // 4. grep
        let out = engine.execute("grep check notes.txt");
        assert_eq!(out.exit_code, 0);
        assert!(out.stdout.contains("System check passed"));

        // 5. ls -l
        let out = engine.execute("ls -l");
        assert_eq!(out.exit_code, 0);
        assert!(out.stdout.contains("notes.txt"));

        // 6. cd back
        let out = engine.execute("cd ..");
        assert_eq!(out.exit_code, 0);
        assert_eq!(engine.cwd(), "/");

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn shell_blocks_unauthorized_commands_and_traversals() {
        let (mut engine, dir) = test_engine();

        // Unknown command
        let out = engine.execute("sudo rm -rf /");
        assert_eq!(out.exit_code, 127);
        assert!(out.stderr.contains("command not found"));

        // Path traversal in cat
        let out = engine.execute("cat ../../etc/shadow");
        assert_eq!(out.exit_code, 1);
        assert!(out.stderr.contains("sandbox path traversal"));

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn tokenize_handles_quotes_and_escapes() {
        let tokens = ShellEngine::tokenize("write \"hello world.txt\" 'foo bar' simple\\ name");
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0], "write");
        assert_eq!(tokens[1], "hello world.txt");
        assert_eq!(tokens[2], "foo bar");
        assert_eq!(tokens[3], "simple name");
    }
}
