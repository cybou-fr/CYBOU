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
use cybou_protocol::model::{ModelIdentity, ModelManifest, SpendPolicy};
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
    // A model group whose name says it is free bills nothing, and the one below it bills anyway —
    // which is the case that matters, because a route that was declared free and then charges has
    // broken a promise rather than used up a budget.
    let group = body
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or("cybou-strong")
        .to_owned();
    let cost = if group == "cybou-free" {
        "0.0"
    } else {
        "0.0000101"
    };
    seen.lock().expect("seen").chat.push((auth(&headers), body));
    let mut response_headers = HeaderMap::new();
    response_headers.insert("x-litellm-response-cost", cost.parse().expect("header"));
    response_headers.insert(
        "x-litellm-model-id",
        "deployment-sha".parse().expect("header"),
    );
    response_headers.insert("x-litellm-model-group", group.parse().expect("header"));
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
                zero_cost: false,
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
            spend: SpendPolicy::Capped(20),
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

/// Start a fake proxy and return the address it is listening on.
async fn proxy(seen: &Shared) -> String {
    let app = Router::new()
        .route("/key/generate", post(key))
        .route("/key/delete", post(delete_key))
        .route("/v1/chat/completions", post(chat))
        .with_state(Arc::clone(seen));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind fake proxy");
    let address = listener.local_addr().expect("address").to_string();
    tokio::spawn(async move { axum::serve(listener, app).await.expect("serve") });
    address
}

fn asking(spend: SpendPolicy) -> ProviderChatRequest {
    ProviderChatRequest {
        request_id: Uuid::from_u128(11),
        model_class: "Free".to_owned(),
        messages: vec![ChatMessage {
            role: "user".to_owned(),
            content: "hello".to_owned(),
        }],
        max_output_tokens: 9,
        spend,
    }
}

fn worker_for(address: &str, group: &str, zero_cost: bool) -> LiteLlmWorker {
    LiteLlmWorker::new(
        manifest(),
        &format!("http://{address}"),
        MASTER.to_owned(),
        vec![LiteLlmRoute {
            model_class: "Free".to_owned(),
            model_group: group.to_owned(),
            zero_cost,
        }],
        NonZeroU64::new(10).expect("unit"),
        2_000,
    )
    .expect("worker")
}

#[tokio::test(flavor = "multi_thread")]
async fn a_zero_cost_request_is_answered_and_its_key_carries_no_budget() {
    // The whole point. Under the old integer this request was indistinguishable from a spent-out
    // one and the transport refused it, so the selection a person makes in order to use a free model
    // was the single selection that could never be served.
    //
    // The key carries no `max_budget` rather than a zero one, because installations disagree about
    // whether zero means nought or unlimited, and a bound that means opposite things on different
    // deployments is not a bound.
    let seen = Shared::default();
    let address = proxy(&seen).await;

    let answer = tokio::task::spawn_blocking(move || {
        worker_for(&address, "cybou-free", true).answer_chat(&asking(SpendPolicy::ZeroCostOnly))
    })
    .await
    .expect("blocking task")
    .expect("a free completion");

    assert_eq!(answer.content, "bounded answer");
    assert_eq!(answer.spend_units, 0);

    let recorded = seen.lock().expect("seen");
    assert_eq!(recorded.key.len(), 1);
    assert!(
        recorded.key[0].1.get("max_budget").is_none(),
        "a zero-cost key carries no budget field: {}",
        recorded.key[0].1
    );
    assert_eq!(recorded.key[0].1["models"], json!(["cybou-free"]));
    assert_eq!(recorded.key[0].1["max_parallel_requests"], 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_zero_cost_request_refuses_a_route_nobody_declared_free() {
    // Serving it from a billable route because it is probably cheap would spend money against a
    // selection that said none. The declaration is the operator's; Cybou cannot see a price list.
    let seen = Shared::default();
    let address = proxy(&seen).await;

    let refusal = tokio::task::spawn_blocking(move || {
        worker_for(&address, "cybou-free", false).answer_chat(&asking(SpendPolicy::ZeroCostOnly))
    })
    .await
    .expect("blocking task")
    .expect_err("a billable route may not serve a zero-cost request");

    assert!(matches!(
        refusal,
        cybou_model_brokerd::WorkerFailed::UnsupportedSurface
    ));
    assert!(
        seen.lock().expect("seen").key.is_empty(),
        "no key should have been minted for a route that may not answer"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_declared_free_route_that_bills_is_refused_rather_than_returned() {
    // The money is already gone by the time this is known, which is exactly why the answer is not
    // handed back: returning content somebody has now been charged for, having asked for none, would
    // make the whole policy cosmetic.
    let seen = Shared::default();
    let address = proxy(&seen).await;

    let refusal = tokio::task::spawn_blocking(move || {
        worker_for(&address, "cybou-free-that-bills", true)
            .answer_chat(&asking(SpendPolicy::ZeroCostOnly))
    })
    .await
    .expect("blocking task")
    .expect_err("a billed zero-cost completion is not a completion");

    // The charge travels with the refusal. A bare failure would leave the gateway unable to charge
    // the lease, so a session that had been billed would read as having spent nothing - the answer
    // withheld and the money hidden, which is the worse of the two halves left undone.
    match refusal {
        cybou_model_brokerd::WorkerFailed::PolicyViolatedAfterCharge {
            spend_units,
            ref detail,
        } => {
            assert!(spend_units > 0, "a violation with no charge is not one");
            assert!(detail.contains("cost nothing"), "{detail}");
        }
        other => panic!("a billed zero-cost completion was reported as {other:?}"),
    }
    assert_eq!(
        seen.lock().expect("seen").delete.len(),
        1,
        "the virtual key is still revoked when the promise was broken"
    );
}
