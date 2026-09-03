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
//! Launch remains a proxy as well. The gateway establishes that the HTTP request belongs to a local
//! or authenticated seat, then carries the caller's selection whole to `Agent1`. It never reads the
//! profile catalogue or computes admission: the owner does both, or refuses.

use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
    http::StatusCode,
};
use cybou_fabric::AGENT;
use cybou_protocol::agent::{LaunchRequest, SessionView};
use uuid::Uuid;

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

/// Return operator-approved profile offers and launch readiness.
///
/// # Errors
///
/// Refuses with `401` when sign-in is required, or `503` when the runtime is unavailable.
pub async fn agent_offers_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
) -> Result<
    Json<cybou_protocol::agent::AgentOffersResponse>,
    (StatusCode, Json<crate::state::ErrorBody>),
> {
    if !state.may_read_mind(&headers) {
        return Err(GatewayState::sign_in_required());
    }

    match offers().await {
        Ok(offers) => Ok(Json(offers)),
        Err(why) => {
            eprintln!(
                "[cybou-web-gateway] {} offers query failed: {why}",
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

/// Ask the owner to launch one profile-bounded agent session.
///
/// # Errors
///
/// Refuses public and signed-out callers before D-Bus is touched. Owner policy refusals are `403`,
/// exhausted host capacity is `409`, and an unavailable runtime is `503`.
pub async fn launch_agent_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(request): Json<LaunchRequest>,
) -> Result<(StatusCode, Json<SessionView>), (StatusCode, Json<crate::state::ErrorBody>)> {
    if state.shell_seat(&headers).is_none() {
        return Err(GatewayState::sign_in_required());
    }

    match launch(request).await {
        Ok(session) => Ok((StatusCode::ACCEPTED, Json(session))),
        Err(why) => {
            eprintln!("[cybou-web-gateway] Agent1 refused a launch: {why}");
            let (status, error, retryable) = if why.contains("LimitsExceeded") {
                (StatusCode::CONFLICT, "agentCapacityExceeded", true)
            } else if why.contains("AccessDenied") || why.contains("InvalidArgs") {
                (StatusCode::FORBIDDEN, "agentLaunchRefused", false)
            } else {
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "agentRuntimeUnavailable",
                    true,
                )
            };
            Err((
                status,
                Json(crate::state::ErrorBody {
                    schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
                    error,
                    retryable,
                }),
            ))
        }
    }
}

/// Ask the owner to end one session, and answer only after teardown is confirmed.
///
/// # Errors
///
/// Refuses public and signed-out callers before D-Bus is touched. An invalid capsule identity is
/// `400`, an unconfirmed teardown is `409`, and an unavailable runtime is `503`. Stopping an
/// already-ended or unknown session is idempotent and returns `204`.
pub async fn stop_agent_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Path(capsule_id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<crate::state::ErrorBody>)> {
    if state.shell_seat(&headers).is_none() {
        return Err(GatewayState::sign_in_required());
    }
    let capsule_id = Uuid::parse_str(&capsule_id)
        .map_err(|_| agent_error(StatusCode::BAD_REQUEST, "agentIdentityInvalid", false))?;

    match stop(&capsule_id.to_string()).await {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => match sessions().await {
            Ok(sessions)
                if sessions
                    .iter()
                    .any(|session| session.capsule_id == capsule_id && session.is_live()) =>
            {
                Err(agent_error(
                    StatusCode::CONFLICT,
                    "agentStopUnconfirmed",
                    true,
                ))
            }
            Ok(_) => Ok(StatusCode::NO_CONTENT),
            Err(why) => {
                eprintln!("[cybou-web-gateway] could not verify Agent1 Stop: {why}");
                Err(agent_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "agentRuntimeUnavailable",
                    true,
                ))
            }
        },
        Err(why) => {
            eprintln!("[cybou-web-gateway] Agent1 Stop failed: {why}");
            Err(agent_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "agentRuntimeUnavailable",
                true,
            ))
        }
    }
}

