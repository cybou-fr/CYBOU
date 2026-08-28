// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Strongly-typed subject references for spatial desktop entities (ADR-0046).

use crate::location::LocationRef;
use percent_encoding::{NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Why a desktop deep link could not become a complete subject reference.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum SubjectDeepLinkError {
    /// The fragment is not a CYBOU entity route.
    #[error("not a CYBOU subject deep link")]
    InvalidRoute,
    /// A percent-encoded segment is not valid UTF-8 or is structurally unsafe.
    #[error("deep-link segment is invalid")]
    InvalidSegment,
    /// The route names an entity whose complete authority or metadata must be resolved by an owner.
    #[error("this subject kind requires an owner-backed resolver")]
    OwnerResolutionRequired,
}

/// An identity-shaped lookup request that has not been resolved by an authoritative owner.
///
/// Unlike [`SubjectRef`], this type carries no claim that an entity exists and cannot assign file
/// authority, agent metadata, installed versions, or other owner-established attributes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "identifier", rename_all = "kebab-case")]
pub enum SubjectQuery {
    /// Lookup a systemd unit by its user-supplied name.
    Service(String),
    /// Lookup a file by a user-supplied path without assigning a [`LocationRef`] domain.
    File(String),
    /// Lookup an agent capsule by an identity string.
    Agent(String),
    /// Lookup an installed package by name.
    Package(String),
    /// Lookup a spatial anchor by identity.
    Anchor(String),
    /// Lookup an active or historical operation by ID.
    Operation(String),
    /// Lookup an operating system process by PID or name.
    Process(String),
}

impl SubjectQuery {
    /// Category name suitable for an unresolved-query UI badge.
    #[must_use]
    pub const fn kind_name(&self) -> &'static str {
        match self {
            Self::Service(_) => "Service query",
            Self::File(_) => "File query",
            Self::Agent(_) => "Agent query",
            Self::Package(_) => "Package query",
            Self::Anchor(_) => "Anchor query",
            Self::Operation(_) => "Operation query",
            Self::Process(_) => "Process query",
        }
    }

    /// Identifier exactly as supplied for owner resolution.
    #[must_use]
    pub fn identifier(&self) -> &str {
        match self {
            Self::Service(value)
            | Self::File(value)
            | Self::Agent(value)
            | Self::Package(value)
            | Self::Anchor(value)
            | Self::Operation(value)
            | Self::Process(value) => value,
        }
    }
}

fn encoded_segment(value: &str) -> String {
    utf8_percent_encode(value, NON_ALPHANUMERIC).to_string()
}

fn decoded_segment(value: &str) -> Result<String, SubjectDeepLinkError> {
    decoded_segment_with_slash(value, false)
}

fn decoded_segment_with_slash(
    value: &str,
    allow_slash: bool,
) -> Result<String, SubjectDeepLinkError> {
    let decoded = percent_decode_str(value)
        .decode_utf8()
        .map_err(|_| SubjectDeepLinkError::InvalidSegment)?;
    if decoded.is_empty()
        || decoded
            .chars()
            .any(|ch| ch.is_control() || (!allow_slash && ch == '/') || ch == '\\')
    {
        return Err(SubjectDeepLinkError::InvalidSegment);
    }
    Ok(decoded.into_owned())
}

