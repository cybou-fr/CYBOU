// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Contract test against an in-process fake `LiteLLM` proxy.

use std::{
    num::NonZeroU64,
    sync::{Arc, Mutex},
};

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
};
use cybou_model_brokerd::{ChatMessage, ProviderChatRequest, Worker};
use cybou_protocol::model::{ModelIdentity, ModelManifest};
use cybou_provider_litellm::{LiteLlmRoute, LiteLlmWorker};
use serde_json::{Value, json};
use uuid::Uuid;

const MASTER: &str = "sk-master-must-not-leak";
const VIRTUAL: &str = "sk-virtual-one-request";

#[derive(Default)]
struct Seen {
    key: Vec<(String, Value)>,
    chat: Vec<(String, Value)>,
    delete: Vec<(String, Value)>,
}

type Shared = Arc<Mutex<Seen>>;

async fn key(
    State(seen): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    seen.lock().expect("seen").key.push((auth(&headers), body));
    (StatusCode::OK, Json(json!({"key": VIRTUAL})))
}

async fn chat(
    State(seen): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    seen.lock().expect("seen").chat.push((auth(&headers), body));
    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        "x-litellm-response-cost",
        "0.0000101".parse().expect("header"),
    );
    response_headers.insert(
        "x-litellm-model-id",
        "deployment-sha".parse().expect("header"),
    );
    response_headers.insert(
        "x-litellm-model-group",
        "cybou-strong".parse().expect("header"),
    );
    response_headers.insert(
        "x-litellm-call-id",
        "proxy-call-42".parse().expect("header"),
    );
    (
        StatusCode::OK,
        response_headers,
        Json(json!({
            "model": "provider/model-revision",
            "choices": [{"message": {"content": "bounded answer"}}],
            "usage": {"prompt_tokens": 3, "completion_tokens": 2, "total_tokens": 5}
        })),
    )
}

async fn delete_key(
    State(seen): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    seen.lock()
        .expect("seen")
        .delete
        .push((auth(&headers), body));
    StatusCode::OK
}

fn auth(headers: &HeaderMap) -> String {
    headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned()
}

fn manifest() -> ModelManifest {
    ModelManifest {
        model_id: "litellm-adapter".to_owned(),
        identity: ModelIdentity {
            family: "litellm-proxy".to_owned(),
            revision: "chat-v1".to_owned(),
            artifact_sha256: [0x55; 32],
            quantization: None,
            backend: "http".to_owned(),
            template_version: 1,
        },
        tasks: Vec::new(),
        license: "MIT".to_owned(),
        languages: Vec::new(),
        min_ram_mb: 1,
        context_limit: 32_768,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn one_request_gets_one_bounded_key_and_complete_attribution() {
    let seen = Shared::default();
    let app = Router::new()
        .route("/key/generate", post(key))
        .route("/key/delete", post(delete_key))
        .route("/v1/chat/completions", post(chat))
        .with_state(Arc::clone(&seen));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind fake proxy");
    let address = listener.local_addr().expect("address");
    let server = tokio::spawn(async move { axum::serve(listener, app).await.expect("serve") });

    let answer = tokio::task::spawn_blocking(move || {
        let worker = LiteLlmWorker::new(
            manifest(),
            &format!("http://{address}"),
            MASTER.to_owned(),
            vec![LiteLlmRoute {
                model_class: "Strong".to_owned(),
                model_group: "cybou-strong".to_owned(),
            }],
            NonZeroU64::new(10).expect("unit"),
            2_000,
        )
        .expect("worker");
        assert!(!format!("{worker:?}").contains(MASTER));
        worker.answer_chat(&ProviderChatRequest {
            request_id: Uuid::from_u128(7),
            model_class: "Strong".to_owned(),
            messages: vec![ChatMessage {
                role: "user".to_owned(),
                content: "hello".to_owned(),
            }],
            max_output_tokens: 9,
            max_spend_units: 20,
        })
    })
    .await
    .expect("blocking task")
    .expect("completion");
    assert_eq!(answer.content, "bounded answer");
    assert_eq!(answer.spend_units, 2, "cost must round upward");
    let attribution = answer.upstream.expect("proxy attribution");
    assert_eq!(attribution.model_group, "cybou-strong");
    assert_eq!(attribution.deployment_id, "deployment-sha");
    assert_eq!(attribution.response_model, "provider/model-revision");
    assert_eq!(attribution.call_id, "proxy-call-42");

    let recorded = seen.lock().expect("seen");
    assert_eq!(recorded.key.len(), 1);
    assert_eq!(recorded.key[0].0, format!("Bearer {MASTER}"));
    assert_eq!(recorded.key[0].1["models"], json!(["cybou-strong"]));
    assert_eq!(recorded.key[0].1["max_budget"], json!(0.0002));
    assert_eq!(recorded.key[0].1["max_parallel_requests"], 1);
    assert_eq!(recorded.key[0].1["duration"], "5m");
    assert_eq!(recorded.chat.len(), 1);
    assert_eq!(recorded.chat[0].0, format!("Bearer {VIRTUAL}"));
    assert_eq!(recorded.chat[0].1["model"], "cybou-strong");
    assert_eq!(recorded.chat[0].1["max_tokens"], 9);
    assert_eq!(recorded.chat[0].1["stream"], false);
    assert!(!recorded.chat[0].1.to_string().contains(MASTER));
    assert_eq!(recorded.delete.len(), 1);
    assert_eq!(recorded.delete[0].0, format!("Bearer {MASTER}"));
    assert_eq!(recorded.delete[0].1["keys"], json!([VIRTUAL]));
    drop(recorded);
    server.abort();
}
