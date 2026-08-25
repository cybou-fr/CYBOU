// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! D-Bus `org.cybou.Body.Executor1` service.

#![allow(missing_docs)]

use std::sync::Arc;

use cybou_fabric::encode;
use time::OffsetDateTime;
use uuid::Uuid;
use zbus::{fdo, interface};

use crate::{Body, PermitSource, execute};

/// The intentionally tiny executor dispatch surface.
pub struct Executor1Service<P, B> {
    permits: Arc<P>,
    body: Arc<B>,
}

impl<P, B> Executor1Service<P, B> {
    /// Bind the permit claimant to the physical adapters.
    #[must_use]
    pub fn new(permits: Arc<P>, body: Arc<B>) -> Self {
        Self { permits, body }
    }
}

#[allow(clippy::unused_async, reason = "zbus handlers are futures")]
#[interface(name = "org.cybou.Body.Executor1")]
impl<P, B> Executor1Service<P, B>
where
    P: PermitSource + 'static,
    B: Body + 'static,
{
    async fn ready(&self) -> bool {
        true
    }

    /// The only request: an opaque permit identity, with no operation or arguments beside it.
    async fn execute(&self, permit_id: String) -> fdo::Result<Vec<u8>> {
        let permit_id = Uuid::parse_str(&permit_id)
            .map_err(|_| fdo::Error::InvalidArgs("invalid permit identity".to_owned()))?;
        let attempt = execute(
            self.permits.as_ref(),
            self.body.as_ref(),
            permit_id,
            OffsetDateTime::now_utc(),
        )
        .await
        .map_err(|error| fdo::Error::AccessDenied(error.to_string()))?;
        encode(&attempt).map_err(|error| fdo::Error::Failed(error.to_string()))
    }
}
