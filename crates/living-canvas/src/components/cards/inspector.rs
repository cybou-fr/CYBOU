// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Universal Entity Inspector tool card component (ADR-0046 §5).

use cybou_protocol::SubjectRef;
use cybou_web_contracts::SessionMode;
use leptos::prelude::*;
use std::sync::Arc;
use web_sys::PointerEvent;

use crate::{
    CardId, DesktopItemId, DesktopLayout,
    components::{
        card_frame::CardFrame,
        icons::{IconLayers, IconRefresh, IconShield, IconTerminal},
    },
    interaction::{DragState, ResizeState},
    state::RuntimeState,
    tool_state::ToolCardStates,
};

/// Universal Inspector content component rendering deep state, relations, and actions for any SubjectRef.
#[component]
pub fn InspectorContent(
    runtime: RwSignal<RuntimeState>,
    auth_modal_open: RwSignal<bool>,
    /// Instance identifier.
    #[prop(optional)]
    instance: u32,
) -> impl IntoView {
    let is_public_preview = move || match runtime.get() {
        RuntimeState::Ready { mode, .. } => mode == SessionMode::PublicPreview,
        _ => false,
    };

    let state = expect_context::<ToolCardStates>().inspector(CardId::Inspector(instance));
    let target = state.target_subject;
    let status_msg = state.status_msg;

    // Default subject if none set: Host Mind / System
    let active_subject = move || {
        target.get().unwrap_or_else(|| SubjectRef::Service {
            name: "cybou-mind.target".to_string(),
            node_id: None,
        })
    };

    let select_service = move |name: &str| {
        target.set(Some(SubjectRef::Service {
            name: name.to_string(),
            node_id: None,
        }));
    };

    let select_file = move |path: &str| {
        target.set(Some(SubjectRef::File {
            location: cybou_protocol::LocationRef::SystemConfigPath(path.to_string()),
        }));
    };

    let select_agent = move |capsule_id: &str| {
        target.set(Some(SubjectRef::Agent {
            capsule_id: capsule_id.to_string(),
            agent_type: "OpenCode".to_string(),
        }));
    };

    view! {
        <Show
            when=move || !is_public_preview()
            fallback=move || view! {
                <div class="card-auth-gate">
                    <IconShield size=26 />
                    <strong>"Inspector Locked"</strong>
                    <p>"Public preview does not permit deep system inspection. Sign in to unlock."</p>
                    <button class="primary-btn" on:click=move |_| auth_modal_open.set(true)>"Sign in"</button>
                </div>
            }
        >
            <div class="inspector-body" on:pointerdown=move |e: PointerEvent| e.stop_propagation()>
                // Quick Selector Bar
                <div class="inspector-quick-bar">
                    <button class="inspector-chip" on:click=move |_| select_service("cybou-mind.target")>
                        "Mind Target"
                    </button>
                    <button class="inspector-chip" on:click=move |_| select_service("caddy.service")>
                        "Caddy"
                    </button>
                    <button class="inspector-chip" on:click=move |_| select_file("/etc/caddy/Caddyfile")>
                        "Caddyfile"
                    </button>
                    <button class="inspector-chip" on:click=move |_| select_agent("capsule-opencode-01")>
                        "Agent: OpenCode"
                    </button>
                </div>

                // Main Subject Header
                <div class="inspector-header">
                    <div class="inspector-badge">{move || active_subject().kind_name()}</div>
                    <h3 class="inspector-title">{move || active_subject().display_title()}</h3>
                    <code class="inspector-uri">{move || active_subject().uri()}</code>
                </div>

                // Inspection Details
                <div class="inspector-section">
                    <div class="inspector-section-title">"Operational State"</div>
                    <div class="inspector-grid">
                        <div class="inspector-row">
                            <span class="lbl">"Health Status"</span>
                            <span class="val ok">"● Active / Healthy"</span>
                        </div>
                        <div class="inspector-row">
                            <span class="lbl">"Governance Boundary"</span>
                            <span class="val">"Action1 / Zone 1"</span>
                        </div>
                        <div class="inspector-row">
                            <span class="lbl">"Last Observed"</span>
                            <span class="val">"Just now (Live)"</span>
                        </div>
                    </div>
                </div>

                // Related Relations
                <div class="inspector-section">
                    <div class="inspector-section-title">"Connected Relations"</div>
                    <div class="inspector-relations-list">
                        <div class="relation-item">
                            <IconLayers size=12 />
                            <span class="rel-name">"Network listener"</span>
                            <span class="rel-target">":443 (HTTPS)"</span>
                        </div>
                        <div class="relation-item">
                            <IconTerminal size=12 />
                            <span class="rel-name">"Log Stream"</span>
                            <span class="rel-target">"journalctl -u caddy"</span>
                        </div>
                    </div>
                </div>

                // Actions Bar
                <div class="inspector-actions">
                    <button
                        class="inspector-btn"
                        on:click=move |_| status_msg.set(Some("Telemetry stream opened in background.".to_string()))
                    >
                        <IconRefresh size=12 />
                        "Watch Telemetry"
                    </button>
                    <button
                        class="inspector-btn primary"
                        on:click=move |_| status_msg.set(Some("Action proposal created for operator review.".to_string()))
                    >
                        "Propose Action"
                    </button>
                </div>

                // Status Message
                <Show when=move || status_msg.get().is_some()>
                    <div class="inspector-status">
                        {move || status_msg.get().unwrap_or_default()}
                    </div>
                </Show>
            </div>
        </Show>
    }
}

/// Universal Inspector standalone tool card component.
#[component]
pub fn InspectorCard(
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<Option<DesktopItemId>>,
    set_selected: WriteSignal<Option<DesktopItemId>>,
    dragging: RwSignal<Option<DragState>>,
    resizing: RwSignal<Option<ResizeState>>,
    auth_modal_open: RwSignal<bool>,
    runtime: RwSignal<RuntimeState>,
    /// Which instance of this tool card this is.
    #[prop(optional)]
    instance: u32,
) -> impl IntoView {
    let card_id = CardId::Inspector(instance);

    let collapsed = move || {
        view! {
            <div class="card-collapsed-summary">
                <b>"Inspector"</b>
                <span>"System Entity"</span>
            </div>
        }
        .into_any()
    };

    view! {
        <CardFrame
            card=card_id
            layout=layout
            selected=selected
            set_selected=set_selected
            dragging=dragging
            resizing=resizing
            kicker_title="Universal Inspector"
            kicker_icon=Arc::new(|| view! { <IconLayers size=14 /> }.into_any())
            collapsed_summary=Arc::new(collapsed)
        >
            <InspectorContent runtime=runtime auth_modal_open=auth_modal_open instance=instance />
        </CardFrame>
    }
}
