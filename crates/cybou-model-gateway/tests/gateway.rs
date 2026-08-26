// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! ADR-0043 acceptance path through the public gateway and shared provider interfaces.

use std::sync::{Arc, Mutex};

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use cybou_capsule::{
    CapabilityProfile, Lease, LeaseRequest, ModelGrant, ResourceBudget, SpendPolicy, Workspace,
    issue_lease,
};
use cybou_model_brokerd::{
    BrokerCore, ChatMessage, ProviderChatOutput, ProviderChatRequest, UsageSubject, Worker,
    WorkerFailed,
};
use cybou_model_gateway::{GatewayCore, GatewayRefused, GatewayRequest, TokenPolicy, router};
use cybou_protocol::model::{
    ALL_MODEL_TASKS, ModelIdentity, ModelInput, ModelManifest, ModelOutput, ModelRequest,
    ModelRequirements, ModelRoute, ModelTask,
};
use http_body_util::BodyExt as _;
use serde_json::json;
use time::{Duration, OffsetDateTime};
use tower::ServiceExt as _;
use uuid::Uuid;

struct BothSurfaces;

impl Worker for BothSurfaces {
    fn manifest(&self) -> &ModelManifest {
        static MANIFEST: std::sync::LazyLock<ModelManifest> =
            std::sync::LazyLock::new(|| ModelManifest {
                model_id: "shared-stub".to_owned(),
                identity: ModelIdentity {
                    family: "shared-stub".to_owned(),
                    revision: "1".to_owned(),
                    artifact_sha256: [0xA5; 32],
                    quantization: None,
                    backend: "test".to_owned(),
                    template_version: 1,
                },
                tasks: vec![ModelTask::InterpretActV1],
                license: "MIT".to_owned(),
                languages: vec!["en".to_owned()],
                min_ram_mb: 1,
                context_limit: 4096,
            });
        &MANIFEST
    }

    fn answer(&self, _request: &ModelRequest) -> Result<ModelOutput, WorkerFailed> {
        Ok(ModelOutput::CandidateAct {
            kind: "inspect".to_owned(),
            subject: "workspace".to_owned(),
        })
    }

    fn answer_chat(
        &self,
        _request: &ProviderChatRequest,
    ) -> Result<ProviderChatOutput, WorkerFailed> {
        Ok(ProviderChatOutput {
            content: "done".to_owned(),
            input_tokens: 2,
            output_tokens: 1,
            spend_units: 7,
            upstream: None,
        })
    }
}

struct UnreachableChat;

/// A provider that answered, billed, and broke the policy it was serving under.
struct BilledThenRefused;

impl Worker for BilledThenRefused {
    fn manifest(&self) -> &ModelManifest {
        BothSurfaces.manifest()
    }

    fn answer(&self, request: &ModelRequest) -> Result<ModelOutput, WorkerFailed> {
        BothSurfaces.answer(request)
    }

    fn answer_chat(
        &self,
        _request: &ProviderChatRequest,
    ) -> Result<ProviderChatOutput, WorkerFailed> {
        Err(WorkerFailed::PolicyViolatedAfterCharge {
            spend_units: 7,
            detail: "the route was declared to cost nothing and billed".to_owned(),
        })
    }
}

impl Worker for UnreachableChat {
    fn manifest(&self) -> &ModelManifest {
        BothSurfaces.manifest()
    }

    fn answer(&self, request: &ModelRequest) -> Result<ModelOutput, WorkerFailed> {
        BothSurfaces.answer(request)
    }

    fn answer_chat(
        &self,
        _request: &ProviderChatRequest,
    ) -> Result<ProviderChatOutput, WorkerFailed> {
        Err(WorkerFailed::NotReady)
    }
}

fn providers() -> Arc<BrokerCore> {
    let mut providers = BrokerCore::new();
    providers.register_provider(
        ModelRoute {
            provider: "shared-provider".to_owned(),
            external_boundary: false,
            sensitivity_ceiling: 3,
            tasks: vec![ModelTask::InterpretActV1],
            context_limit: 4096,
        },
        vec!["Strong".to_owned()],
        Box::new(BothSurfaces),
    );
    Arc::new(providers)
}

