// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Bounded HTTP boundary between Living Canvas and the runtime owners it presents.

use std::{path::PathBuf, sync::Arc};

use axum::{
    Router,
    http::{HeaderName, HeaderValue},
    routing::{any, delete, get, post},
};
use cybou_web_contracts::{SessionProjection, WEB_SCHEMA_V1};
use time::{Duration as TimeDuration, OffsetDateTime, format_description::well_known::Rfc3339};
use tower_http::{
    services::{ServeDir, ServeFile},
    set_header::SetResponseHeaderLayer,
};
use uuid::Uuid;

pub mod access;
#[cfg(target_os = "linux")]
pub mod auth_socket;
/// Deep cross-subsystem Cognitive Graph & Canonical Event1 Journal provider.
pub mod cognitive_hub;
pub mod disclose;
pub mod fixture;
#[cfg(target_os = "linux")]
pub mod host_files_socket;
/// Insight summaries.
pub mod insight;
/// Meaning and dialogue parsing engine.
pub mod meaning_hub;
/// Lifelong learning candidate evaluation, artifact lineages, and capability governance.
pub mod learning_hub;
/// Desktop notifications hub.
pub mod notifications_hub;
/// Long-running server operations manager.
pub mod operations_hub;
/// Personal pack (Mail, Calendar, Notes, Contacts).
pub mod personal_hub;
/// Real-time D-Bus presence subscription.
pub mod presence_zbus;
/// Sensitive information redaction.
pub mod redact;
/// HTTP route handlers.
pub mod routes;
/// Bounded sandboxed shell sessions.
pub mod shells;
/// Gateway application state.
pub mod state;
/// System services, processes, storage, network, and packages provider.
pub mod system_hub;

pub use access::{
    CredentialVerifier, LoginOutcome, LoginRequest, Session, Sessions, VerifiedAccount,
};
pub use disclose::Disclosures;
pub use learning_hub as learning;
pub use meaning_hub as meaning;
pub use shells::{SHELL_IDLE_LIFETIME, ShellOwner, Shells, sandbox_root};
pub use state::{
    Delivered, DisclosureSink, EVENT_POLL_INTERVAL, GatewayError, PresenceSource, SNAPSHOT_BUDGET,
    SessionContext,
};

use routes::{
    actions_handler, add_ssh_key, agent_offers_handler, agents_handler, api_not_found,
    apply_system_updates, cancel_operation, capsule_action_handler, capsule_telemetry_handler,
    connect_network, copy_host_path_handler, create_calendar_event, create_contact,
    create_file_handler, create_host_directory_handler, create_host_file_handler, create_note,
    create_snapshot, create_user, delete_draft_handler, delete_host_path_handler, delete_ssh_key,
    dialogue_memory_handler, disclosure_handler, dismiss_notifications, events_handler,
    execute_notification_action, execute_package_action, execute_service_action,
    get_artifacts_handler, get_backup_settings, get_calendar, get_candidates_handler,
    get_cognitive_graph, get_contacts, get_event_journal, get_governance_scopes_handler,
    get_mail, get_network, get_notes, get_operation, get_operation_logs, get_packages,
    get_security_settings, get_storage, get_system_logs, get_system_monitor, get_system_updates,
    get_users_settings, insight_handler, interpret_handler, launch_agent_handler,
    list_directory_handler, list_drafts_handler, list_host_directory_handler, list_notifications,
    list_operations, list_processes, list_services, login_handler, logout_handler, mind_handler,
    propose_candidate_handler, query_cognitive_graph, read_file_handler, read_host_file_handler,
    recent_actions_handler, rename_host_path_handler, restore_archive, restore_snapshot,
    revoke_artifact_handler, save_draft_handler, send_mail, send_process_signal, session_handler,
    shell_close_handler, shell_exec_handler, snapshot_handler, stop_agent_handler, trigger_backup,
    update_backup_schedule, update_note, update_security_policy, write_file_handler,
    write_host_file_handler, evaluate_candidate_handler,
};
use state::GatewayState;

/// Build the v1 router around a single source.
pub fn router(presence: Arc<dyn PresenceSource>) -> Router {
    router_with_verifier_and_access(presence, None, None, None, SessionContext::local_desktop())
}

/// Build the v1 router serving static assets when configured.
pub fn router_with_assets(presence: Arc<dyn PresenceSource>, web_root: Option<PathBuf>) -> Router {
    router_with_verifier_and_access(
        presence,
        None,
        None,
        web_root,
        SessionContext::local_desktop(),
    )
}

/// Build the v1 router with an explicit server-established trust context.
pub fn router_with_assets_and_session(
    presence: Arc<dyn PresenceSource>,
    web_root: Option<PathBuf>,
    session_context: SessionContext,
) -> Router {
    router_with_verifier_and_access(presence, None, None, web_root, session_context)
}

/// Build the v1 router with a filtered source for everyone and a full source for a signed-in
/// reader.
///
/// # Panics
///
/// If the sandbox jail the shell surface runs in cannot be created.
pub fn router_with_verifier_and_access(
    presence: Arc<dyn PresenceSource>,
    privileged: Option<Arc<dyn PresenceSource>>,
    verifier: Option<Arc<dyn CredentialVerifier>>,
    web_root: Option<PathBuf>,
    session_context: SessionContext,
) -> Router {
    router_recording_disclosures(
        presence,
        privileged,
        verifier,
        None,
        web_root,
        session_context,
    )
}

/// Build the v1 router that records what it supplies to whom.
///
/// # Panics
///
/// If the sandbox the shell surface runs in cannot be created.
pub fn router_recording_disclosures(
    presence: Arc<dyn PresenceSource>,
    privileged: Option<Arc<dyn PresenceSource>>,
    verifier: Option<Arc<dyn CredentialVerifier>>,
    journal: Option<Arc<dyn DisclosureSink>>,
    web_root: Option<PathBuf>,
    session_context: SessionContext,
) -> Router {
    router_in_sandbox(
        presence,
        privileged,
        verifier,
        journal,
        web_root,
        session_context,
        &shells::sandbox_root(),
    )
}

