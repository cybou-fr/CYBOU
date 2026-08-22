// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Shell execution types, error representations, and payload limits.

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
