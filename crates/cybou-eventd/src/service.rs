// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! D-Bus `org.cybou.Mind.Event1` service implementation on zbus.

// The `#[interface]` expansion emits part of its dispatch surface with the attribute's own span,
// which an `allow` on the impl block cannot reach. Every handler written here is documented.
#![allow(missing_docs)]

use std::{fs, path::PathBuf, sync::Arc};

use cybou_protocol::canonical::CanonicalEnvelope;
use uuid::Uuid;
use zbus::{interface, object_server::SignalEmitter};

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
            .and_then(|p| p.parent().map(std::path::Path::to_path_buf));
        Self {
            core,
            trusted_bin_dir,
        }
    }

    fn resolve_caller_organ(&self, pid: u32) -> Option<String> {
        let exe_path = fs::read_link(format!("/proc/{pid}/exe")).ok()?;
        let parent = exe_path.parent()?;
        if let Some(ref trusted) = self.trusted_bin_dir
            && parent != trusted
        {
            return None;
        }
        let file_name = exe_path.file_name()?.to_str()?;
        let mut name = file_name;
        if let Some(stripped) = name.strip_prefix('.') {
            name = stripped;
        }
        if let Some(stripped) = name.strip_suffix("-wrapped") {
            name = stripped;
        }
        if let Some(organ) = name.strip_prefix("cybou-")
            && is_reserved_organ(organ)
        {
            return Some(organ.to_string());
        }
        None
    }
}

#[allow(
    clippy::unused_async,
    reason = "zbus dispatches every exported handler as a future"
)]
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
        self.core
            .head()
            .map_or_else(Vec::new, |e| encode_envelope(&e))
    }

    /// The verification the last incremental pass established, as CBOR, or empty when none has
    /// run yet.
    async fn verification(&self) -> Vec<u8> {
        self.core
            .verification()
            .map_or_else(Vec::new, |state| encode(&state))
    }

    /// Advance a full re-verification of the chain by one page, as CBOR.
    ///
    /// Bounded per call so the caller decides how much of a quiet moment to spend on it and can
    /// stop between pages when the moment ends.
    async fn verify_fully_step(&self, max_rows: u32) -> Vec<u8> {
        let step = self
            .core
            .verify_fully_step(u64::from(max_rows.clamp(1, 4096)));
        encode(&step)
    }

    /// The most exposing sensitivity anything in the Journal carries, on the frozen scale.
    ///
    /// Answers -1 when that could not be established, because the wire has no option type here and
    /// a caller must be able to tell "nothing sensitive" from "could not be read". Silently sending
    /// the safest-looking number would make an unreadable Journal indistinguishable from an
    /// innocuous one.
    async fn highest_sensitivity(&self) -> i16 {
        self.core.highest_sensitivity().map_or(-1, i16::from)
    }

    /// Current erasure epoch.
    async fn erasure_epoch(&self) -> u64 {
        self.core.erasure_epoch()
    }

    /// Forget a contribution and everything derived from it, and say what was reached.
    ///
    /// ADR-0028 keeps this off `Submit` on purpose: destroying biography must never be reachable
    /// by the same call that records a thought about it. The reason is a closed set, because an
    /// erasure record is permanent and free text would let the thing being forgotten be restated
    /// in the one place that can never be erased.
    ///
    /// Answers the number of contributions whose payload was actually redacted, or -1 when the
    /// erasure was refused. Zero is a real answer: a target already erased is not an error.
    async fn request_erasure(&self, message_id: String, reason: String) -> i64 {
        let Ok(target) = uuid::Uuid::parse_str(&message_id) else {
            return -1;
        };
        let Some(reason) = cybou_protocol::admission::ErasureReason::from_name(&reason) else {
            println!("[cybou-eventd] Erasure refused: '{reason}' is not a reason this build knows");
            return -1;
        };
        match self.core.request_erasure(&target, reason) {
            Ok(outcome) => {
                println!(
                    "[cybou-eventd] Erased {} of {} contributions in the closure of {target}; epoch {}",
                    outcome.redacted.len(),
                    outcome.closure.len(),
                    outcome.epoch
                );
                i64::try_from(outcome.redacted.len()).unwrap_or(i64::MAX)
            }
            Err(error) => {
                println!("[cybou-eventd] Erasure refused: {error}");
                -1
            }
        }
    }

    /// Retrieve an envelope at a specific sequence number.
    async fn at_sequence(&self, sequence: u64) -> Vec<u8> {
        self.core
            .at_sequence(sequence)
            .map_or_else(Vec::new, |e| encode_envelope(&e))
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
            .map_or_else(Vec::new, |e| encode_envelope(&e))
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
        let lim = usize::try_from(limit).unwrap_or(0);
        let envelopes = self.core.recent(lim);
        encode_envelopes(&envelopes)
    }

    /// Replay contributions after sequence up to limit.
    async fn replay(&self, after_sequence: u64, limit: i32) -> Vec<u8> {
        let lim = usize::try_from(limit).unwrap_or(0);
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

    /// Submit a contribution to `Event1` and emit an `Accepted` signal upon durable `SQLite` commit.
    async fn submit(
        &self,
        #[zbus(signal_emitter)] ctxt: SignalEmitter<'_>,
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
        if let Some(sender) = header.sender()
            && let Ok(dbus) = zbus::fdo::DBusProxy::new(connection).await
            && let Ok(pid) = dbus
                .get_connection_unix_process_id(sender.to_owned().into())
                .await
        {
            caller_organ = self.resolve_caller_organ(pid);
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
        ctxt: &SignalEmitter<'_>,
        encoded_envelope: &[u8],
        sequence: u64,
    ) -> zbus::Result<()>;
}

fn encode<T: serde::Serialize>(value: &T) -> Vec<u8> {
    let mut buf = Vec::new();
    let _ = ciborium::into_writer(value, &mut buf);
    buf
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