/// Build the v1 router around a named sandbox root.
///
/// The public builders resolve the root from the deployment. This one is told, because a test must
/// not reach into whatever sandbox the host happens to have: the deployed host has a real
/// `/home/demo` that the build user cannot write to, and a test that wrote there passed on a
/// developer machine and failed on the one that matters.
///
/// # Panics
///
/// If the sandbox the shell surface runs in cannot be created.
pub(crate) fn router_in_sandbox(
    presence: Arc<dyn PresenceSource>,
    privileged: Option<Arc<dyn PresenceSource>>,
    verifier: Option<Arc<dyn CredentialVerifier>>,
    journal: Option<Arc<dyn DisclosureSink>>,
    web_root: Option<PathBuf>,
    session_context: SessionContext,
    sandbox_path: &std::path::Path,
) -> Router {
    let now = OffsetDateTime::now_utc();
    let expires_at = (now + TimeDuration::hours(8))
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into());

    let jail = cybou_jailfs::JailFs::new(sandbox_path).expect("initialize gateway sandbox jail");
    let shells = Arc::new(shells::Shells::new(jail.clone()));
    #[cfg(test)]
    let drafts = Arc::new(crate::routes::UserDraftStore::new());
    #[cfg(not(test))]
    let drafts = {
        let db_path = crate::routes::draft_database_path(sandbox_path);
        crate::routes::validate_draft_database_isolation(&db_path, sandbox_path)
            .expect("draft database isolation invariant violated");
        Arc::new(
            crate::routes::UserDraftStore::open(&db_path)
                .expect("initialize private draft database"),
        )
    };

    let state = GatewayState {
        presence,
        privileged,
        verifier,
        sessions: Arc::new(Sessions::default()),
        disclosures: Arc::new(Disclosures::new()),
        journal,
        session: SessionProjection {
            schema_version: WEB_SCHEMA_V1,
            session_id: Uuid::new_v4(),
            mode: session_context.mode,
            consumer_id: session_context.consumer_id,
            expires_at,
        },
        shells,
        files: jail,
        host_user_files: host_user_files_source(),
        drafts,
        operations: Arc::new(crate::operations_hub::OperationsHub::new()),
        notifications: Arc::new(crate::notifications_hub::NotificationsHub::new()),
        system: Arc::new(crate::system_hub::SystemHub::new()),
        personal: Arc::new(crate::personal_hub::PersonalHub::new()),
        cognitive: Arc::new(crate::cognitive_hub::CognitiveHub::new()),
        meaning: Arc::new(crate::meaning_hub::MeaningHub::new()),
        learning: Arc::new(crate::learning_hub::LearningHub::new()),
    };

    let app = Router::new()
        .route("/api/v1/session", get(session_handler))
        .route("/api/v1/login", post(login_handler))
        .route("/api/v1/logout", post(logout_handler))
        .route("/api/v1/snapshot", get(snapshot_handler))
        .route("/api/v1/mind", get(mind_handler))
        .route("/api/v1/events", get(events_handler))
        .route("/api/v1/disclosure", get(disclosure_handler))
        .route("/api/v1/insight", get(insight_handler))
        .route("/api/v1/actions", get(actions_handler))
        .route("/api/v1/actions/recent", get(recent_actions_handler))
        .route("/api/v1/agents/offers", get(agent_offers_handler))
        .route(
            "/api/v1/agents",
            get(agents_handler).post(launch_agent_handler),
        )
        .route("/api/v1/agents/{capsule_id}", delete(stop_agent_handler))
        .route("/api/v1/agents/{capsule_id}/action", post(capsule_action_handler))
        .route("/api/v1/agents/{capsule_id}/telemetry", get(capsule_telemetry_handler))
        .route("/api/v1/shell/exec", post(shell_exec_handler))
        .route("/api/v1/shell/close", post(shell_close_handler))
        .route("/api/v1/files/list", post(list_directory_handler))
        .route("/api/v1/files/read", post(read_file_handler))
        .route("/api/v1/files/write", post(write_file_handler))
        .route("/api/v1/files/create", post(create_file_handler))
        .route("/api/v1/host-files/list", post(list_host_directory_handler))
        .route("/api/v1/host-files/read", post(read_host_file_handler))
        .route("/api/v1/host-files/write", post(write_host_file_handler))
        .route("/api/v1/host-files/create", post(create_host_file_handler))
        .route("/api/v1/host-files/mkdir", post(create_host_directory_handler))
        .route("/api/v1/host-files/rename", post(rename_host_path_handler))
        .route("/api/v1/host-files/delete", post(delete_host_path_handler))
        .route("/api/v1/host-files/copy", post(copy_host_path_handler))
        .route("/api/v1/drafts", get(list_drafts_handler))
        .route("/api/v1/drafts/save", post(save_draft_handler))
        .route("/api/v1/drafts/delete", post(delete_draft_handler))
        .route("/api/v1/operations", get(list_operations))
        .route("/api/v1/operations/{id}", get(get_operation))
        .route("/api/v1/operations/{id}/logs", get(get_operation_logs))
        .route("/api/v1/operations/cancel", post(cancel_operation))
        .route("/api/v1/notifications", get(list_notifications))
        .route("/api/v1/notifications/dismiss", post(dismiss_notifications))
        .route("/api/v1/notifications/action", post(execute_notification_action))
        .route("/api/v1/system/services", get(list_services))
        .route("/api/v1/system/services/action", post(execute_service_action))
        .route("/api/v1/system/processes", get(list_processes))
        .route("/api/v1/system/processes/signal", post(send_process_signal))
        .route("/api/v1/system/monitor", get(get_system_monitor))
        .route("/api/v1/system/logs", get(get_system_logs))
        .route("/api/v1/system/storage", get(get_storage))
        .route("/api/v1/system/storage/snapshots", post(create_snapshot))
        .route("/api/v1/system/storage/restore", post(restore_snapshot))
        .route("/api/v1/system/network", get(get_network))
        .route("/api/v1/system/network/connect", post(connect_network))
        .route("/api/v1/system/packages", get(get_packages))
        .route("/api/v1/system/packages/action", post(execute_package_action))
        .route("/api/v1/system/updates", get(get_system_updates))
        .route("/api/v1/system/updates/apply", post(apply_system_updates))
        .route("/api/v1/system/users", get(get_users_settings).post(create_user))
        .route("/api/v1/system/users/ssh-keys", post(add_ssh_key))
        .route("/api/v1/system/users/ssh-keys/delete", post(delete_ssh_key))
        .route("/api/v1/system/security", get(get_security_settings))
        .route("/api/v1/system/security/policy", post(update_security_policy))
        .route("/api/v1/system/backup", get(get_backup_settings))
        .route("/api/v1/system/backup/trigger", post(trigger_backup))
        .route("/api/v1/system/backup/restore", post(restore_archive))
        .route("/api/v1/system/backup/schedule", post(update_backup_schedule))
        .route("/api/v1/personal/mail", get(get_mail))
        .route("/api/v1/personal/mail/send", post(send_mail))
        .route("/api/v1/personal/calendar", get(get_calendar))
        .route("/api/v1/personal/calendar/events", post(create_calendar_event))
        .route("/api/v1/personal/notes", get(get_notes).post(create_note))
        .route("/api/v1/personal/notes/update", post(update_note))
        .route("/api/v1/personal/contacts", get(get_contacts).post(create_contact))
        .route("/api/v1/cognitive/graph", get(get_cognitive_graph))
        .route("/api/v1/cognitive/query", post(query_cognitive_graph))
        .route("/api/v1/cognitive/journal", get(get_event_journal))
        .route("/api/v1/meaning/interpret", post(interpret_handler))
        .route("/api/v1/meaning/dialogue", get(dialogue_memory_handler))
        .route(
            "/api/v1/learning/candidates",
            get(get_candidates_handler).post(propose_candidate_handler),
        )
        .route(
            "/api/v1/learning/candidates/{candidate_id}/evaluate",
            post(evaluate_candidate_handler),
        )
        .route("/api/v1/learning/artifacts", get(get_artifacts_handler))
        .route(
            "/api/v1/learning/artifacts/{artifact_id}/revoke",
            post(revoke_artifact_handler),
        )
        .route("/api/v1/governance/scopes", get(get_governance_scopes_handler))
        .route("/api/{*path}", any(api_not_found))
        .with_state(state)
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("content-security-policy"),
            HeaderValue::from_static(
                "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self'; connect-src 'self'; img-src 'self' data:; font-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'",
            ),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static("no-referrer"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("cache-control"),
            HeaderValue::from_static("no-store"),
        ));

    match web_root {
        Some(root) => app.fallback_service(
            ServeDir::new(&root).not_found_service(ServeFile::new(root.join("index.html"))),
        ),
        None => app,
    }
}

