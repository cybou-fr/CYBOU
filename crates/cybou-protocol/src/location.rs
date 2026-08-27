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
    /// Editor-local draft that has not been bound to any filesystem owner.
    Draft {
        /// Identity within the owning editor state.
        draft_id: String,
    },
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
            Self::Draft { draft_id } => format!("draft://{draft_id}"),
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
            Self::Draft { .. }
            | Self::AgentWorkspace { .. }
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
}

impl Default for LocationRef {
    fn default() -> Self {
        Self::Draft {
            draft_id: "untitled".to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LocationRef;

    #[test]
    fn a_jail_path_never_becomes_a_host_authority_domain() {
        let location = LocationRef::SafeShellJail {
            session_id: "seat-1".to_string(),
            path: "/etc/example.conf".to_string(),
        };

        assert_eq!(location.as_host_path(), None);
        assert!(!location.requires_action_authorization());
        assert_eq!(location.display_path(), "jail://seat-1//etc/example.conf");
    }

    #[test]
    fn an_unbound_draft_claims_no_host_path() {
        let draft = LocationRef::default();
        assert_eq!(draft.as_host_path(), None);
        assert_eq!(draft.display_path(), "draft://untitled");
    }
}