fn at(offset: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1_787_000_000 + offset).expect("fixed instant")
}

fn capsule_lease() -> Arc<Mutex<Lease>> {
    let mut profile = CapabilityProfile::bounded(
        "agent-development",
        ResourceBudget {
            memory_mib: 1024,
            cpus: 1,
            tasks_max: 64,
            lifetime: Duration::hours(4),
        },
    )
    .expect("profile");
    profile.model = Some(ModelGrant {
        class: "Strong".to_owned(),
        spend: SpendPolicy::Capped(20),
    });
    Arc::new(Mutex::new(
        issue_lease(
            LeaseRequest {
                selected_profile: profile,
                capsule_id: Uuid::from_u128(8472),
                agent: "opencode".to_owned(),
                workspace: Workspace::at("/srv/project"),
            },
            at(0),
        )
        .expect("lease"),
    ))
}

fn policy() -> TokenPolicy {
    TokenPolicy {
        local_only: true,
        sensitivity: 2,
        max_output_tokens: 64,
        token_limit: 1024,
    }
}

fn request() -> GatewayRequest {
    GatewayRequest {
        request_id: Uuid::from_u128(99),
        model_class: "Strong".to_owned(),
        messages: vec![ChatMessage {
            role: "user".to_owned(),
            content: "work".to_owned(),
        }],
        max_output_tokens: 16,
    }
}

fn typed_request() -> ModelRequest {
    ModelRequest {
        request_id: Uuid::from_u128(11),
        consumer: "meaningd".to_owned(),
        task: ModelTask::InterpretActV1,
        delivery: Uuid::from_u128(12),
        requirements: ModelRequirements {
            local_only: true,
            sensitivity_ceiling: 3,
            max_latency_ms: 1000,
            max_input_tokens: 128,
            max_output_tokens: 32,
            max_ram_mb: 1024,
        },
        input: ModelInput::Utterance {
            text: "inspect".to_owned(),
        },
        carries_sensitivity: 1,
        input_tokens: 2,
    }
}

#[test]
fn two_surfaces_use_one_worker_policy_and_ledger_without_widening_modelbroker1() {
    // N1 is held at the protocol boundary: the exact closed vocabulary is untouched.
    assert_eq!(
        ALL_MODEL_TASKS,
        &[
            ModelTask::InterpretActV1,
            ModelTask::RealizeResponsePlanV1,
            ModelTask::EmbedTextV1,
            ModelTask::RerankV1,
            ModelTask::SummarizeEvidenceV1,
            ModelTask::ProposeDesktopPlanV1,
            ModelTask::DiagnoseSystemV1,
        ]
    );

    let providers = providers();
    providers.submit(&typed_request()).expect("typed surface");
    let gateway = GatewayCore::new(providers.clone());
    let lease = capsule_lease();
    let token = gateway
        .issue_token(lease.clone(), Uuid::from_u128(55), policy(), at(1))
        .expect("token");
    let completion = gateway
        .complete(token.expose_secret(), &request(), at(2))
        .expect("agent surface");
    assert_eq!(completion.result.content, "done");
    assert_eq!(lease.lock().expect("lease").model_spent(), 7);

    let usage = providers.recent_usage();
    assert_eq!(usage.len(), 2);
    assert!(matches!(usage[0].subject, UsageSubject::Mind { .. }));
    assert_eq!(usage[0].provider, "shared-provider");
    assert!(matches!(
        usage[1].subject,
        UsageSubject::Agent {
            capsule_id,
            ref agent,
            task_id,
        } if capsule_id == Uuid::from_u128(8472)
            && agent == "opencode"
            && task_id == Uuid::from_u128(55)
    ));
    assert_eq!(usage[1].model.artifact_sha256, [0xA5; 32]);
    assert_eq!(usage[1].spend_units, 7);
}