#[cfg(all(target_os = "linux", not(test)))]
fn host_user_files_source() -> Option<Arc<dyn state::HostUserFileSource>> {
    std::env::var_os("CYBOU_HOST_FILES_SOCKET_DIR").map(|directory| {
        Arc::new(host_files_socket::SocketHostUserFiles::in_directory(
            directory,
        )) as Arc<dyn state::HostUserFileSource>
    })
}

#[cfg(any(not(target_os = "linux"), test))]
fn host_user_files_source() -> Option<Arc<dyn state::HostUserFileSource>> {
    None
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicU64, Ordering},
        },
        time::Duration,
    };

    use async_trait::async_trait;
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
    };
    use cybou_protocol::{KnowledgeState, agent::LaunchRequest};
    use cybou_web_contracts::{
        DisclosureProjection, MindProjection, SessionMode, SessionProjection, ShellExecRequest,
        ShellExecResponse, SnapshotProjection, UserDraftListProjection,
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    use uuid::Uuid;

    use super::{
        CredentialVerifier, GatewayError, PresenceSource, SessionContext, router,
        router_with_assets, router_with_assets_and_session, router_with_verifier_and_access,
    };
    use crate::fixture::FixturePresenceSource;

    struct NamedSource {
        name: &'static str,
    }

    #[async_trait]
    impl PresenceSource for NamedSource {
        async fn snapshot(&self) -> Result<SnapshotProjection, GatewayError> {
            let mut snapshot = FixturePresenceSource::nominal()
                .snapshot()
                .await
                .expect("the fixture answers");
            snapshot.cursor = self.name.to_owned();
            Ok(snapshot)
        }
    }

    /// A source that reports a delivery with a gap in it.
    ///
    /// The fixture source supplies nothing and withholds nothing, which is the one case where the
    /// disclosure surface has no work to do. This one supplies more than it can account for and
    /// refuses two items for two different reasons, because those are the facts the surface exists
    /// to show.
    struct PartlyAccountedSource;

    #[async_trait]
    impl PresenceSource for PartlyAccountedSource {
        async fn snapshot(&self) -> Result<SnapshotProjection, GatewayError> {
            FixturePresenceSource::nominal().snapshot().await
        }

        async fn mind(&self) -> Result<MindProjection, GatewayError> {
            FixturePresenceSource::nominal().mind().await
        }

        fn last_delivery(&self) -> crate::Delivered {
            crate::Delivered {
                // Two source contributions, cited between them by the two items that could say
                // where they came from. A set of sources, not a count of items.
                items: vec![Uuid::from_u128(1), Uuid::from_u128(2)],
                provenance_count: 2,
                // Three supplied, two of which named their provenance. The third is the case the
                // two numbers exist for.
                item_count: 3,
                accounted_for: 2,
                withheld: vec![
                    cybou_protocol::disclosure::Withheld {
                        subject: Some("disk-pressure".to_owned()),
                        because: cybou_protocol::disclosure::WithheldBecause::AboveConsumerTrust,
                    },
                    cybou_protocol::disclosure::Withheld {
                        subject: None,
                        because: cybou_protocol::disclosure::WithheldBecause::BelongsToThePerson,
                    },
                ],
            }
        }
    }

    /// A guarded router whose shell sandbox is a directory this test owns.
    ///
    /// The `TempDir` is returned rather than dropped: dropping it removes the sandbox out from
    /// under the router. Tests bind it to `_sandbox` so it lives to the end of the test.
    fn shell_router_over_a_temporary_sandbox() -> (Router, tempfile::TempDir) {
        let sandbox = tempfile::tempdir().expect("a sandbox root");
        std::fs::create_dir(sandbox.path().join("somewhere")).expect("a directory to enter");
        let app = crate::router_in_sandbox(
            Arc::new(FixturePresenceSource::nominal()),
            None,
            Some(Arc::new(OneAccount)),
            None,
            None,
            SessionContext::public_preview(),
            sandbox.path(),
        );
        (app, sandbox)
    }

    /// Read the disclosure surface as whoever holds this cookie, if any.
    async fn disclosure_for(app: &Router, cookie: Option<&str>) -> DisclosureProjection {
        let mut request = Request::builder().uri("/api/v1/disclosure");
        if let Some(cookie) = cookie {
            request = request.header("cookie", cookie);
        }
        let response = app
            .clone()
            .oneshot(request.body(Body::empty()).expect("a request"))
            .await
            .expect("a response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("a body");
        serde_json::from_slice(&body).expect("a disclosure projection")
    }

    /// Fetch the Mind projection, which is what records a delivery.
    async fn read_mind(app: &Router, cookie: Option<&str>) {
        let mut request = Request::builder().uri("/api/v1/mind");
        if let Some(cookie) = cookie {
            request = request.header("cookie", cookie);
        }
        let response = app
            .clone()
            .oneshot(request.body(Body::empty()).expect("a request"))
            .await
            .expect("a response");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn nothing_supplied_yet_is_not_the_same_as_an_empty_delivery() {
        // A surface that answered "you were supplied nothing" before anything had been supplied
        // would be making a claim about a delivery that never happened.
        let app = router(Arc::new(PartlyAccountedSource));
        let disclosure = disclosure_for(&app, None).await;
        assert!(!disclosure.delivered);
        assert_eq!(disclosure.supplied, 0);
        assert!(disclosure.withheld.is_empty());
    }

    #[tokio::test]
    async fn a_reader_is_shown_that_something_was_kept_from_them_and_why() {
        // This router has no Journal sink, which is deliberate: what was supplied must be
        // remembered whether or not there is anywhere durable to write it down.
        //
        // It also carries no session, so the reader is the public consumer even though the
        // deployment is a local desktop — which is correct, because a reader without a session is
        // served the filtered source. They are told what was refused and why; the subjects are the
        // business of the separate test below.
        let app = router(Arc::new(PartlyAccountedSource));
        read_mind(&app, None).await;

        let disclosure = disclosure_for(&app, None).await;
        assert!(disclosure.delivered);
        assert_eq!(disclosure.withheld.len(), 2);
        assert!(
            disclosure
                .withheld
                .iter()
                .any(|item| item.because == "aboveConsumerTrust")
        );
        assert!(
            disclosure
                .withheld
                .iter()
                .any(|item| item.because == "belongsToThePerson")
        );
    }

    /// A source whose items cite far more contributions than there are items.
    ///
    /// The realistic shape: one belief can name hundreds of contributions. This is what made the
    /// provenance set unusable as a count, and what made the response a hundred kilobytes.
    struct WidelyDerivedSource;

    #[async_trait]
    impl PresenceSource for WidelyDerivedSource {
        async fn snapshot(&self) -> Result<SnapshotProjection, GatewayError> {
            FixturePresenceSource::nominal().snapshot().await
        }

        async fn mind(&self) -> Result<MindProjection, GatewayError> {
            FixturePresenceSource::nominal().mind().await
        }

        fn last_delivery(&self) -> crate::Delivered {
            crate::Delivered {
                // A delivery whose record could carry every source it cited, and one whose sources
                // outran the bound: both are exercised, because the surface has to read the count
                // rather than the length either way.
                items: (0..cybou_protocol::disclosure::MAX_RECORDED_PROVENANCE as u128)
                    .map(Uuid::from_u128)
                    .collect(),
                provenance_count: 500,
                item_count: 10,
                accounted_for: 10,
                withheld: Vec::new(),
            }
        }
    }

    #[tokio::test]
    async fn a_wide_provenance_set_is_sampled_and_its_true_size_reported() {
        let app = router(Arc::new(WidelyDerivedSource));
        read_mind(&app, None).await;

        let disclosure = disclosure_for(&app, None).await;
        assert_eq!(disclosure.supplied, 10);
        assert_eq!(disclosure.accounted_for, 10);
        // The total is never truncated, only the sample is.
        assert_eq!(disclosure.provenance_count, 500);
        assert_eq!(
            disclosure.items.len(),
            cybou_web_contracts::DISCLOSURE_ITEM_SAMPLE
        );
    }

    #[tokio::test]
    async fn a_stranger_is_told_how_much_was_refused_and_never_what_it_was_about() {
        // The subject of a refused concept is that concept's label. Publishing it to explain the
        // refusal would perform the disclosure the refusal prevented, so the surface that reports
        // a filter must not be a way around it.
        let app = router_with_assets_and_session(
            Arc::new(PartlyAccountedSource),
            None,
            SessionContext::public_preview(),
        );
        read_mind(&app, None).await;

        let disclosure = disclosure_for(&app, None).await;
        assert!(!disclosure.subjects_visible);
        assert_eq!(
            disclosure.withheld.len(),
            2,
            "the count of refusals is a fact about the system and is still given"
        );
        assert!(
            disclosure
                .withheld
                .iter()
                .all(|item| item.subject.is_none()),
            "a public reader was named a withheld subject"
        );
        // The grounds are still stated: how much was refused and why are not the person's secrets.
        assert!(
            disclosure
                .withheld
                .iter()
                .any(|item| item.because == "aboveConsumerTrust")
        );
    }

    #[tokio::test]
    async fn the_person_the_record_is_about_is_named_the_subjects() {
        let app = router_with_verifier_and_access(
            Arc::new(PartlyAccountedSource),
            Some(Arc::new(PartlyAccountedSource)),
            Some(Arc::new(OneAccount)),
            None,
            SessionContext::public_preview(),
        );

        let cookie = sign_in(&app, "alice", "hunter2").await.expect("a session");
        read_mind(&app, Some(&cookie)).await;

        let disclosure = disclosure_for(&app, Some(&cookie)).await;
        assert!(disclosure.subjects_visible);
        assert!(
            disclosure
                .withheld
                .iter()
                .any(|item| item.subject.as_deref() == Some("disk-pressure")),
            "the owner could not see what was held back from them"
        );
    }

    #[tokio::test]
    async fn a_delivery_says_what_it_cannot_account_for() {
        // The whole reason two numbers are carried: three items crossed the boundary and only two
        // of them can name where they came from. A surface reporting "two" would be claiming
        // provenance it does not have.
        let app = router(Arc::new(PartlyAccountedSource));
        read_mind(&app, None).await;

        let disclosure = disclosure_for(&app, None).await;
        assert_eq!(disclosure.supplied, 3);
        assert_eq!(disclosure.accounted_for, 2);
        // Never more accounted for than supplied. The first live deployment of this surface
        // reported ten supplied and three thousand accounted for, because the length of the
        // provenance set was being read as a count of items. It is its own number now.
        assert!(disclosure.accounted_for <= disclosure.supplied);
        assert_eq!(disclosure.provenance_count, 2);
        assert_eq!(disclosure.items.len(), 2);
        assert!(disclosure.external_boundary);
        assert!(!disclosure.retains);
    }

    #[tokio::test]
    async fn a_reader_is_shown_their_own_deliveries_and_not_someone_elses() {
        // The record is keyed by consumer. A surface that answered for whoever read last would be
        // a log of what was done to everyone, which is a different and much worse thing.
        let app = router_with_verifier_and_access(
            Arc::new(PartlyAccountedSource),
            None,
            Some(Arc::new(OneAccount)),
            None,
            SessionContext::public_preview(),
        );

        // A stranger reads, and is recorded as the public consumer.
        read_mind(&app, None).await;
        let stranger = disclosure_for(&app, None).await;
        assert!(stranger.delivered);
        assert_eq!(stranger.consumer_id, "living-canvas:public");

        // Someone who signs in is a different consumer, and has been supplied nothing yet.
        let cookie = sign_in(&app, "alice", "hunter2").await.expect("a session");
        let owner = disclosure_for(&app, Some(&cookie)).await;
        assert_eq!(owner.consumer_id, "living-canvas:alice");
        assert!(!owner.delivered);
    }

    struct OneAccount;

    #[async_trait]
    impl CredentialVerifier for OneAccount {
        async fn verify(&self, username: &str, password: &str) -> Option<crate::VerifiedAccount> {
            (username == "alice" && password == "hunter2").then(|| crate::VerifiedAccount {
                username: username.to_owned(),
                uid: 1000,
                home: "/home/alice".to_owned(),
            })
        }
    }

    async fn cursor_for(app: &Router, cookie: Option<&str>) -> String {
        let mut request = Request::builder().uri("/api/v1/snapshot");
        if let Some(cookie) = cookie {
            request = request.header("cookie", cookie);
        }
        let response = app
            .clone()
            .oneshot(request.body(Body::empty()).expect("a request"))
            .await
            .expect("a response");
        let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("a body");
        let projection: SnapshotProjection = serde_json::from_slice(&body).expect("a projection");
        projection.cursor
    }

    async fn sign_in(app: &Router, username: &str, password: &str) -> Option<String> {
        let body = format!(r#"{{"username":"{username}","password":"{password}"}}"#);
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/login")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .expect("a request"),
            )
            .await
            .expect("a response");
        if response.status() != StatusCode::OK {
            return None;
        }
        let cookie = response
            .headers()
            .get("set-cookie")?
            .to_str()
            .ok()?
            .split(';')
            .next()?
            .to_owned();
        Some(cookie)
    }

    fn guarded_router() -> Router {
        router_with_verifier_and_access(
            Arc::new(NamedSource { name: "public" }),
            Some(Arc::new(NamedSource { name: "privileged" })),
            Some(Arc::new(OneAccount)),
            None,
            SessionContext::public_preview(),
        )
    }

    #[tokio::test]
    async fn a_reader_who_has_not_signed_in_is_served_the_public_source() {
        let app = guarded_router();
        assert_eq!(cursor_for(&app, None).await, "public");
    }

    #[tokio::test]
    async fn a_session_nobody_issued_is_served_the_public_source_rather_than_refused() {
        let app = guarded_router();
        assert_eq!(
            cursor_for(&app, Some("cybou_session=invented")).await,
            "public"
        );
        assert_eq!(cursor_for(&app, Some("cybou_session=")).await, "public");
        assert_eq!(cursor_for(&app, Some("theme=dark")).await, "public");
    }

    #[tokio::test]
    async fn signing_in_is_what_reaches_the_unfiltered_source() {
        let app = guarded_router();
        let cookie = sign_in(&app, "alice", "hunter2")
            .await
            .expect("the account and secret are the ones this verifier accepts");
        assert_eq!(cursor_for(&app, Some(&cookie)).await, "privileged");
    }

    #[tokio::test]
    async fn a_wrong_secret_establishes_nothing() {
        let app = guarded_router();
        assert!(sign_in(&app, "alice", "not-hunter2").await.is_none());
        assert!(sign_in(&app, "mallory", "hunter2").await.is_none());
        assert!(sign_in(&app, "alice", "").await.is_none());
    }

    #[tokio::test]
    async fn signing_out_ends_the_session_it_was_given() {
        let app = guarded_router();
        let cookie = sign_in(&app, "alice", "hunter2").await.expect("signed in");
        assert_eq!(cursor_for(&app, Some(&cookie)).await, "privileged");

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/logout")
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .expect("a request"),
            )
            .await
            .expect("a response");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(cursor_for(&app, Some(&cookie)).await, "public");
    }

    #[tokio::test]
    async fn drafts_follow_the_authenticated_account_across_sessions() {
        let app = guarded_router();
        let first = sign_in(&app, "alice", "hunter2")
            .await
            .expect("first session");
        let saved = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/drafts/save")
                    .header("cookie", &first)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"draftId":"recovery","title":"Recovery","content":"survives login","baseLocation":null,"baseSha256":null}"#,
                    ))
                    .expect("save request"),
            )
            .await
            .expect("save response");
        assert_eq!(saved.status(), StatusCode::OK);

        let logged_out = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/logout")
                    .header("cookie", &first)
                    .body(Body::empty())
                    .expect("logout request"),
            )
            .await
            .expect("logout response");
        assert_eq!(logged_out.status(), StatusCode::OK);

        let second = sign_in(&app, "alice", "hunter2")
            .await
            .expect("second session");
        let listed = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/drafts")
                    .header("cookie", second)
                    .body(Body::empty())
                    .expect("list request"),
            )
            .await
            .expect("list response");
        assert_eq!(listed.status(), StatusCode::OK);
        let body = axum::body::to_bytes(listed.into_body(), 2 * 1024 * 1024)
            .await
            .expect("list body");
        let projection: UserDraftListProjection =
            serde_json::from_slice(&body).expect("draft list projection");
        assert_eq!(projection.drafts.len(), 1);
        assert_eq!(projection.drafts[0].draft_id, "recovery");
    }

    #[tokio::test]
    async fn a_deployment_that_cannot_authenticate_anybody_serves_everyone_the_public_source() {
        let app = router_with_verifier_and_access(
            Arc::new(NamedSource { name: "public" }),
            None,
            None,
            None,
            SessionContext::public_preview(),
        );
        assert_eq!(cursor_for(&app, None).await, "public");
        assert!(sign_in(&app, "alice", "hunter2").await.is_none());
    }

    #[tokio::test]
    async fn a_signed_in_session_says_which_account_it_belongs_to() {
        let app = guarded_router();
        let cookie = sign_in(&app, "alice", "hunter2").await.expect("signed in");
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/session")
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .expect("a request"),
            )
            .await
            .expect("a response");
        let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("a body");
        let projection: SessionProjection = serde_json::from_slice(&body).expect("a projection");
        assert_eq!(projection.mode, SessionMode::RemoteBrowser);
        assert_eq!(projection.consumer_id, "alice");
    }

    #[tokio::test]
    async fn session_is_server_established_local_and_not_cached() {
        let response = router(Arc::new(FixturePresenceSource::nominal()))
            .oneshot(
                Request::builder()
                    .uri("/api/v1/session")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("router response");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert_eq!(response.headers()["x-content-type-options"], "nosniff");
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("session body")
            .to_bytes();
        let session: SessionProjection = serde_json::from_slice(&bytes).expect("typed session");
        assert_eq!(session.mode, SessionMode::LocalDesktop);
    }

    #[tokio::test]
    async fn public_preview_never_claims_local_desktop_trust() {
        let response = router_with_assets_and_session(
            Arc::new(FixturePresenceSource::nominal()),
            None,
            SessionContext::public_preview(),
        )
        .oneshot(
            Request::builder()
                .uri("/api/v1/session")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("router response");
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("session body")
            .to_bytes();
        let session: SessionProjection = serde_json::from_slice(&bytes).expect("typed session");
        assert_eq!(session.mode, SessionMode::PublicPreview);
        assert_eq!(session.consumer_id, "public-preview");
    }

    #[tokio::test]
    async fn event_stream_emits_snapshot_with_resumable_cursor() {
        let response = router(Arc::new(FixturePresenceSource::nominal()))
            .oneshot(
                Request::builder()
                    .uri("/api/v1/events")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("router response");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["content-type"], "text/event-stream");
        let mut body = response.into_body();
        let frame = tokio::time::timeout(Duration::from_millis(100), body.frame())
            .await
            .expect("initial event budget")
            .expect("initial event frame")
            .expect("valid event frame");
        let data = frame.into_data().expect("event data");
        let event = String::from_utf8_lossy(&data);
        assert!(event.contains("event: snapshot"));
        assert!(event.contains("id: fixture:presence:42"));
        assert!(event.contains("\"projectionVersion\":42"));
    }

    #[tokio::test]
    async fn event_stream_rejects_unbounded_resume_cursor() {
        let response = router(Arc::new(FixturePresenceSource::nominal()))
            .oneshot(
                Request::builder()
                    .uri("/api/v1/events")
                    .header("last-event-id", "x".repeat(257))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("router response");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    struct SignaledPresence {
        version: AtomicU64,
    }

    #[async_trait]
    impl PresenceSource for SignaledPresence {
        async fn snapshot(&self) -> Result<SnapshotProjection, GatewayError> {
            let version = self.version.load(Ordering::Relaxed);
            let mut snapshot = FixturePresenceSource::nominal().snapshot().await?;
            snapshot.projection_version = version;
            snapshot.cursor = format!("signal:{version}");
            Ok(snapshot)
        }

        async fn wait_for_change(&self) -> Result<(), GatewayError> {
            self.version.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    #[tokio::test]
    async fn event_stream_waits_for_source_signal_before_new_snapshot() {
        let response = router(Arc::new(SignaledPresence {
            version: AtomicU64::new(42),
        }))
        .oneshot(
            Request::builder()
                .uri("/api/v1/events")
                .header("last-event-id", "signal:42")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("router response");
        let mut body = response.into_body();
        let frame = tokio::time::timeout(Duration::from_millis(100), body.frame())
            .await
            .expect("native change notification budget")
            .expect("changed event frame")
            .expect("valid changed event frame");
        let data = frame.into_data().expect("event data");
        let event = String::from_utf8_lossy(&data);
        assert!(event.contains("id: signal:43"));
        assert!(event.contains("\"projectionVersion\":43"));
    }

    #[tokio::test]
    async fn snapshot_round_trips_as_typed_projection() {
        let response = router(Arc::new(FixturePresenceSource::nominal()))
            .oneshot(
                Request::builder()
                    .uri("/api/v1/snapshot")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("router response");
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("snapshot body")
            .to_bytes();
        let snapshot: SnapshotProjection = serde_json::from_slice(&bytes).expect("typed snapshot");
        assert_eq!(snapshot.projection_version, 42);
    }

    #[tokio::test]
    async fn mind_route_returns_what_the_owners_hold() {
        let response = router(Arc::new(FixturePresenceSource::nominal()))
            .oneshot(
                Request::builder()
                    .uri("/api/v1/mind")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("router response");
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("mind body")
            .to_bytes();
        let mind: MindProjection = serde_json::from_slice(&bytes).expect("typed mind projection");
        assert_eq!(mind.identity.knowledge, KnowledgeState::Known);
        assert_eq!(mind.journal.contribution_count, Some(134));
        assert_eq!(mind.commitments.open.len(), 1);
    }

    #[tokio::test]
    async fn a_source_that_cannot_reach_the_owners_says_so_rather_than_answering_empty() {
        struct SnapshotOnlySource;

        #[async_trait]
        impl PresenceSource for SnapshotOnlySource {
            async fn snapshot(&self) -> Result<SnapshotProjection, GatewayError> {
                FixturePresenceSource::nominal().snapshot().await
            }
        }

        let response = router(Arc::new(SnapshotOnlySource))
            .oneshot(
                Request::builder()
                    .uri("/api/v1/mind")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("router response");
        assert_ne!(response.status(), StatusCode::OK);
    }

    struct SlowPresence;

    #[async_trait]
    impl PresenceSource for SlowPresence {
        async fn snapshot(&self) -> Result<SnapshotProjection, GatewayError> {
            tokio::time::sleep(Duration::from_secs(5)).await;
            Err(GatewayError::Unavailable)
        }
    }

    #[tokio::test]
    async fn outer_budget_returns_typed_timeout_instead_of_empty_success() {
        tokio::time::pause();
        let task = tokio::spawn(async {
            router(Arc::new(SlowPresence))
                .oneshot(
                    Request::builder()
                        .uri("/api/v1/snapshot")
                        .body(Body::empty())
                        .expect("request"),
                )
                .await
                .expect("router response")
        });
        tokio::time::advance(Duration::from_secs(2)).await;
        let response = task.await.expect("timeout task");
        assert_eq!(response.status(), StatusCode::GATEWAY_TIMEOUT);
    }

    #[tokio::test]
    async fn mutation_and_generic_rpc_routes_do_not_exist() {
        for uri in ["/api/v1/rpc", "/api/v1/commands/promise"] {
            let response = router(Arc::new(FixturePresenceSource::nominal()))
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(uri)
                        .body(Body::empty())
                        .expect("request"),
                )
                .await
                .expect("router response");
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
        }
    }

    #[tokio::test]
    async fn living_canvas_and_api_share_an_origin_without_spa_fallback_for_unknown_api() {
        let root = tempfile::tempdir().expect("temporary web root");
        std::fs::write(
            root.path().join("index.html"),
            "<!doctype html><title>Living Canvas</title>",
        )
        .expect("test index");
        let app = router_with_assets(
            Arc::new(FixturePresenceSource::nominal()),
            Some(root.path().to_path_buf()),
        );

        let page = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("page response");
        assert_eq!(page.status(), StatusCode::OK);
        let page_bytes = page
            .into_body()
            .collect()
            .await
            .expect("page body")
            .to_bytes();
        assert!(String::from_utf8_lossy(&page_bytes).contains("Living Canvas"));

        let unknown_api = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/not-a-route")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("unknown API response");
        assert_eq!(unknown_api.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn shell_exec_runs_in_local_desktop_mode() {
        let app = router(Arc::new(FixturePresenceSource::nominal()));
        let payload = serde_json::to_vec(&ShellExecRequest {
            command: "pwd".into(),
            instance: 0,
        })
        .expect("serialize request");

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/shell/exec")
                    .header("content-type", "application/json")
                    .body(Body::from(payload))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes();
        let parsed: ShellExecResponse =
            serde_json::from_slice(&body_bytes).expect("deserialize shell response");
        assert_eq!(parsed.exit_code, 0);
        assert_eq!(parsed.stdout.trim(), "/");
    }

    /// Run one command as the holder of this cookie, and return what the shell said.
    async fn shell_exec_as(app: &Router, cookie: &str, command: &str) -> ShellExecResponse {
        shell_exec_instance(app, cookie, command, 0).await
    }

    /// Run one command in a named shell of the seat this cookie holds.
    async fn shell_exec_instance(
        app: &Router,
        cookie: &str,
        command: &str,
        instance: u32,
    ) -> ShellExecResponse {
        let payload = serde_json::to_vec(&ShellExecRequest {
            command: command.into(),
            instance,
        })
        .expect("serialize request");
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/shell/exec")
                    .header("content-type", "application/json")
                    .header("cookie", cookie)
                    .body(Body::from(payload))
                    .expect("a request"),
            )
            .await
            .expect("a response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = response
            .into_body()
            .collect()
            .await
            .expect("a body")
            .to_bytes();
        serde_json::from_slice(&body).expect("a shell response")
    }

    #[tokio::test]
    async fn one_session_does_not_move_another_sessions_shell() {
        // Two sign-ins are two seats even when they are the same account, because a shell is where
        // one person is standing. Before shells were split per session, the `cd` below moved the
        // second caller too, and its `pwd` answered for somebody else.
        let (app, _sandbox) = shell_router_over_a_temporary_sandbox();

        let first = sign_in(&app, "alice", "hunter2").await.expect("a session");
        let second = sign_in(&app, "alice", "hunter2").await.expect("a session");
        assert_ne!(first, second, "two sign-ins must be two sessions");

        let moved = shell_exec_as(&app, &first, "cd somewhere").await;
        assert_eq!(moved.exit_code, 0);
        assert_eq!(moved.cwd, "/somewhere");

        let untouched = shell_exec_as(&app, &second, "pwd").await;
        assert_eq!(untouched.stdout.trim(), "/");
        assert_eq!(untouched.cwd, "/");

        // And the session that moved is still where it put itself.
        let still_there = shell_exec_as(&app, &first, "pwd").await;
        assert_eq!(still_there.stdout.trim(), "/somewhere");
    }

    /// Ask a typed filesystem route as whoever holds this cookie.
    async fn files_post(
        app: &Router,
        route: &str,
        cookie: &str,
        path: &str,
    ) -> axum::http::Response<Body> {
        let payload = serde_json::to_vec(&cybou_web_contracts::FilePathRequest {
            path: path.to_owned(),
        })
        .expect("serialize request");
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(route)
                    .header("content-type", "application/json")
                    .header("cookie", cookie)
                    .body(Body::from(payload))
                    .expect("a request"),
            )
            .await
            .expect("a response")
    }

    /// A router that serves nothing until somebody signs in.
    fn closed_router() -> Router {
        router_with_verifier_and_access(
            Arc::new(FixturePresenceSource::nominal()),
            Some(Arc::new(FixturePresenceSource::nominal())),
            Some(Arc::new(OneAccount)),
            None,
            SessionContext::sign_in_required_context(),
        )
    }

    /// Ask one route, with a cookie or without one.
    async fn get_route(app: &Router, uri: &str, cookie: Option<&str>) -> StatusCode {
        let mut request = Request::builder().uri(uri);
        if let Some(cookie) = cookie {
            request = request.header("cookie", cookie);
        }
        app.clone()
            .oneshot(request.body(Body::empty()).expect("a request"))
            .await
            .expect("a response")
            .status()
    }

    #[tokio::test]
    async fn a_stranger_is_served_no_projection_at_all() {
        // Not a filtered one. Filtering is not the same as not showing, and the person the
        // projection is about had agreed to neither. This is asked at the gateway, so hiding the
        // cards in the page would not have been enough: every one of these is reachable with curl.
        let app = closed_router();
        for route in [
            "/api/v1/snapshot",
            "/api/v1/mind",
            "/api/v1/events",
            "/api/v1/disclosure",
        ] {
            assert_eq!(
                get_route(&app, route, None).await,
                StatusCode::UNAUTHORIZED,
                "{route} answered a stranger"
            );
        }
    }

    #[tokio::test]
    async fn the_way_in_is_still_open() {
        // A gate that refused the routes needed to pass it would be a locked door with no handle.
        let app = closed_router();
        assert_eq!(
            get_route(&app, "/api/v1/session", None).await,
            StatusCode::OK
        );
        assert!(sign_in(&app, "alice", "hunter2").await.is_some());
    }

    #[tokio::test]
    async fn signing_in_is_what_opens_it() {
        let app = closed_router();
        let cookie = sign_in(&app, "alice", "hunter2").await.expect("a session");
        for route in ["/api/v1/snapshot", "/api/v1/mind", "/api/v1/disclosure"] {
            assert_eq!(
                get_route(&app, route, Some(&cookie)).await,
                StatusCode::OK,
                "{route} refused someone who had signed in"
            );
        }
    }

    #[tokio::test]
    async fn a_session_nobody_issued_does_not_open_it() {
        let app = closed_router();
        assert_eq!(
            get_route(&app, "/api/v1/mind", Some("cybou_session=invented")).await,
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn a_surface_deliberately_opened_still_serves() {
        // PublicPreview did not stop meaning what it means; it stopped being what happens when
        // nobody chooses anything.
        let app = router_with_assets_and_session(
            Arc::new(FixturePresenceSource::nominal()),
            None,
            SessionContext::public_preview(),
        );
        assert_eq!(
            get_route(&app, "/api/v1/snapshot", None).await,
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn a_directory_is_read_as_structure_rather_than_as_terminal_output() {
        // The File Manager used to parse `ls -la`. Its parser wanted nine whitespace fields, the
        // engine produced six, and every entry fell through both branches — so the panel showed an
        // empty directory and reported no error, because from the parser's view there was nothing
        // there. This route hands back what the sandbox established.
        let (app, sandbox) = shell_router_over_a_temporary_sandbox();
        std::fs::write(sandbox.path().join("welcome.txt"), "hello").expect("a file");
        let cookie = sign_in(&app, "alice", "hunter2").await.expect("a session");

        let response = files_post(&app, "/api/v1/files/list", &cookie, "/").await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("a body");
        let listing: cybou_web_contracts::DirectoryListingProjection =
            serde_json::from_slice(&body).expect("a listing");

        assert!(!listing.truncated);
        assert_eq!(listing.total_entries, 2);
        assert!(
            listing
                .entries
                .iter()
                .all(|entry| entry.name != ".cybou-private"),
            "private operational state must never appear inside the user-visible jail"
        );
        // Directories first, then by name — the order the sandbox sorted them in.
        assert!(listing.entries[0].is_dir);
        assert_eq!(listing.entries[0].name, "somewhere");
        let file = listing
            .entries
            .iter()
            .find(|entry| entry.name == "welcome.txt")
            .expect("the file");
        assert!(!file.is_dir);
        assert_eq!(file.size_bytes, 5);
    }

    #[tokio::test]
    async fn a_file_is_read_with_the_size_it_actually_has() {
        let (app, sandbox) = shell_router_over_a_temporary_sandbox();
        std::fs::write(sandbox.path().join("welcome.txt"), "hello").expect("a file");
        let cookie = sign_in(&app, "alice", "hunter2").await.expect("a session");

        let response = files_post(&app, "/api/v1/files/read", &cookie, "/welcome.txt").await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("a body");
        let content: cybou_web_contracts::FileContentProjection =
            serde_json::from_slice(&body).expect("file content");
        assert_eq!(content.text, "hello");
        assert_eq!(content.size_bytes, 5);
        assert_eq!(
            content.content_sha256,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
        assert!(matches!(
            content.location,
            cybou_protocol::LocationRef::SafeShellJail { ref session_id, ref path }
                if session_id.starts_with("session-") && path == "/welcome.txt"
        ));
    }

    #[tokio::test]
    async fn a_file_write_requires_the_version_read_and_returns_verified_state() {
        let (app, sandbox) = shell_router_over_a_temporary_sandbox();
        std::fs::write(sandbox.path().join("welcome.txt"), "hello").expect("a file");
        let cookie = sign_in(&app, "alice", "hunter2").await.expect("a session");

        let read = files_post(&app, "/api/v1/files/read", &cookie, "/welcome.txt").await;
        let body = axum::body::to_bytes(read.into_body(), 64 * 1024)
            .await
            .expect("read body");
        let content: cybou_web_contracts::FileContentProjection =
            serde_json::from_slice(&body).expect("file content");
        let request = cybou_web_contracts::FileWriteRequest {
            location: content.location,
            expected_sha256: content.content_sha256,
            text: "updated".to_string(),
        };
        let payload = serde_json::to_vec(&request).expect("write request");
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/files/write")
                    .header("content-type", "application/json")
                    .header("cookie", &cookie)
                    .body(Body::from(payload))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("write body");
        let saved: cybou_web_contracts::FileWriteProjection =
            serde_json::from_slice(&body).expect("write result");
        assert_eq!(saved.size_bytes, 7);
        assert_eq!(
            std::fs::read_to_string(sandbox.path().join("welcome.txt")).expect("saved file"),
            "updated"
        );

        let stale_payload = serde_json::to_vec(&request).expect("stale write request");
        let stale = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/files/write")
                    .header("content-type", "application/json")
                    .header("cookie", &cookie)
                    .body(Body::from(stale_payload))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(stale.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn a_system_spelled_path_inside_the_jail_remains_a_jail_location() {
        let (app, sandbox) = shell_router_over_a_temporary_sandbox();
        std::fs::create_dir_all(sandbox.path().join("etc")).expect("jail etc directory");
        std::fs::write(sandbox.path().join("etc/example.conf"), "demo").expect("jail file");
        let cookie = sign_in(&app, "alice", "hunter2").await.expect("a session");

        let response = files_post(&app, "/api/v1/files/read", &cookie, "/etc/example.conf").await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("a body");
        let content: cybou_web_contracts::FileContentProjection =
            serde_json::from_slice(&body).expect("file content");

        assert!(matches!(
            content.location,
            cybou_protocol::LocationRef::SafeShellJail { ref path, .. }
                if path == "/etc/example.conf"
        ));
    }

    #[tokio::test]
    async fn leaving_the_sandbox_is_answered_exactly_as_not_existing() {
        // Distinguishing the two would let a caller entitled to read inside the sandbox map its
        // edge by watching which refusals differ.
        let (app, _sandbox) = shell_router_over_a_temporary_sandbox();
        let cookie = sign_in(&app, "alice", "hunter2").await.expect("a session");

        let escaped = files_post(&app, "/api/v1/files/read", &cookie, "/../../etc/passwd").await;
        let absent = files_post(&app, "/api/v1/files/read", &cookie, "/no-such-file").await;
        assert_eq!(escaped.status(), StatusCode::NOT_FOUND);
        assert_eq!(absent.status(), escaped.status());

        let escaped_body = axum::body::to_bytes(escaped.into_body(), 16 * 1024)
            .await
            .expect("a body");
        let absent_body = axum::body::to_bytes(absent.into_body(), 16 * 1024)
            .await
            .expect("a body");
        assert_eq!(
            escaped_body, absent_body,
            "the two refusals are distinguishable"
        );
    }

    #[tokio::test]
    async fn a_public_reader_cannot_read_the_sandbox_at_all() {
        let app = router_with_assets_and_session(
            Arc::new(FixturePresenceSource::nominal()),
            None,
            SessionContext::public_preview(),
        );
        let payload = serde_json::to_vec(&cybou_web_contracts::FilePathRequest {
            path: "/".to_owned(),
        })
        .expect("serialize request");
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/files/list")
                    .header("content-type", "application/json")
                    .body(Body::from(payload))
                    .expect("a request"),
            )
            .await
            .expect("a response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn two_shells_in_one_session_are_two_places_to_stand() {
        // `CardSpec` says a Shell card is not a singleton. Until the instance reached the gateway
        // that was a promise the backend could not keep: every card in a session drove one engine,
        // so opening a second Shell gave you a second view of the first one's working directory.
        let (app, _sandbox) = shell_router_over_a_temporary_sandbox();
        let cookie = sign_in(&app, "alice", "hunter2").await.expect("a session");

        let moved = shell_exec_instance(&app, &cookie, "cd somewhere", 0).await;
        assert_eq!(moved.cwd, "/somewhere");

        let other = shell_exec_instance(&app, &cookie, "pwd", 1).await;
        assert_eq!(
            other.cwd, "/",
            "a second Shell card inherited the first one's cwd"
        );
        assert_eq!(other.stdout.trim(), "/");

        // And the first is still where it put itself.
        assert_eq!(
            shell_exec_instance(&app, &cookie, "pwd", 0).await.cwd,
            "/somewhere"
        );
    }

    #[tokio::test]
    async fn signing_out_ends_every_shell_the_session_opened() {
        // Not just the one numbered zero: a person who opened three shells and signed out left
        // three working directories behind, and only one of them was being forgotten.
        let (app, _sandbox) = shell_router_over_a_temporary_sandbox();
        let cookie = sign_in(&app, "alice", "hunter2").await.expect("a session");

        for instance in 0..3 {
            assert_eq!(
                shell_exec_instance(&app, &cookie, "cd somewhere", instance)
                    .await
                    .cwd,
                "/somewhere"
            );
        }

        let signed_out = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/logout")
                    .header("cookie", cookie.clone())
                    .body(Body::empty())
                    .expect("a request"),
            )
            .await
            .expect("a response");
        assert_eq!(signed_out.status(), StatusCode::OK);

        for instance in 0..3 {
            let payload = serde_json::to_vec(&ShellExecRequest {
                command: "pwd".into(),
                instance,
            })
            .expect("serialize request");
            let refused = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/v1/shell/exec")
                        .header("content-type", "application/json")
                        .header("cookie", cookie.clone())
                        .body(Body::from(payload))
                        .expect("a request"),
                )
                .await
                .expect("a response");
            assert_eq!(refused.status(), StatusCode::FORBIDDEN);
        }
    }

    #[tokio::test]
    async fn signing_out_forgets_where_the_session_was_standing() {
        let (app, _sandbox) = shell_router_over_a_temporary_sandbox();

        let cookie = sign_in(&app, "alice", "hunter2").await.expect("a session");
        assert_eq!(
            shell_exec_as(&app, &cookie, "cd somewhere").await.cwd,
            "/somewhere"
        );

        let signed_out = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/logout")
                    .header("cookie", cookie.clone())
                    .body(Body::empty())
                    .expect("a request"),
            )
            .await
            .expect("a response");
        assert_eq!(signed_out.status(), StatusCode::OK);

        // The cookie no longer names a session, so the shell surface refuses rather than handing
        // the caller whatever shell that token used to own.
        let payload = serde_json::to_vec(&ShellExecRequest {
            command: "pwd".into(),
            instance: 0,
        })
        .expect("serialize request");
        let refused = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/shell/exec")
                    .header("content-type", "application/json")
                    .header("cookie", cookie)
                    .body(Body::from(payload))
                    .expect("a request"),
            )
            .await
            .expect("a response");
        assert_eq!(refused.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn shell_exec_is_strictly_forbidden_in_public_preview() {
        let app = router_with_assets_and_session(
            Arc::new(FixturePresenceSource::nominal()),
            None,
            SessionContext::public_preview(),
        );
        let payload = serde_json::to_vec(&ShellExecRequest {
            command: "pwd".into(),
            instance: 0,
        })
        .expect("serialize request");

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/shell/exec")
                    .header("content-type", "application/json")
                    .body(Body::from(payload))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn agent_launch_is_strictly_forbidden_in_public_preview() {
        let app = router_with_assets_and_session(
            Arc::new(FixturePresenceSource::nominal()),
            None,
            SessionContext::public_preview(),
        );
        let payload = serde_json::to_vec(&LaunchRequest {
            profile: "bounded".into(),
            agent: "opencode".into(),
            workspace: "/srv/project".into(),
            model_class: Some("Strong".into()),
            prompt: "Inspect the repository".into(),
        })
        .expect("serialize request");

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/agents")
                    .header("content-type", "application/json")
                    .body(Body::from(payload))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn agent_stop_is_strictly_forbidden_in_public_preview() {
        let app = router_with_assets_and_session(
            Arc::new(FixturePresenceSource::nominal()),
            None,
            SessionContext::public_preview(),
        );

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/v1/agents/00000000-0000-0000-0000-000000000001")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn agent_stop_rejects_an_invalid_identity_before_asking_the_runtime() {
        let app = router_with_assets_and_session(
            Arc::new(FixturePresenceSource::nominal()),
            None,
            SessionContext::local_desktop(),
        );

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/v1/agents/not-a-capsule")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
