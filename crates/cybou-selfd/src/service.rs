// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! D-Bus `org.cybou.Mind.Self1` service implementation on zbus.

// The `#[interface]` expansion emits part of its dispatch surface with the attribute's own span,
// which an `allow` on the impl block cannot reach. Every handler written here is documented, and
// the generated dispatch reads parameters that the fail-closed stubs below deliberately ignore.
#![allow(clippy::used_underscore_binding, missing_docs)]

use std::sync::Arc;

use cybou_fabric::{EVENT, INTENTION, PREDICTOR};
use time::OffsetDateTime;
use zbus::interface;

use crate::{CalibrationEntry, SelfCore, SelfReport, VerificationKnowledge, narrate_self_report};

/// D-Bus Service exporting `org.cybou.Mind.Self1`.
pub struct Self1Service {
    core: Arc<SelfCore>,
}

impl Self1Service {
    /// Create a new Self1 D-Bus service handler around `SelfCore`.
    #[must_use]
    pub fn new(core: Arc<SelfCore>) -> Self {
        Self { core }
    }

    /// Assemble a report from the owners that hold the facts it is about.
    ///
    /// Self1 owns the assessment, not the facts: obligations belong to Intention1, calibration to
    /// Predictor1 and the contribution count to Event1. An owner that does not answer leaves its
    /// part unknown rather than zero, which is why the narration can say it cannot determine
    /// something instead of claiming there is nothing to determine.
    async fn measure_now(&self, conn: &zbus::Connection, now: OffsetDateTime) -> SelfReport {
        let obligations = self.read_obligations(conn, now).await;
        let calibrations = self.read_calibrations(conn).await;
        let contributions = read::<u64>(conn, EVENT, "Count").await.unwrap_or_default();
        let verification = read_verification(conn).await;
        self.core
            .measure_with(now, contributions, obligations, calibrations, verification)
    }

    async fn read_obligations(
        &self,
        conn: &zbus::Connection,
        now: OffsetDateTime,
    ) -> Option<(u32, i64)> {
        let count = read::<u32>(conn, INTENTION, "OpenCount").await?;
        let encoded = read::<Vec<u8>>(conn, INTENTION, "Open").await?;
        // An owner holding nothing answers with an empty body; that is a known zero, and the
        // oldest obligation of none is none rather than an error.
        if encoded.is_empty() {
            return Some((count, 0));
        }
        let open: Vec<OpenIntention> = ciborium::from_reader(encoded.as_slice()).ok()?;
        let oldest = open
            .iter()
            .map(|item| (now - item.formed).whole_days())
            .max()
            .unwrap_or_default();
        Some((count, oldest))
    }

    async fn read_calibrations(
        &self,
        conn: &zbus::Connection,
    ) -> Option<(Vec<CalibrationEntry>, u32)> {
        let encoded = read::<Vec<u8>>(conn, PREDICTOR, "AllCalibrations").await?;
        if encoded.is_empty() {
            return Some((Vec::new(), 0));
        }
        let calibrations: Vec<OwnerCalibration> = ciborium::from_reader(encoded.as_slice()).ok()?;
        let settled = calibrations.iter().map(|item| item.settled).sum();
        Some((
            calibrations
                .into_iter()
                .map(|item| CalibrationEntry {
                    subject: item.subject,
                    settled: item.settled,
                    bias: item.bias,
                })
                .collect(),
            settled,
        ))
    }
}

/// What Event1 established about the chain, translated into what Self1 is willing to claim.
///
/// A chain replayed only part of the way is not verified memory. Reporting it as verified would
/// state something about rows nobody has looked at, so a pass still catching up stays unknown.
async fn read_verification(conn: &zbus::Connection) -> VerificationKnowledge {
    let Some(encoded) = read::<Vec<u8>>(conn, EVENT, "Verification").await else {
        return VerificationKnowledge::Unknown;
    };
    if encoded.is_empty() {
        return VerificationKnowledge::Unknown;
    }
    let Ok(state) = ciborium::from_reader::<OwnerVerification, _>(encoded.as_slice()) else {
        return VerificationKnowledge::Unknown;
    };
    match state.broken_at {
        Some(first_broken_at) => VerificationKnowledge::Invalid { first_broken_at },
        None if state.verified_through >= state.head => VerificationKnowledge::Verified,
        None => VerificationKnowledge::Unknown,
    }
}

/// Event1's verification state.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct OwnerVerification {
    verified_through: u64,
    head: u64,
    broken_at: Option<u64>,
}

/// One open obligation, of which only its age is needed here.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenIntention {
    #[serde(with = "time::serde::rfc3339")]
    formed: OffsetDateTime,
}

/// Predictor1's calibration row.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct OwnerCalibration {
    subject: String,
    settled: u32,
    bias: f64,
}

/// Call one owner method and decode its reply, treating any failure as "not answered".
async fn read<T>(
    conn: &zbus::Connection,
    endpoint: cybou_fabric::BusEndpoint,
    method: &str,
) -> Option<T>
where
    T: serde::de::DeserializeOwned + zbus::zvariant::Type,
{
    conn.call_method(
        Some(endpoint.service),
        endpoint.object_path,
        Some(endpoint.interface),
        method,
        &(),
    )
    .await
    .ok()?
    .body()
    .deserialize()
    .ok()
}

#[allow(
    clippy::unused_async,
    reason = "zbus dispatches every exported handler as a future"
)]
#[interface(name = "org.cybou.Mind.Self1")]
impl Self1Service {
    /// Service readiness.
    async fn ready(&self) -> bool {
        true
    }

    /// Measure the self model and return the `SelfReport` encoded as CBOR.
    async fn measure(&self, #[zbus(connection)] conn: &zbus::Connection) -> Vec<u8> {
        let now = OffsetDateTime::now_utc();
        let report = self.measure_now(conn, now).await;
        let mut buf = Vec::new();
        let _ = ciborium::into_writer(&report, &mut buf);
        buf
    }

    /// Assess self against a specific cause contribution and return `SelfReport` CBOR.
    async fn assess(
        &self,
        #[zbus(connection)] conn: &zbus::Connection,
        _cause_id: String,
    ) -> Vec<u8> {
        let now = OffsetDateTime::now_utc();
        let report = self.measure_now(conn, now).await;
        let mut buf = Vec::new();
        let _ = ciborium::into_writer(&report, &mut buf);
        buf
    }

    /// Narrate a given CBOR encoded `SelfReport` into a human-readable text.
    async fn narrate(&self, encoded_report: Vec<u8>) -> String {
        if let Ok(report) = ciborium::from_reader::<SelfReport, _>(encoded_report.as_slice()) {
            narrate_self_report(&report)
        } else {
            let now = OffsetDateTime::now_utc();
            let current = self.core.measure(now, 0);
            narrate_self_report(&current)
        }
    }
}