/// Strongly-typed subject reference for entities displayed, connected, or dragged on the Living Canvas.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", content = "payload", rename_all = "kebab-case")]
pub enum SubjectRef {
    /// System service daemon (e.g. systemd unit).
    Service {
        /// Canonical unit name.
        name: String,
        /// Optional cluster node identifier.
        node_id: Option<String>,
    },
    /// Operating system process.
    Process {
        /// Process ID.
        pid: u32,
        /// Executable name or command line summary.
        name: String,
    },
    /// Filesystem file or directory reference with authority boundaries.
    File {
        /// Target location.
        location: LocationRef,
    },
    /// Autonomous Agent Capsule instance.
    Agent {
        /// Capsule identifier.
        capsule_id: String,
        /// Agent type or profile.
        agent_type: String,
    },
    /// Mail message item.
    MailMessage {
        /// Mail account identifier.
        account_id: String,
        /// Folder name.
        folder: String,
        /// Message ID.
        message_id: String,
    },
    /// Calendar schedule event.
    CalendarEvent {
        /// Account ID.
        account_id: String,
        /// Event ID.
        event_id: String,
    },
    /// TLS / X.509 Certificate.
    Certificate {
        /// Fully qualified domain name.
        domain: String,
        /// SHA-256 fingerprint.
        thumbprint: String,
    },
    /// Storage mount point or disk volume.
    Filesystem {
        /// Mount path.
        mount_point: String,
        /// Filesystem type.
        fs_type: String,
    },
    /// OS Package definition.
    Package {
        /// Package name.
        name: String,
        /// Installed version string.
        installed_version: Option<String>,
    },
    /// Spatial desktop camera anchor.
    Anchor {
        /// Anchor identifier.
        anchor_id: String,
        /// Human-readable label.
        label: String,
    },
    /// Server-owned background operation or long-running task.
    Operation {
        /// Operation identifier.
        operation_id: String,
        /// Operation kind.
        kind: String,
        /// Human-readable label or description.
        label: String,
    },
}

impl SubjectRef {
    /// Human-readable title for UI chips and inspectors.
    #[must_use]
    pub fn display_title(&self) -> String {
        match self {
            Self::Service { name, .. } => name.clone(),
            Self::Process { pid, name } => format!("{name} (pid {pid})"),
            Self::File { location } => location.display_path(),
            Self::Agent {
                agent_type,
                capsule_id,
            } => {
                format!("{agent_type} [{}]", &capsule_id[..capsule_id.len().min(8)])
            }
            Self::MailMessage { message_id, .. } => format!("Message {message_id}"),
            Self::CalendarEvent { event_id, .. } => format!("Event {event_id}"),
            Self::Certificate { domain, .. } => format!("Cert: {domain}"),
            Self::Filesystem { mount_point, .. } => format!("FS: {mount_point}"),
            Self::Package {
                name,
                installed_version,
            } => {
                if let Some(v) = installed_version {
                    format!("{name} ({v})")
                } else {
                    name.clone()
                }
            }
            Self::Anchor { label, .. } => format!("Anchor: {label}"),
            Self::Operation { label, .. } => format!("Op: {label}"),
        }
    }