fn agent_error(
    status: StatusCode,
    error: &'static str,
    retryable: bool,
) -> (StatusCode, Json<crate::state::ErrorBody>) {
    (
        status,
        Json(crate::state::ErrorBody {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            error,
            retryable,
        }),
    )
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

/// Ask the owner what profiles and capabilities it offers.
async fn offers() -> Result<cybou_protocol::agent::AgentOffersResponse, String> {
    let encoded: Vec<u8> = zbus::Connection::session()
        .await
        .map_err(|error| error.to_string())?
        .call_method(
            Some(AGENT.service),
            AGENT.object_path,
            Some(AGENT.interface),
            "Offers",
            &(),
        )
        .await
        .map_err(|error| error.to_string())?
        .body()
        .deserialize()
        .map_err(|error| error.to_string())?;

    cybou_fabric::decode(&encoded).map_err(|error| error.to_string())
}

/// Carry one launch selection to the owner without adding a grant to it.
async fn launch(request: LaunchRequest) -> Result<SessionView, String> {
    let encoded = cybou_fabric::encode(&request).map_err(|error| error.to_string())?;
    let reply: Vec<u8> = zbus::Connection::session()
        .await
        .map_err(|error| error.to_string())?
        .call_method(
            Some(AGENT.service),
            AGENT.object_path,
            Some(AGENT.interface),
            "Launch",
            &(encoded,),
        )
        .await
        .map_err(|error| error.to_string())?
        .body()
        .deserialize()
        .map_err(|error| error.to_string())?;
    cybou_fabric::decode(&reply).map_err(|error| error.to_string())
}

/// Ask the owner to stop one session. `false` still needs a canonical listing: it can mean either
/// that the session was already absent or that teardown could not be proven.
async fn stop(capsule_id: &str) -> Result<bool, String> {
    zbus::Connection::session()
        .await
        .map_err(|error| error.to_string())?
        .call_method(
            Some(AGENT.service),
            AGENT.object_path,
            Some(AGENT.interface),
            "Stop",
            &(capsule_id.to_owned(),),
        )
        .await
        .map_err(|error| error.to_string())?
        .body()
        .deserialize()
        .map_err(|error| error.to_string())
}

/// Ask the owner to perform an action (Freeze, Resume, Quarantine, Stop) on a live capsule.
///
/// # Errors
///
/// Reports the owner unavailable when it cannot be read.
pub async fn capsule_action_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Path(capsule_id): Path<String>,
    Json(request): Json<cybou_web_contracts::CapsuleControlRequest>,
) -> Result<StatusCode, (StatusCode, Json<crate::state::ErrorBody>)> {
    if state.shell_seat(&headers).is_none() {
        return Err(GatewayState::sign_in_required());
    }
    let capsule_id = Uuid::parse_str(&capsule_id)
        .map_err(|_| agent_error(StatusCode::BAD_REQUEST, "agentIdentityInvalid", false))?;

    let action_str = match request.action {
        cybou_protocol::agent::CapsuleAction::Freeze => "freeze",
        cybou_protocol::agent::CapsuleAction::Resume => "resume",
        cybou_protocol::agent::CapsuleAction::Quarantine => "quarantine",
        cybou_protocol::agent::CapsuleAction::Stop => "stop",
    };

    match action(&capsule_id.to_string(), action_str).await {
        Ok(true) => Ok(StatusCode::OK),
        Ok(false) => Err(agent_error(
            StatusCode::NOT_FOUND,
            "agentSessionNotFound",
            false,
        )),
        Err(why) => {
            eprintln!("[cybou-web-gateway] Agent1 Action failed: {why}");
            Err(agent_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "agentRuntimeUnavailable",
                true,
            ))
        }
    }
}

/// Retrieve live telemetry for an active capsule.
///
/// # Errors
///
/// Refuses when the request may not read Mind, and reports unavailable when the owner cannot be read.
pub async fn capsule_telemetry_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Path(capsule_id): Path<String>,
) -> Result<
    Json<cybou_web_contracts::CapsuleTelemetryProjection>,
    (StatusCode, Json<crate::state::ErrorBody>),
> {
    if !state.may_read_mind(&headers) {
        return Err(GatewayState::sign_in_required());
    }
    let capsule_id = Uuid::parse_str(&capsule_id)
        .map_err(|_| agent_error(StatusCode::BAD_REQUEST, "agentIdentityInvalid", false))?;

    match telemetry(&capsule_id.to_string()).await {
        Ok(tel) => Ok(Json(cybou_web_contracts::CapsuleTelemetryProjection {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            telemetry: tel,
        })),
        Err(why) => {
            eprintln!("[cybou-web-gateway] Agent1 Telemetry query failed: {why}");
            Err(agent_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "agentTelemetryUnavailable",
                true,
            ))
        }
    }
}

async fn action(capsule_id: &str, action: &str) -> Result<bool, String> {
    zbus::Connection::session()
        .await
        .map_err(|error| error.to_string())?
        .call_method(
            Some(AGENT.service),
            AGENT.object_path,
            Some(AGENT.interface),
            "Action",
            &(capsule_id.to_owned(), action.to_owned()),
        )
        .await
        .map_err(|error| error.to_string())?
        .body()
        .deserialize()
        .map_err(|error| error.to_string())
}

async fn telemetry(
    capsule_id: &str,
) -> Result<cybou_protocol::agent::CapsuleTelemetryRecord, String> {
    let reply: Vec<u8> = zbus::Connection::session()
        .await
        .map_err(|error| error.to_string())?
        .call_method(
            Some(AGENT.service),
            AGENT.object_path,
            Some(AGENT.interface),
            "Telemetry",
            &(capsule_id.to_owned(),),
        )
        .await
        .map_err(|error| error.to_string())?
        .body()
        .deserialize()
        .map_err(|error| error.to_string())?;
    cybou_fabric::decode(&reply).map_err(|error| error.to_string())
}