#[test]
fn a_token_is_only_its_capsule_lease_class_lifetime_and_budget() {
    let gateway = GatewayCore::new(providers());
    let lease = capsule_lease();
    let token = gateway
        .issue_token(lease.clone(), Uuid::from_u128(55), policy(), at(1))
        .expect("token");
    assert!(!format!("{token:?}").contains(token.expose_secret()));
    assert_eq!(
        gateway.complete("wrong", &request(), at(2)),
        Err(GatewayRefused::Unauthorized)
    );

    let mut wrong_class = request();
    wrong_class.model_class = "Fast".to_owned();
    assert_eq!(
        gateway.complete(token.expose_secret(), &wrong_class, at(2)),
        Err(GatewayRefused::ModelClassNotGranted)
    );

    lease.lock().expect("lease").revoke(at(3));
    assert_eq!(
        gateway.complete(token.expose_secret(), &request(), at(4)),
        Err(GatewayRefused::Unauthorized)
    );
}

#[test]
fn reservation_and_spend_are_enforced_before_a_second_completion() {
    let providers = providers();
    let gateway = GatewayCore::new(providers.clone());
    let lease = capsule_lease();
    let mut narrow = policy();
    narrow.token_limit = 32;
    let token = gateway
        .issue_token(lease.clone(), Uuid::from_u128(55), narrow, at(1))
        .expect("token");

    let mut too_large = request();
    too_large.max_output_tokens = 30;
    assert_eq!(
        gateway.complete(token.expose_secret(), &too_large, at(2)),
        Err(GatewayRefused::BudgetExceeded)
    );
    assert!(providers.recent_usage().is_empty());

    gateway
        .complete(token.expose_secret(), &request(), at(2))
        .expect("first completion");
    gateway
        .complete(token.expose_secret(), &request(), at(3))
        .expect("second completion");
    assert_eq!(lease.lock().expect("lease").model_spent(), 14);
    assert_eq!(
        gateway.complete(token.expose_secret(), &request(), at(4)),
        Err(GatewayRefused::Provider(
            cybou_model_brokerd::ChatRefused::ExceededReservation
        ))
    );
    assert_eq!(lease.lock().expect("lease").model_spent(), 14);
}

#[test]
fn an_unreachable_selected_provider_is_named_and_never_silently_substituted() {
    let route = |provider: &str| ModelRoute {
        provider: provider.to_owned(),
        external_boundary: false,
        sensitivity_ceiling: 3,
        tasks: vec![ModelTask::InterpretActV1],
        context_limit: 4096,
    };
    let mut providers = BrokerCore::new();
    providers.register_provider(
        route("declared-primary"),
        vec!["Strong".to_owned()],
        Box::new(UnreachableChat),
    );
    providers.register_provider(
        route("undeclared-fallback"),
        vec!["Strong".to_owned()],
        Box::new(BothSurfaces),
    );
    let providers = Arc::new(providers);
    let gateway = GatewayCore::new(providers.clone());
    let token = gateway
        .issue_token(capsule_lease(), Uuid::from_u128(55), policy(), at(1))
        .expect("token");

    assert_eq!(
        gateway.complete(token.expose_secret(), &request(), at(2)),
        Err(GatewayRefused::Provider(
            cybou_model_brokerd::ChatRefused::WorkerFailed {
                provider: "declared-primary".to_owned(),
                failure: WorkerFailed::NotReady,
            }
        ))
    );
    assert!(
        providers.recent_usage().is_empty(),
        "a second provider silently answered"
    );
}

