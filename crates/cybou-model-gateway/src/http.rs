// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! OpenAI-compatible HTTP adapter. It parses compatibility data and owns no policy.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
};
use cybou_model_brokerd::ChatMessage;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::core::{GatewayCore, GatewayRefused, GatewayRequest};

const MAX_REQUEST_BYTES: usize = 1024 * 1024;
const DEFAULT_MAX_OUTPUT_TOKENS: u32 = 512;

/// Build the compatibility surface around a configured gateway core.
///
/// Binding and TLS belong to deployment. Keeping the router separate lets the first agent pack put
/// it on the capsule-only listener without this crate silently opening a host port.
pub fn router(core: Arc<GatewayCore>) -> Router {
    Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
        .with_state(core)
}

#[derive(Deserialize)]
struct WireRequest {
    model: String,
    messages: Vec<WireMessage>,
    #[serde(default, alias = "max_completion_tokens")]
    max_tokens: Option<u32>,
    #[serde(default)]
    stream: bool,
}

#[derive(Deserialize, Serialize)]
struct WireMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct WireResponse {
    id: String,
    object: &'static str,
    created: i64,
    model: String,
    choices: Vec<Choice>,
    usage: Usage,
}

#[derive(Serialize)]
struct Choice {
    index: u32,
    message: WireMessage,
    finish_reason: &'static str,
}

#[derive(Serialize)]
#[allow(
    clippy::struct_field_names,
    reason = "these three names are fixed by the OpenAI compatibility response"
)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u64,
}

#[derive(Serialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Serialize)]
struct ErrorBody {
    message: String,
    r#type: &'static str,
    code: &'static str,
}

async fn chat_completions(
    State(core): State<Arc<GatewayCore>>,
    headers: HeaderMap,
    Json(request): Json<WireRequest>,
) -> Response {
    let Some(bearer) = bearer(&headers) else {
        return refusal(&GatewayRefused::Unauthorized);
    };
    if request.stream {
        return refusal(&GatewayRefused::InvalidRequest(
            "streaming belongs to the later session stage",
        ));
    }
    let now = OffsetDateTime::now_utc();
    let request = GatewayRequest {
        request_id: Uuid::new_v4(),
        model_class: request.model,
        messages: request
            .messages
            .into_iter()
            .map(|message| ChatMessage {
                role: message.role,
                content: message.content,
            })
            .collect(),
        max_output_tokens: request.max_tokens.unwrap_or(DEFAULT_MAX_OUTPUT_TOKENS),
    };
    let bearer = bearer.to_owned();
    let completed =
        tokio::task::spawn_blocking(move || core.complete(&bearer, &request, now)).await;
    let Ok(completed) = completed else {
        return refusal(&GatewayRefused::Provider(
            cybou_model_brokerd::ChatRefused::WorkerFailed {
                provider: "gateway-worker".to_owned(),
                failure: cybou_model_brokerd::WorkerFailed::Unusable {
                    detail: "worker task stopped".to_owned(),
                },
            },
        ));
    };
    match completed {
        Ok(completion) => {
            let result = completion.result;
            let model = format!(
                "{}/{}",
                result.answered_by.family, result.answered_by.revision
            );
            Json(WireResponse {
                id: format!("chatcmpl-{}", result.request_id.simple()),
                object: "chat.completion",
                created: now.unix_timestamp(),
                model,
                choices: vec![Choice {
                    index: 0,
                    message: WireMessage {
                        role: "assistant".to_owned(),
                        content: result.content,
                    },
                    finish_reason: "stop",
                }],
                usage: Usage {
                    prompt_tokens: result.input_tokens,
                    completion_tokens: result.output_tokens,
                    total_tokens: u64::from(result.input_tokens) + u64::from(result.output_tokens),
                },
            })
            .into_response()
        }
        Err(refused) => refusal(&refused),
    }
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .filter(|token| !token.is_empty())
}

fn refusal(refused: &GatewayRefused) -> Response {
    let (status, kind, code) = match refused {
        GatewayRefused::Unauthorized => (
            StatusCode::UNAUTHORIZED,
            "authentication_error",
            "invalid_api_key",
        ),
        GatewayRefused::ModelClassNotGranted => (
            StatusCode::FORBIDDEN,
            "permission_error",
            "model_not_granted",
        ),
        GatewayRefused::InvalidRequest(_) => (
            StatusCode::BAD_REQUEST,
            "invalid_request_error",
            "invalid_request",
        ),
        GatewayRefused::BudgetExceeded => (
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limit_error",
            "lease_budget_exceeded",
        ),
        GatewayRefused::Provider(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            "server_error",
            "provider_unavailable",
        ),
    };
    (
        status,
        Json(ErrorEnvelope {
            error: ErrorBody {
                message: refused.to_string(),
                r#type: kind,
                code,
            },
        }),
    )
        .into_response()
}