    /// Category / kind name of this subject.
    #[must_use]
    pub const fn kind_name(&self) -> &'static str {
        match self {
            Self::Service { .. } => "Service",
            Self::Process { .. } => "Process",
            Self::File { .. } => "File",
            Self::Agent { .. } => "Agent",
            Self::MailMessage { .. } => "Mail",
            Self::CalendarEvent { .. } => "Calendar",
            Self::Certificate { .. } => "Certificate",
            Self::Filesystem { .. } => "Filesystem",
            Self::Package { .. } => "Package",
            Self::Anchor { .. } => "Anchor",
            Self::Operation { .. } => "Operation",
        }
    }

    /// Canonical URI representation (e.g. `cybou://service/nginx.service`).
    #[must_use]
    pub fn uri(&self) -> String {
        match self {
            Self::Service { name, .. } => format!("cybou://service/{name}"),
            Self::Process { pid, .. } => format!("cybou://process/{pid}"),
            Self::File { location } => format!("cybou://file/{}", location.display_path()),
            Self::Agent { capsule_id, .. } => format!("cybou://agent/{capsule_id}"),
            Self::MailMessage {
                account_id,
                folder,
                message_id,
            } => {
                format!("cybou://mail/{account_id}/{folder}/{message_id}")
            }
            Self::CalendarEvent {
                account_id,
                event_id,
            } => {
                format!("cybou://calendar/{account_id}/{event_id}")
            }
            Self::Certificate { domain, .. } => format!("cybou://certificate/{domain}"),
            Self::Filesystem { mount_point, .. } => format!("cybou://filesystem{mount_point}"),
            Self::Package { name, .. } => format!("cybou://package/{name}"),
            Self::Anchor { anchor_id, .. } => format!("cybou://anchor/{anchor_id}"),
            Self::Operation { operation_id, .. } => format!("cybou://operation/{operation_id}"),
        }
    }

    /// Web hash fragment for internal deep linking (e.g. `/#/service/nginx.service`).
    #[must_use]
    pub fn deep_link_hash(&self) -> String {
        match self {
            Self::Service { name, .. } => format!("/#/service/{}", encoded_segment(name)),
            Self::Process { pid, .. } => format!("/#/process/{pid}"),
            Self::File { location } => format!("/#/file{}", location.display_path()),
            Self::Agent { capsule_id, .. } => format!("/#/agent/{capsule_id}"),
            Self::MailMessage {
                account_id,
                folder,
                message_id,
            } => {
                format!(
                    "/#/mail/{}/{}/{}",
                    encoded_segment(account_id),
                    encoded_segment(folder),
                    encoded_segment(message_id)
                )
            }
            Self::CalendarEvent {
                account_id,
                event_id,
            } => {
                format!(
                    "/#/calendar/{}/{}",
                    encoded_segment(account_id),
                    encoded_segment(event_id)
                )
            }
            Self::Certificate { domain, .. } => format!("/#/certificate/{domain}"),
            Self::Filesystem { mount_point, .. } => format!("/#/filesystem{mount_point}"),
            Self::Package { name, .. } => format!("/#/package/{}", encoded_segment(name)),
            Self::Anchor { anchor_id, .. } => format!("/#/anchor/{anchor_id}"),
            Self::Operation { operation_id, .. } => format!("/#/operation/{operation_id}"),
        }
    }

    /// Parse a browser hash into a complete subject where the URL carries enough information.
    ///
    /// File, process, agent, certificate, filesystem, and anchor links deliberately require an
    /// owner-backed resolver: their public URL does not contain enough metadata or authority to
    /// construct a truthful [`SubjectRef`] in the browser.
    ///
    /// # Errors
    ///
    /// Returns [`SubjectDeepLinkError`] for malformed routes, unsafe encoding, unsupported subject
    /// kinds, or identity-only routes that require an owner lookup.
    pub fn from_deep_link_hash(hash: &str) -> Result<Self, SubjectDeepLinkError> {
        let route = hash
            .strip_prefix("/#/")
            .or_else(|| hash.strip_prefix("#/"))
            .ok_or(SubjectDeepLinkError::InvalidRoute)?;
        let parts = route.split('/').collect::<Vec<_>>();
        match parts.as_slice() {
            ["service", name] => Ok(Self::Service {
                name: decoded_segment(name)?,
                node_id: None,
            }),
            ["package", name] => Ok(Self::Package {
                name: decoded_segment(name)?,
                installed_version: None,
            }),
            ["mail", account_id, folder, message_id] => Ok(Self::MailMessage {
                account_id: decoded_segment(account_id)?,
                folder: decoded_segment_with_slash(folder, true)?,
                message_id: decoded_segment(message_id)?,
            }),
            ["calendar", account_id, event_id] => Ok(Self::CalendarEvent {
                account_id: decoded_segment(account_id)?,
                event_id: decoded_segment(event_id)?,
            }),
            [
                "file" | "process" | "agent" | "certificate" | "filesystem" | "anchor" | "operation",
                ..,
            ] => Err(SubjectDeepLinkError::OwnerResolutionRequired),
            _ => Err(SubjectDeepLinkError::InvalidRoute),
        }
    }
}

/// Epistemic presentation state for user-facing attributes (ADR-0046 §25).
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "state", content = "data", rename_all = "kebab-case")]
pub enum EpistemicPresentation<T> {
    /// Actively known and confirmed data.
    Known(T),
    /// Unknown due to absence of observation.
    Unknown {
        /// Explanation for unknown status.
        reason: String,
    },
    /// Unavailable due to subsystem or network deficit.
    Unavailable {
        /// Explanation for unavailability.
        reason: String,
    },
    /// Stale data beyond freshness horizon.
    Stale {
        /// Last observed data value.
        data: T,
        /// UTC timestamp of last observation.
        last_observed: String,
    },
    /// Forbidden due to authorization deficit.
    Forbidden {
        /// Required capability or permission scope.
        required_scope: String,
    },
    /// Subsystem or entity is unconfigured.
    NotConfigured,
    /// Verified empty collection or zero occurrences.
    Empty,
}

