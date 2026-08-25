// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! D-Bus `org.cybou.Mind.Action1` service.

#![allow(missing_docs)]

use std::sync::Arc;

use cybou_fabric::{decode, encode};
use cybou_protocol::telemetry::SystemInsight;
use time::OffsetDateTime;
use uuid::Uuid;
use zbus::{fdo, interface};

use crate::ActionCore;

/// Process-owned Action1 dispatch surface.
pub struct Action1Service {
    core: Arc<ActionCore>,
}

impl Action1Service {
    /// Wrap the lifecycle owner.
    #[must_use]
    pub fn new(core: Arc<ActionCore>) -> Self {
        Self { core }
    }
}

#[allow(clippy::unused_async, reason = "zbus handlers are futures")]
#[interface(name = "org.cybou.Mind.Action1")]
impl Action1Service {
    async fn ready(&self) -> bool {
        true
    }

    async fn evaluate_insight(
        &self,
        insight: Vec<u8>,
        operation: String,
    ) -> fdo::Result<(Vec<u8>, String)> {
        let insight: SystemInsight =
            decode(&insight).map_err(|error| fdo::Error::InvalidArgs(error.to_string()))?;
        let record = self
            .core
            .evaluate_insight(&insight, &operation, OffsetDateTime::now_utc())
            .map_err(|error| fdo::Error::Failed(error.to_string()))?;
        let permit_id = record
            .permit_id
            .map_or_else(String::new, |id| id.to_string());
        let encoded = encode(&record).map_err(|error| fdo::Error::Failed(error.to_string()))?;
        Ok((encoded, permit_id))
    }

    async fn claim_permit(&self, permit_id: String) -> fdo::Result<Vec<u8>> {
        let permit_id = Uuid::parse_str(&permit_id)
            .map_err(|_| fdo::Error::InvalidArgs("invalid permit identity".to_owned()))?;
        let permit = self
            .core
            .claim_permit(permit_id, OffsetDateTime::now_utc())
            .map_err(|error| fdo::Error::AccessDenied(error.to_string()))?;
        encode(&permit).map_err(|error| fdo::Error::Failed(error.to_string()))
    }
}
