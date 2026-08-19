// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! D-Bus `org.cybou.Mind.Event1` service implementation on zbus.

use std::{fs, path::PathBuf, sync::Arc};

use cybou_protocol::canonical::CanonicalEnvelope;
use uuid::Uuid;
use zbus::{SignalContext, interface};

use crate::{EventCore, SubmitResult, is_reserved_organ};

/// D-Bus Service exporting `org.cybou.Mind.Event1`.
pub struct Event1Service {
    core: Arc<EventCore>,
    trusted_bin_dir: Option<PathBuf>,
}

impl Event1Service {
    /// Create a new Event1 D-Bus service handler around `EventCore`.
    #[must_use]
    pub fn new(core: Arc<EventCore>) -> Self {
        let trusted_bin_dir = fs::read_link("/proc/self/exe")
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()));
        Self {
            core,
            trusted_bin_dir,
        }
    }

    fn resolve_caller_organ(&self, pid: u32) -> Option<String> {
        let exe_path = fs::read_link(format!("/proc/{pid}/exe")).ok()?;
        let parent = exe_path.parent()?;
        if let Some(ref trusted) = self.trusted_bin_dir {
            if parent != trusted {
                return None;
            }
        }
        let file_name = exe_path.file_name()?.to_str()?;
        let mut name = file_name;
        if let Some(stripped) = name.strip_prefix('.') {
            name = stripped;
        }
        if let Some(stripped) = name.strip_suffix("-wrapped") {
            name = stripped;
        }
        if let Some(organ) = name.strip_prefix("cybou-") {
            if is_reserved_organ(organ) {
                return Some(organ.to_string());
            }
        }
        None
    }
}

#[interface(name = "org.cybou.Mind.Event1")]
impl Event1Service {
    /// Ready status.
    async fn ready(&self) -> bool {
        true
    }

    /// Database schema version.
    async fn schema_version(&self) -> i32 {
        2
    }

    /// Total contribution count.
    async fn count(&self) -> u64 {
        self.core.count()
    }

    /// Head envelope as CBOR bytes.
    async fn head(&self) -> Vec<u8> {
        self.core.head().map_or_default(|e| encode_envelope(&e))
    }

    /// Current erasure epoch.
    async fn erasure_epoch(&self) -> u64 {
        self.core.erasure_epoch()
    }

    /// Retrieve an envelope at a specific sequence number.
    async fn at_sequence(&self, sequence: u64) -> Vec<u8> {
        self.core
            .at_sequence(sequence)
            .map_or_default(|e| encode_envelope(&e))
    }

    /// Return whether message ID is present.
    async fn contains(&self, message_id: String) -> bool {
        let Ok(id) = Uuid::parse_str(&message_id) else {
            return false;
        };
        self.core.find_by_message_id(&id).is_some()
    }

    /// Retrieve contribution by message ID.
    async fn contribution(&self, message_id: String) -> Vec<u8> {
        let Ok(id) = Uuid::parse_str(&message_id) else {
            return vec![];
        };
        self.core
            .find_by_message_id(&id)
            .map_or_default(|e| encode_envelope(&e))
    }

    /// Retrieve evidence UUIDs for message ID.
    async fn evidence_for(&self, message_id: String) -> Vec<u8> {
        let Ok(id) = Uuid::parse_str(&message_id) else {
            return vec![];
        };
        let uuids = self.core.evidence_for(&id);
        let strs: Vec<String> = uuids.into_iter().map(|u| u.to_string()).collect();
        let mut buf = Vec::new();
        let _ = ciborium::into_writer(&strs, &mut buf);
        buf
    }

    /// Check outcome existence for cause and organ.
    async fn has_outcome_for(&self, cause_id: String, origin_organ: String) -> bool {
        let Ok(id) = Uuid::parse_str(&cause_id) else {
            return false;
        };
        self.core.has_outcome_for(&id, &origin_organ)
    }

    /// Ensure consumer offset registration.
    async fn ensure_consumer(&self, consumer_id: String, initial_offset: u64) -> bool {
        self.core.ensure_consumer(&consumer_id, initial_offset)
    }

    /// Advance consumer offset.
    async fn advance_consumer(&self, consumer_id: String, offset: u64) -> bool {
        self.core.advance_consumer(&consumer_id, offset)
    }

    /// Consumer backlog.
    async fn consumer_backlog(&self, consumer_id: String) -> Vec<u8> {
        let backlog = self.core.consumer_backlog(&consumer_id).unwrap_or(0);
        let mut buf = Vec::new();
        let _ = ciborium::into_writer(&backlog, &mut buf);
        buf
    }

    /// Recent contributions up to limit.
    async fn recent(&self, limit: i32) -> Vec<u8> {
        let lim = if limit > 0 { limit as usize } else { 0 };
        let envelopes = self.core.recent(lim);
        encode_envelopes(&envelopes)
    }

    /// Replay contributions after sequence up to limit.
    async fn replay(&self, after_sequence: u64, limit: i32) -> Vec<u8> {
        let lim = if limit > 0 { limit as usize } else { 0 };
        let envelopes = self.core.replay(after_sequence, lim);
        encode_envelopes(&envelopes)
    }

    /// Episode contributions for correlation ID.
    async fn episode(&self, correlation_id: String) -> Vec<u8> {
        let Ok(id) = Uuid::parse_str(&correlation_id) else {
            return vec![];
        };
        let envelopes = self.core.find_by_correlation_id(&id);
        encode_envelopes(&envelopes)
    }

    /// Submit a contribution to Event1 and emit Accepted signal upon durable SQLite commit.
    async fn submit(
        &self,
        #[zbus(signal_context)] ctxt: SignalContext<'_>,
        #[zbus(header)] header: zbus::message::Header<'_>,
        #[zbus(connection)] connection: &zbus::Connection,
        encoded_envelope: Vec<u8>,
    ) -> Vec<u8> {
        let decoded: CanonicalEnvelope = match ciborium::from_reader(encoded_envelope.as_slice()) {
            Ok(env) => env,
            Err(err) => {
                return SubmitResult::failure(format!("CBOR decode error: {err}")).to_cbor();
            }
        };

        // Check sender PID via D-Bus connection interface if available
        let mut caller_organ = None;
        if let Some(sender) = header.sender() {
            if let Ok(pid) = connection
                .inner()
                .bus_interface()
                .get_connection_unix_process_id(sender.into())
                .await
            {
                caller_organ = self.resolve_caller_organ(pid);
            }
        }

        match self.core.submit(&decoded, caller_organ.as_deref()) {
            Ok(appended) => {
                let _ = Self::accepted(&ctxt, &encoded_envelope, appended.sequence).await;
                SubmitResult::success(appended.sequence).to_cbor()
            }
            Err(err) => SubmitResult::failure(err.to_string()).to_cbor(),
        }
    }

    /// Signal emitted when a contribution is durably committed to the Journal.
    #[zbus(signal)]
    async fn accepted(
        ctxt: &SignalContext<'_>,
        encoded_envelope: &[u8],
        sequence: u64,
    ) -> zbus::Result<()>;
}

fn encode_envelope(env: &CanonicalEnvelope) -> Vec<u8> {
    let mut buf = Vec::new();
    let _ = ciborium::into_writer(env, &mut buf);
    buf
}

fn encode_envelopes(envs: &[CanonicalEnvelope]) -> Vec<u8> {
    let mut buf = Vec::new();
    let _ = ciborium::into_writer(envs, &mut buf);
    buf
}
