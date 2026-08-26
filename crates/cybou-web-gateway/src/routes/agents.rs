// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! What agents are running, asked of the thing that is running them.
//!
//! A proxy and deliberately nothing more. This does not read the launch directory, does not ask a
//! service manager, and does not assemble a session from a lease and a plan — `Agent1` does all of
//! that, and a second thing doing it would be a second answer to *what is running*. The one that is
//! not the owner's is wrong the moment a session starts or ends between its listing and its reading,
//! and a person comparing two surfaces would have no way to tell which.
//!
//! So the shape it serves is the owner's own [`SessionView`], carried whole. Nothing here reshapes
//! it, fills anything in, or supplies a figure the owner declined to state — an unknown spend stays
//! unknown, because the reason it is unknown is that nobody in this process has seen the ledger
//! either.
//!
//! ## Read, and only read
//!
//! There is no stop here and no launch. Stopping is a decision about somebody's running work and
//! belongs behind the owner's own surface, where the ending can be confirmed before it is reported.
//! Launching needs admission against the whole host and authorization of the caller to the profile,
//! neither of which a proxy can do.

use axum::{Json, extract::State, http::HeaderMap, http::StatusCode};
use cybou_fabric::AGENT;
use cybou_protocol::agent::SessionView;

use crate::state::GatewayState;

/// Return every agent session this host is holding.
///
/// # Errors
///
/// Refuses with `401` when nobody has signed in, and `503` when the agent runtime is not answering.
/// Those are different facts and are reported differently: *you may not see this* and *there is
/// nothing here to ask* would otherwise be indistinguishable to somebody whose runtime had stopped.
pub async fn agents_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
) -> Result<Json<Vec<SessionView>>, (StatusCode, Json<crate::state::ErrorBody>)> {
    // Gated with the rest of Mind. What is running on somebody's host, in which directory, against
    // which model and with what left to spend, is not a public fact about them.
    if !state.may_read_mind(&headers) {
        return Err(GatewayState::sign_in_required());
    }

    match sessions().await {
        Ok(sessions) => Ok(Json(sessions)),
        Err(why) => {
            // The detail goes to the operator's log rather than into the response. What is wrong
            // with a bus connection is a fact about this host's insides, and a signed-out stranger
            // guessing at whether an endpoint exists should not learn it from an error body.
            eprintln!(
                "[cybou-web-gateway] {} is not answering: {why}",
                AGENT.service
            );
            Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(crate::state::ErrorBody {
                    schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
                    error: "agentRuntimeUnavailable",
                    retryable: true,
                }),
            ))
        }
    }
}

/// Ask the owner what it holds.
async fn sessions() -> Result<Vec<SessionView>, String> {
    let encoded: Vec<u8> = zbus::Connection::session()
        .await
        .map_err(|error| error.to_string())?
        .call_method(
            Some(AGENT.service),
            AGENT.object_path,
            Some(AGENT.interface),
            "Sessions",
            &(),
        )
        .await
        .map_err(|error| error.to_string())?
        .body()
        .deserialize()
        .map_err(|error| error.to_string())?;

    cybou_fabric::decode(&encoded).map_err(|error| error.to_string())
}
