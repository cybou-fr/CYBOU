// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Universal Workspace and Authority Domain references (ADR-0045).

use serde::{Deserialize, Serialize};

/// Typed authority domain and workspace location reference.
///
/// Distinguishes between standard user files, privileged system files requiring
/// Action1 `FileWrite` proposals, isolated agent capsule sandboxes, bounded demo jails,
/// and immutable backup snapshots.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "target")]
pub enum LocationRef {
    /// Standard host user directory or file (e.g. `/home/user/...`).
    HostUserPath(String),
    /// Privileged system configuration file (e.g. `/etc/nginx/nginx.conf`).
    /// Direct browser writes are forbidden; saves must route through Action1 `FileWrite` proposals.
    SystemConfigPath(String),
    /// Ephemeral isolated agent capsule sandbox.
    AgentWorkspace {
        /// Capsule or session identifier.
        capsule_id: String,
        /// Relative path inside the capsule jail.
        relative_path: String,
    },
    /// Bounded safe shell environment jail.
    SafeShellJail {
        /// Session identifier.
        session_id: String,
        /// Path within the safe jail.
        path: String,
    },
    /// Read-only historical snapshot.
    BackupSnapshot {
        /// Snapshot identifier or timestamp.
        snapshot_id: String,
        /// Path inside the snapshot tree.
        path: String,
    },
}

impl LocationRef {
    /// Return the user-facing display path or identifier.
    #[must_use]
    pub fn display_path(&self) -> String {
        match self {
            Self::HostUserPath(p) | Self::SystemConfigPath(p) => p.clone(),
            Self::AgentWorkspace {
                capsule_id,
                relative_path,
            } => {
                format!("agent://{capsule_id}/{relative_path}")
            }
            Self::SafeShellJail { session_id, path } => {
                format!("jail://{session_id}/{path}")
            }
            Self::BackupSnapshot { snapshot_id, path } => {
                format!("snapshot://{snapshot_id}/{path}")
            }
        }
    }

    /// Return the underlying filesystem path string if this is a host or system path.
    #[must_use]
    pub fn as_host_path(&self) -> Option<&str> {
        match self {
            Self::HostUserPath(p) | Self::SystemConfigPath(p) => Some(p.as_str()),
            Self::AgentWorkspace { .. }
            | Self::SafeShellJail { .. }
            | Self::BackupSnapshot { .. } => None,
        }
    }

    /// Whether saving modifications to this location requires an Action1 `FileWrite` proposal.
    #[must_use]
    pub fn requires_action_authorization(&self) -> bool {
        matches!(self, Self::SystemConfigPath(_))
    }

    /// Whether this location is strictly read-only.
    #[must_use]
    pub fn is_read_only(&self) -> bool {
        matches!(self, Self::BackupSnapshot { .. })
    }

    /// Create a new `LocationRef` by detecting if a path is privileged system path.
    #[must_use]
    pub fn from_path(path: impl Into<String>) -> Self {
        let p = path.into();
        if p.starts_with("/etc/")
            || p.starts_with("/usr/")
            || p.starts_with("/lib/")
            || p.starts_with("/boot/")
        {
            Self::SystemConfigPath(p)
        } else {
            Self::HostUserPath(p)
        }
    }
}

impl Default for LocationRef {
    fn default() -> Self {
        Self::HostUserPath("/".to_owned())
    }
}