#[tokio::test]
async fn openai_compatible_http_shape_requires_the_ephemeral_bearer() {
    let now = OffsetDateTime::now_utc();
    let mut profile = CapabilityProfile::bounded(
        "http-agent",
        ResourceBudget {
            memory_mib: 1024,
            cpus: 1,
            tasks_max: 64,
            lifetime: Duration::hours(1),
        },
    )
    .expect("profile");
    profile.model = Some(ModelGrant {
        class: "Strong".to_owned(),
        spend: SpendPolicy::Capped(20),
    });
    let lease = Arc::new(Mutex::new(
        issue_lease(
            LeaseRequest {
                selected_profile: profile,
                capsule_id: Uuid::from_u128(8472),
                agent: "opencode".to_owned(),
                workspace: Workspace::at("/srv/project"),
            },
            now,
        )
        .expect("lease"),
    ));
    let core = Arc::new(GatewayCore::new(providers()));
    let token = core
        .issue_token(lease, Uuid::from_u128(55), policy(), now)
        .expect("token");
    let app = router(core);

    let body = json!({
        "model": "Strong",
        "messages": [{"role": "user", "content": "work"}],
        "max_tokens": 16
    })
    .to_string();
    let unauthorized = app
        .clone()
        .oneshot(
            Request::post("/v1/chat/completions")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.clone()))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let response = app
        .oneshot(
            Request::post("/v1/chat/completions")
                .header(header::CONTENT_TYPE, "application/json")
                .header(
                    header::AUTHORIZATION,
                    format!("Bearer {}", token.expose_secret()),
                )
                .body(Body::from(body))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let response: serde_json::Value = serde_json::from_slice(&body).expect("JSON");
    assert_eq!(response["object"], "chat.completion");
    assert_eq!(response["choices"][0]["message"]["role"], "assistant");
    assert_eq!(response["choices"][0]["message"]["content"], "done");
    assert_eq!(response["usage"]["total_tokens"], 3);
}

