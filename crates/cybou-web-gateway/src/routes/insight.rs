// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! What the host makes of itself, for the person looking at it (ADR-0041 S0).
//!
//! The answer to *what is going on with this host*, served from the deterministic layer. No model
//! is consulted and none could be: the whole path from `/proc` to this response is arithmetic and a
//! closed vocabulary, which is what makes it work at the moment it is most needed — when the network
//! is the thing under investigation.
//!
//! What it shows that a monitoring dashboard does not: **what the host would offer to do about each
//! finding, and what the authorization gate says about doing it.** Nothing here can carry any of it
//! out. Showing the verdict before an executor exists is deliberate — a person should be able to see
//! what the system would ask permission for while the answer is still theoretical.

use axum::{Json, extract::State, http::HeaderMap, http::StatusCode};

use crate::state::GatewayState;
use cybou_web_contracts::InsightProjection;

/// Return what this host currently makes of itself.
///
/// # Errors
///
/// Refuses with `401` when this deployment serves nothing until somebody signs in and nobody has.
pub async fn insight_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
) -> Result<Json<InsightProjection>, (StatusCode, Json<crate::state::ErrorBody>)> {
    // Gated with the rest of Mind rather than served openly. What a host is doing minute to minute
    // is not a public fact about it: a stranger who can read that memory pressure has been climbing
    // for an hour knows when to try something, and the readings are about this person's machine.
    if !state.may_read_mind(&headers) {
        return Err(GatewayState::sign_in_required());
    }
    Ok(Json(state.presence.insight().await))
}
