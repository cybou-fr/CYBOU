// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! ADR-0043 acceptance path through the public gateway and shared provider interfaces.

use std::sync::{Arc, Mutex};

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use cybou_capsule::{
    CapabilityProfile, Lease, LeaseRequest, ModelGrant, ResourceBudget, Workspace, issue_lease,
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
        })
    }
}

struct UnreachableChat;

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
        spend_limit: 20,
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
        spend_limit: 20,
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