impl<T> EpistemicPresentation<T> {
    /// Check whether the value is currently known and fresh.
    #[must_use]
    pub const fn is_known(&self) -> bool {
        matches!(self, Self::Known(_))
    }

    /// Extract inner reference if known or stale.
    #[must_use]
    pub fn value(&self) -> Option<&T> {
        match self {
            Self::Known(v) | Self::Stale { data: v, .. } => Some(v),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subject_ref_uris_and_titles() {
        let service = SubjectRef::Service {
            name: "nginx.service".to_string(),
            node_id: None,
        };
        assert_eq!(service.display_title(), "nginx.service");
        assert_eq!(service.kind_name(), "Service");
        assert_eq!(service.uri(), "cybou://service/nginx.service");
        assert_eq!(service.deep_link_hash(), "/#/service/nginx%2Eservice");
        assert_eq!(
            SubjectRef::from_deep_link_hash("#/service/nginx%2Eservice"),
            Ok(service)
        );
    }

    #[test]
    fn subject_queries_carry_identity_without_owner_claims() {
        let file = SubjectQuery::File("/etc/hosts".to_string());
        assert_eq!(file.kind_name(), "File query");
        assert_eq!(file.identifier(), "/etc/hosts");
        assert_eq!(
            serde_json::to_value(file).expect("query serializes"),
            serde_json::json!({"kind": "file", "identifier": "/etc/hosts"})
        );

        let agent = SubjectQuery::Agent("candidate-capsule".to_string());
        assert_eq!(agent.kind_name(), "Agent query");
        assert_eq!(agent.identifier(), "candidate-capsule");
    }

    #[test]
    fn deep_links_decode_complete_subjects_without_inventing_metadata() {
        let calendar = SubjectRef::CalendarEvent {
            account_id: "work@example.test".to_string(),
            event_id: "event 42".to_string(),
        };
        assert_eq!(
            SubjectRef::from_deep_link_hash(&calendar.deep_link_hash()),
            Ok(calendar)
        );
        let mail = SubjectRef::MailMessage {
            account_id: "work".to_string(),
            folder: "Archive/2026".to_string(),
            message_id: "message 7".to_string(),
        };
        assert_eq!(
            SubjectRef::from_deep_link_hash(&mail.deep_link_hash()),
            Ok(mail)
        );
        assert_eq!(
            SubjectRef::from_deep_link_hash("#/package/cybou%2Dagentd"),
            Ok(SubjectRef::Package {
                name: "cybou-agentd".to_string(),
                installed_version: None,
            })
        );
    }

    #[test]
    fn authority_bearing_and_unsafe_deep_links_are_not_browser_minted() {
        assert_eq!(
            SubjectRef::from_deep_link_hash("#/file/etc/passwd"),
            Err(SubjectDeepLinkError::OwnerResolutionRequired)
        );
        assert_eq!(
            SubjectRef::from_deep_link_hash("#/service/%2E%2E%2Fetc"),
            Err(SubjectDeepLinkError::InvalidSegment)
        );
        assert_eq!(
            SubjectRef::from_deep_link_hash("#/service/%FF"),
            Err(SubjectDeepLinkError::InvalidSegment)
        );
    }

    #[test]
    fn epistemic_presentation_helpers() {
        let known = EpistemicPresentation::Known(42);
        assert!(known.is_known());
        assert_eq!(known.value(), Some(&42));

        let stale = EpistemicPresentation::Stale {
            data: 42,
            last_observed: "2026-08-27T16:00:00Z".to_string(),
        };
        assert!(!stale.is_known());
        assert_eq!(stale.value(), Some(&42));

        let empty: EpistemicPresentation<i32> = EpistemicPresentation::Empty;
        assert!(!empty.is_known());
        assert_eq!(empty.value(), None);
    }
}
