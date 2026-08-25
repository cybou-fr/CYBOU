// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! OpenAI-compatible HTTP adapter. It parses compatibility data and owns no policy.
//!
//! ## Streaming, and what this one is not
//!
//! `stream: true` is answered with a real `text/event-stream` in the shape every OpenAI-compatible
//! client expects, and the completion inside it is produced whole before the first byte leaves. It
//! is protocol compatibility, not incremental delivery: nothing arrives sooner than it would have.
//!
//! That is a stage and it is written down rather than implied, because a client cannot tell the
//! difference and would be entitled to assume otherwise. It matters because the alternative was
//! refusing the request outright, which is not "no streaming yet" — it is an agent that cannot run
//! at all, since a coding agent asks for a stream and treats a refusal as a broken endpoint.
//!
//! Doing it in this order is also the safe direction. The lease is charged before any of the
//! response is sent, so a completion that would exceed the ceiling is refused while a refusal is
//! still possible. Delivering tokens as they arrive means charging as they arrive, and a ceiling
//! reached mid-sentence cannot be honoured by unsending what has already gone — that needs a
//! mid-stream cancellation design rather than a smaller change here.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
};
use cybou_model_brokerd::{AgentChatResult, ChatMessage};
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
    let streaming = request.stream;
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
            if streaming {
                return stream(&result, &model, now);
            }
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

/// The same completion, in the event shape a compatibility client expects.
///
/// Three events and a sentinel, because that is the smallest exchange every OpenAI-compatible client
/// accepts: the role, the content, and a finish with usage on it. Emitting the content as one chunk
/// rather than many is honest about what happened — the text was produced whole, and cutting it into
/// pieces after the fact would be staging an arrival that already took place.
fn stream(result: &AgentChatResult, model: &str, now: OffsetDateTime) -> Response {
    let id = format!("chatcmpl-{}", result.request_id.simple());
    let created = now.unix_timestamp();
    let chunk = |delta: serde_json::Value, finish: Option<&str>, usage: Option<Usage>| {
        let mut value = serde_json::json!({
            "id": id,
            "object": "chat.completion.chunk",
            "created": created,
            "model": model,
            "choices": [{"index": 0, "delta": delta, "finish_reason": finish}],
        });
        if let Some(usage) = usage
            && let Ok(usage) = serde_json::to_value(usage)
            && let Some(object) = value.as_object_mut()
        {
            object.insert("usage".to_owned(), usage);
        }
        format!("data: {value}\n\n")
    };

    let body = chunk(serde_json::json!({"role": "assistant"}), None, None)
        + &chunk(serde_json::json!({"content": result.content}), None, None)
        + &chunk(
            serde_json::json!({}),
            Some("stop"),
            Some(Usage {
                prompt_tokens: result.input_tokens,
                completion_tokens: result.output_tokens,
                total_tokens: u64::from(result.input_tokens) + u64::from(result.output_tokens),
            }),
        )
        + "data: [DONE]\n\n";

    (
        [
            (axum::http::header::CONTENT_TYPE, "text/event-stream"),
            // No store and no buffering. A proxy that held this until it was complete would turn a
            // stream into a slower non-stream, which is the one thing worse than not streaming.
            (axum::http::header::CACHE_CONTROL, "no-store"),
        ],
        body,
    )
        .into_response()
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