#[tokio::test]
async fn a_streaming_request_gets_an_event_stream_rather_than_a_refusal() {
    // A coding agent asks for a stream and treats a refusal as a broken endpoint, so refusing this
    // was not "no streaming yet" — it was an agent that could not run at all.
    //
    // What comes back is the real event shape and not incremental delivery: the completion is
    // produced whole and charged before the first byte leaves. That order is the safe one. Charging
    // as tokens arrive means a ceiling can be reached mid-sentence, and nothing can unsend what has
    // already gone.
    let now = OffsetDateTime::now_utc();
    let mut profile = CapabilityProfile::bounded(
        "streaming-agent",
        ResourceBudget {
            memory_mib: 1024,
            cpus: 1,
            tasks_max: 64,
            lifetime: Duration::hours(1),
        },
    )
    .expect("profile");
    profile.model = Some(ModelGrant {
        class: "Strong".to_owned(),
        spend: SpendPolicy::Capped(20),
    });
    let lease = Arc::new(Mutex::new(
        issue_lease(
            LeaseRequest {
                selected_profile: profile,
                capsule_id: Uuid::from_u128(8473),
                agent: "opencode".to_owned(),
                workspace: Workspace::at("/srv/project"),
            },
            now,
        )
        .expect("lease"),
    ));
    let core = Arc::new(GatewayCore::new(providers()));
    let token = core
        .issue_token(Arc::clone(&lease), Uuid::from_u128(56), policy(), now)
        .expect("token");
    let app = router(core);

    let response = app
        .oneshot(
            Request::post("/v1/chat/completions")
                .header(header::CONTENT_TYPE, "application/json")
                .header(
                    header::AUTHORIZATION,
                    format!("Bearer {}", token.expose_secret()),
                )
                .body(Body::from(
                    json!({
                        "model": "Strong",
                        "messages": [{"role": "user", "content": "work"}],
                        "max_tokens": 16,
                        "stream": true
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .expect("a content type"),
        "text/event-stream"
    );
    let body = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let body = String::from_utf8(body.to_vec()).expect("utf-8");

    let events: Vec<&str> = body
        .split("\n\n")
        .filter(|event| !event.is_empty())
        .map(|event| event.trim_start_matches("data: "))
        .collect();
    assert_eq!(events.len(), 4, "{body}");
    assert_eq!(*events.last().expect("a sentinel"), "[DONE]");

    let parsed: Vec<serde_json::Value> = events[..3]
        .iter()
        .map(|event| serde_json::from_str(event).expect("JSON"))
        .collect();
    for event in &parsed {
        assert_eq!(event["object"], "chat.completion.chunk");
    }
    assert_eq!(parsed[0]["choices"][0]["delta"]["role"], "assistant");
    assert_eq!(parsed[1]["choices"][0]["delta"]["content"], "done");
    assert_eq!(parsed[2]["choices"][0]["finish_reason"], "stop");
    assert_eq!(parsed[2]["usage"]["total_tokens"], 3);

    // Charged once, before anything was sent. A stream that billed as it went would have had to
    // decide what to do about a ceiling reached halfway through a sentence.
    assert_eq!(
        lease.lock().expect("lease").model_spent(),
        7,
        "the completion's whole cost, charged once, before any of it was sent"
    );
}

#[test]
fn a_refusal_that_cost_money_still_charges_the_lease() {
    // The failure this closes: the provider billed, the answer was withheld, and the charge went
    // nowhere - so a person who selected a spending bound would be told they had spent nothing
    // while their account said otherwise. Money already gone is a fact about the session, not a
    // property of whether the answer was delivered.
    let mut providers = BrokerCore::new();
    providers.register_provider(
        ModelRoute {
            provider: "shared-provider".to_owned(),
            external_boundary: false,
            sensitivity_ceiling: 3,
            tasks: vec![ModelTask::InterpretActV1],
            context_limit: 4096,
        },
        vec!["Strong".to_owned()],
        Box::new(BilledThenRefused),
    );
    let core = GatewayCore::new(Arc::new(providers));

    let lease = capsule_lease();
    let token = core
        .issue_token(Arc::clone(&lease), Uuid::from_u128(9), policy(), at(0))
        .expect("a token");

    let refused = core
        .complete(token.expose_secret(), &request(), at(1))
        .expect_err("the answer is withheld");
    assert!(matches!(refused, GatewayRefused::Provider(_)));

    assert_eq!(
        lease.lock().expect("the lease").model_spent(),
        7,
        "the charge reached the ledger even though the content did not reach the agent"
    );
}

/// A lease that spends nothing, on a route that costs nothing.
fn zero_cost_lease() -> Arc<Mutex<Lease>> {
    let mut profile = CapabilityProfile::bounded(
        "free-only",
        ResourceBudget {
            memory_mib: 512,
            cpus: 1,
            tasks_max: 64,
            lifetime: time::Duration::hours(1),
        },
    )
    .expect("profile");
    profile.model = Some(ModelGrant {
        class: "Strong".to_owned(),
        spend: SpendPolicy::ZeroCostOnly,
    });
    Arc::new(Mutex::new(
        issue_lease(
            LeaseRequest {
                selected_profile: profile,
                capsule_id: Uuid::from_u128(8473),
                agent: "opencode".to_owned(),
                workspace: Workspace::at("/srv/project"),
            },
            at(0),
        )
        .expect("lease"),
    ))
}

#[test]
fn a_zero_cost_route_that_billed_once_is_closed_to_everything_after() {
    // The second half of the zero-cost promise. The first was that a billed completion is charged
    // rather than lost; this is that it is also the last. Without it a route that broke its promise
    // once went on breaking it, billing a person who had selected nothing, one refused answer at a
    // time - and each refusal looked like the system working.
    let mut providers = BrokerCore::new();
    providers.register_provider(
        ModelRoute {
            provider: "shared-provider".to_owned(),
            external_boundary: false,
            sensitivity_ceiling: 3,
            tasks: vec![ModelTask::InterpretActV1],
            context_limit: 4096,
        },
        vec!["Strong".to_owned()],
        Box::new(BilledThenRefused),
    );
    let core = GatewayCore::new(Arc::new(providers));

    let lease = zero_cost_lease();
    let token = core
        .issue_token(Arc::clone(&lease), Uuid::from_u128(11), policy(), at(0))
        .expect("a token");

    // The first call reaches the provider, is billed, and is refused for breaking the policy.
    let first = core
        .complete(token.expose_secret(), &request(), at(1))
        .expect_err("the answer is withheld");
    assert!(matches!(first, GatewayRefused::Provider(_)));
    assert_eq!(lease.lock().expect("lease").model_spent(), 7);

    // The second never reaches one. The ledger already said stop, and now somebody asks it.
    let second = core
        .complete(token.expose_secret(), &request(), at(2))
        .expect_err("the route is closed");
    assert!(
        matches!(second, GatewayRefused::BudgetExceeded),
        "a closed grant must be refused before a provider is called, not after: {second:?}"
    );
    assert_eq!(
        lease.lock().expect("lease").model_spent(),
        7,
        "nothing further was spent"
    );
}
