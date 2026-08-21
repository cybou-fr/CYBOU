// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Core gateway state, trait abstractions, errors, and trust context types.

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use axum::{
    Json,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use cybou_protocol::{
    canonical::CanonicalEnvelope,
    disclosure::{ConsumerTrust, Destination},
};
use cybou_web_contracts::{
    MindProjection, SessionMode, SessionProjection, SnapshotProjection, WEB_SCHEMA_V1,
};
use serde::Serialize;
use thiserror::Error;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::access::{self, CredentialVerifier, Session, Sessions};
use crate::disclose::Disclosures;

/// Maximum time the gateway permits one Presence projection request to occupy.
pub const SNAPSHOT_BUDGET: Duration = Duration::from_millis(1_500);

/// Interval used until Presence exposes a native changed-event subscription.
pub const EVENT_POLL_INTERVAL: Duration = Duration::from_secs(2);

/// Maximum byte length for a resume cursor header value.
pub const MAX_CURSOR_BYTES: usize = 256;

/// What one projection supplied, and what it held back.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Delivered {
    /// The contributions the supplied items were derived from.
    pub items: Vec<Uuid>,
    /// How many items were supplied, whether or not their provenance was known.
    pub item_count: u32,
    /// What was held back, and why.
    pub withheld: Vec<cybou_protocol::disclosure::Withheld>,
}

/// Where a disclosure record goes.
#[async_trait]
pub trait DisclosureSink: Send + Sync + 'static {
    /// Record one disclosure, answering whether it was accepted.
    async fn record(&self, envelope: &CanonicalEnvelope) -> bool;
}

/// Where the gateway reads Mind from.
#[async_trait]
pub trait PresenceSource: Send + Sync + 'static {
    /// Return one atomic, presentation-ready snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`GatewayError`] when transport, decoding, or owner availability prevents a typed projection.
    async fn snapshot(&self) -> Result<SnapshotProjection, GatewayError>;

    /// What the last projection this source built supplied, and what it left out.
    fn last_delivery(&self) -> Delivered {
        Delivered::default()
    }

    /// Wait until the source may have a newer projection.
    async fn wait_for_change(&self) -> Result<(), GatewayError> {
        tokio::time::sleep(EVENT_POLL_INTERVAL).await;
        Ok(())
    }

    /// Return what the owners behind Mind actually hold right now.
    ///
    /// # Errors
    ///
    /// Returns [`GatewayError`] when the source cannot produce the projection at all.
    async fn mind(&self) -> Result<MindProjection, GatewayError> {
        Err(GatewayError::Unavailable)
    }
}

/// Failure response safe to expose at the browser boundary.
#[derive(Clone, Debug, Error)]
pub enum GatewayError {
    /// Presence did not answer within the gateway's outer budget.
    #[error("presence snapshot exceeded the gateway budget")]
    Timeout,
    /// Presence transport is unavailable.
    #[error("presence transport is unavailable")]
    Unavailable,
    /// Presence returned data that cannot satisfy the web contract.
    #[error("presence projection is incompatible with the web contract")]
    InvalidProjection,
    /// A resume cursor is malformed or exceeds the bounded header budget.
    #[error("event resume cursor is invalid")]
    InvalidCursor,
}

/// Error payload serialised to HTTP responses.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorBody {
    /// Web contract schema version.
    pub schema_version: cybou_protocol::SchemaVersion,
    /// Stable error code string.
    pub error: &'static str,
    /// Whether the client may retry the operation.
    pub retryable: bool,
}

impl IntoResponse for GatewayError {
    fn into_response(self) -> Response {
        let (status, error, retryable) = match self {
            Self::Timeout => (StatusCode::GATEWAY_TIMEOUT, "presenceTimeout", true),
            Self::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, "presenceUnavailable", true),
            Self::InvalidProjection => {
                (StatusCode::BAD_GATEWAY, "invalidPresenceProjection", false)
            }
            Self::InvalidCursor => (StatusCode::BAD_REQUEST, "invalidEventCursor", false),
        };
        (
            status,
            Json(ErrorBody {
                schema_version: WEB_SCHEMA_V1,
                error,
                retryable,
            }),
        )
            .into_response()
    }
}

/// Gateway state shared across axum routes.
#[derive(Clone)]
pub struct GatewayState {
    /// Primary Presence projection source.
    pub presence: Arc<dyn PresenceSource>,
    /// Unfiltered Presence projection source for authenticated sessions.
    pub privileged: Option<Arc<dyn PresenceSource>>,
    /// Pluggable credential verifier.
    pub verifier: Option<Arc<dyn CredentialVerifier>>,
    /// Active sessions store.
    pub sessions: Arc<Sessions>,
    /// Disclosures tracker.
    pub disclosures: Arc<Disclosures>,
    /// Journal disclosure sink.
    pub journal: Option<Arc<dyn DisclosureSink>>,
    /// Baseline session projection.
    pub session: SessionProjection,
    /// Sandboxed shell engine instance.
    pub shell: Arc<tokio::sync::Mutex<cybou_shelld::ShellEngine>>,
}

impl GatewayState {
    /// Who this request is, as a consumer.
    #[must_use]
    pub fn destination_for(session: Option<&Session>) -> Destination {
        session.map_or_else(
            || Destination {
                id: "living-canvas:public".to_owned(),
                trust: ConsumerTrust::Public,
                retains: false,
                external_boundary: true,
            },
            |session| Destination {
                id: format!("living-canvas:{}", session.username),
                trust: ConsumerTrust::Owner,
                retains: false,
                external_boundary: true,
            },
        )
    }

    /// Record that this consumer was supplied what the source just built, if that is new.
    pub async fn record_delivery(&self, destination: &Destination, delivered: &Delivered) {
        let Some(journal) = &self.journal else {
            return;
        };
        let Some(record) =
            self.disclosures
                .record_for(destination, delivered, OffsetDateTime::now_utc())
        else {
            return;
        };
        if !journal.record(&record).await {
            eprintln!(
                "a delivery to {} could not be recorded; the audit trail is incomplete",
                destination.id
            );
        }
    }

    /// The session this request carries, if it carries a live one.
    #[must_use]
    pub fn session_for(&self, headers: &HeaderMap) -> Option<Session> {
        let token = headers
            .get(axum::http::header::COOKIE)
            .and_then(|value| value.to_str().ok())
            .and_then(access::token_in)?;
        self.sessions.resolve(token, OffsetDateTime::now_utc())
    }

    /// The source this request is entitled to.
    #[must_use]
    pub fn source_for(&self, headers: &HeaderMap) -> &Arc<dyn PresenceSource> {
        let Some(privileged) = &self.privileged else {
            return &self.presence;
        };
        if self.session_for(headers).is_some() {
            privileged
        } else {
            &self.presence
        }
    }
}

/// Server-established browser trust context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionContext {
    /// Trust mode presented to the frontend.
    pub mode: SessionMode,
    /// Stable named consumer used by projection policy.
    pub consumer_id: String,
}

impl SessionContext {
    /// Device-bound context used by the desktop shell.
    #[must_use]
    pub fn local_desktop() -> Self {
        Self {
            mode: SessionMode::LocalDesktop,
            consumer_id: "cybou-desktop".into(),
        }
    }

    /// Read-only public preview context.
    #[must_use]
    pub fn public_preview() -> Self {
        Self {
            mode: SessionMode::PublicPreview,
            consumer_id: "public-preview".into(),
        }
    }
}
