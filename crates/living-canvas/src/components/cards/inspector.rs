// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Universal Entity Inspector tool card component (ADR-0046 §5).

use cybou_protocol::{EpistemicPresentation, SubjectQuery};
use cybou_web_contracts::SessionMode;
use leptos::prelude::*;
use lucide_leptos::{Check, Copy, Layers, RefreshCw, Shield};
use std::sync::Arc;
use web_sys::{KeyboardEvent, PointerEvent};

use crate::{
    CardId, DesktopItemId, DesktopLayout,
    components::card_frame::CardFrame,
    interaction::{DragState, ResizeState},
    state::RuntimeState,
    tool_state::ToolCardStates,
};

/// Preset categories for quick subject selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PresetCategory {
    Services,
    Files,
    Agents,
    System,
}

/// Detail tab inside the entity inspector.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InspectorTab {
    Attributes,
    RawSpec,
    Relations,
}

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
    let subject_query = state.subject_query;
    let status_msg = state.status_msg;

    let active_category = RwSignal::new(PresetCategory::Services);
    let active_tab = RwSignal::new(InspectorTab::Attributes);

    let custom_kind = RwSignal::new("service".to_string());
    let custom_input = RwSignal::new(String::new());

    let copied_uri = RwSignal::new(false);
    let copied_hash = RwSignal::new(false);
    let copied_json = RwSignal::new(false);

    // No owner-backed SubjectProjection resolver is connected yet. Keep this explicit so the
    // Inspector cannot accidentally present a selected reference as observed system state.
    let inspection_state = EpistemicPresentation::<()>::Unavailable {
        reason: "No authoritative inspection projection is connected for this subject.".to_string(),
    };
    let inspection_reason = StoredValue::new(match inspection_state {
        EpistemicPresentation::Unavailable { reason } => reason,
        _ => unreachable!("the prototype Inspector has no projection source"),
    });

    let select_query = move |query: SubjectQuery| {
        target.set(None);
        subject_query.set(Some(query));
        status_msg.set(Some(
            "Subject query recorded; authoritative owner resolution is unavailable.".to_string(),
        ));
    };

    let apply_custom_subject = move || {
        let val = custom_input.get();
        let trimmed = val.trim();
        if trimmed.is_empty() {
            return;
        }
        let kind = custom_kind.get();
        let identifier = trimmed.to_string();
        select_query(match kind.as_str() {
            "service" => SubjectQuery::Service(identifier),
            "file" => SubjectQuery::File(identifier),
            "agent" => SubjectQuery::Agent(identifier),
            "package" => SubjectQuery::Package(identifier),
            "anchor" => SubjectQuery::Anchor(identifier),
            _ => SubjectQuery::Service(identifier),
        });
        custom_input.set(String::new());
    };

    let copy_text_to_clipboard = move |text: String, flag: RwSignal<bool>| {
        if let Some(win) = web_sys::window() {
            let nav = win.navigator();
            let clipboard = nav.clipboard();
            let _ = clipboard.write_text(&text);
            flag.set(true);
            leptos::task::spawn_local(async move {
                gloo_timers::future::TimeoutFuture::new(1500).await;
                flag.set(false);
            });
        }
    };

    let subject_json = move || {
        if let Some(subject) = target.get() {
            serde_json::to_string_pretty(&subject).unwrap_or_else(|_| "{}".to_string())
        } else if let Some(query) = subject_query.get() {
            serde_json::to_string_pretty(&query).unwrap_or_else(|_| "{}".to_string())
        } else {
            "{}".to_string()
        }
    };

    let selected_kind = move || {
        target.get().map_or_else(
            || {
                subject_query
                    .get()
                    .map_or("No subject", |query| query.kind_name())
            },
            |subject| subject.kind_name(),
        )
    };
    let selected_title = move || {
        target.get().map_or_else(
            || {
                subject_query.get().map_or_else(
                    || "Choose a subject".to_string(),
                    |query| query.identifier().to_string(),
                )
            },
            |subject| subject.display_title(),
        )
    };

    view! {
        <Show
            when=move || !is_public_preview()
            fallback=move || view! {
                <div class="card-auth-gate">
                    <Shield size=26 />
                    <strong>"Inspector Locked"</strong>
                    <p>"Public preview does not permit deep system inspection. Sign in to unlock."</p>
                    <button class="primary-btn" on:click=move |_| auth_modal_open.set(true)>"Sign in"</button>
                </div>
            }
        >
            <div class="inspector-body" on:pointerdown=move |e: PointerEvent| e.stop_propagation()>
                // Category Presets Tabs
                <div class="inspector-category-nav">
                    <button
                        type="button"
                        class="inspector-cat-btn"
                        class:active=move || active_category.get() == PresetCategory::Services
                        on:click=move |_| active_category.set(PresetCategory::Services)
                    >
                        "Services"
                    </button>
                    <button
                        type="button"
                        class="inspector-cat-btn"
                        class:active=move || active_category.get() == PresetCategory::Files
                        on:click=move |_| active_category.set(PresetCategory::Files)
                    >
                        "Files"
                    </button>
                    <button
                        type="button"
                        class="inspector-cat-btn"
                        class:active=move || active_category.get() == PresetCategory::Agents
                        on:click=move |_| active_category.set(PresetCategory::Agents)
                    >
                        "Agents"
                    </button>
                    <button
                        type="button"
                        class="inspector-cat-btn"
                        class:active=move || active_category.get() == PresetCategory::System
                        on:click=move |_| active_category.set(PresetCategory::System)
                    >
                        "System"
                    </button>
                </div>

                // Quick Selector Bar per Category
                <div class="inspector-quick-bar">
                    {move || match active_category.get() {
                        PresetCategory::Services => view! {
                            <div class="inspector-chips-group">
                                <button class="inspector-chip" on:click=move |_| select_query(SubjectQuery::Service("cybou-mind.target".into()))>"cybou-mind.target"</button>
                                <button class="inspector-chip" on:click=move |_| select_query(SubjectQuery::Service("caddy.service".into()))>"caddy.service"</button>
                                <button class="inspector-chip" on:click=move |_| select_query(SubjectQuery::Service("systemd-journald.service".into()))>"journald.service"</button>
                                <button class="inspector-chip" on:click=move |_| select_query(SubjectQuery::Service("ssh.service".into()))>"ssh.service"</button>
                            </div>
                        }.into_any(),
                        PresetCategory::Files => view! {
                            <div class="inspector-chips-group">
                                <button class="inspector-chip" on:click=move |_| select_query(SubjectQuery::File("/etc/caddy/Caddyfile".into()))>"/etc/caddy/Caddyfile"</button>
                                <button class="inspector-chip" on:click=move |_| select_query(SubjectQuery::File("/etc/cybou/config.toml".into()))>"/etc/cybou/config.toml"</button>
                                <button class="inspector-chip" on:click=move |_| select_query(SubjectQuery::File("/etc/hosts".into()))>"/etc/hosts"</button>
                                <button class="inspector-chip" on:click=move |_| select_query(SubjectQuery::File("/var/log/syslog".into()))>"/var/log/syslog"</button>
                            </div>
                        }.into_any(),
                        PresetCategory::Agents => view! {
                            <div class="inspector-chips-group">
                                <span>"Enter an owner-issued capsule ID below; no agent identities are assumed."</span>
                            </div>
                        }.into_any(),
                        PresetCategory::System => view! {
                            <div class="inspector-chips-group">
                                <button class="inspector-chip" on:click=move |_| select_query(SubjectQuery::Package("cybou-mind".into()))>"Package query: cybou-mind"</button>
                                <button class="inspector-chip" on:click=move |_| select_query(SubjectQuery::Anchor("home".into()))>"Anchor query: home"</button>
                            </div>
                        }.into_any(),
                    }}
                </div>

                // Custom Subject Entry Bar
                <div class="inspector-input-row">
                    <select
                        class="inspector-custom-select"
                        prop:value=move || custom_kind.get()
                        on:change=move |e| custom_kind.set(event_target_value(&e))
                    >
                        <option value="service">"Service"</option>
                        <option value="file">"File"</option>
                        <option value="agent">"Agent"</option>
                        <option value="package">"Package"</option>
                        <option value="anchor">"Anchor"</option>
                    </select>
                    <input
                        type="text"
                        class="inspector-custom-input"
                        placeholder="Target identifier or path…"
                        prop:value=move || custom_input.get()
                        on:input=move |e| custom_input.set(event_target_value(&e))
                        on:keydown=move |e: KeyboardEvent| {
                            if e.key() == "Enter" {
                                apply_custom_subject();
                            }
                        }
                    />
                    <button type="button" class="inspector-custom-btn" on:click=move |_| apply_custom_subject()>"Inspect"</button>
                </div>

                // Main Subject Header
                <div class="inspector-header">
                    <div class="inspector-header-top">
                        <div class="inspector-badge">{selected_kind}</div>
                        <h3 class="inspector-title">{selected_title}</h3>
                    </div>

                    // URI & Deep Link
                    <Show
                        when=move || target.get().is_some()
                        fallback=move || view! {
                            <div class="inspector-uri-row">
                                <code class="inspector-uri">"Unresolved query — no canonical URI"</code>
                            </div>
                        }
                    >
                      <div class="inspector-uri-row">
                        <code class="inspector-uri">{move || target.get().map_or_else(String::new, |subject| subject.uri())}</code>
                        <button
                            type="button"
                            class="inspector-copy-btn"
                            title="Copy owner-resolved canonical URI"
                            on:click=move |_| {
                                if let Some(subject) = target.get() {
                                    copy_text_to_clipboard(subject.uri(), copied_uri);
                                }
                            }
                        >
                            {move || if copied_uri.get() {
                                view! { <Check size=12 /> <span>"Copied"</span> }.into_any()
                            } else {
                                view! { <Copy size=12 /> <span>"URI"</span> }.into_any()
                            }}
                        </button>
                        <button
                            type="button"
                            class="inspector-copy-btn"
                            title="Copy Deep Link"
                            on:click=move |_| {
                                if let Some(subject) = target.get() {
                                    copy_text_to_clipboard(subject.deep_link_hash(), copied_hash);
                                }
                            }
                        >
                            {move || if copied_hash.get() {
                                view! { <Check size=12 /> <span>"Copied"</span> }.into_any()
                            } else {
                                view! { <Copy size=12 /> <span>"Hash"</span> }.into_any()
                            }}
                        </button>
                      </div>
                    </Show>
                </div>

                // View Tabs
                <div class="inspector-tabs-nav">
                    <button
                        type="button"
                        class="inspector-tab-btn"
                        class:active=move || active_tab.get() == InspectorTab::Attributes
                        on:click=move |_| active_tab.set(InspectorTab::Attributes)
                    >
                        "Attributes"
                    </button>
                    <button
                        type="button"
                        class="inspector-tab-btn"
                        class:active=move || active_tab.get() == InspectorTab::RawSpec
                        on:click=move |_| active_tab.set(InspectorTab::RawSpec)
                    >
                        "JSON Spec"
                    </button>
                    <button
                        type="button"
                        class="inspector-tab-btn"
                        class:active=move || active_tab.get() == InspectorTab::Relations
                        on:click=move |_| active_tab.set(InspectorTab::Relations)
                    >
                        "Relations"
                    </button>
                </div>

                // Tab Content Panes
                <div class="inspector-tab-content">
                    {move || match active_tab.get() {
                        InspectorTab::Attributes => view! {
                            <div class="inspector-section">
                                <div class="inspector-section-title">"Entity Attributes & Boundaries"</div>
                                <div class="inspector-grid">
                                    <div class="inspector-row">
                                        <span class="lbl">"Kind"</span>
                                        <span class="val">{selected_kind}</span>
                                    </div>
                                    <div class="inspector-row">
                                        <span class="lbl">"Identifier"</span>
                                        <span class="val">{selected_title}</span>
                                    </div>
                                    <div class="inspector-row">
                                        <span class="lbl">"Resolution State"</span>
                                        <span class="val">{move || if target.get().is_some() { "Owner-resolved reference" } else { "Unresolved query" }}</span>
                                    </div>
                                    <div class="inspector-row">
                                        <span class="lbl">"Inspection State"</span>
                                        <span class="val">"Unavailable"</span>
                                    </div>
                                    <div class="inspector-row">
                                        <span class="lbl">"Epistemic Basis"</span>
                                        <span class="val">{move || inspection_reason.get_value()}</span>
                                    </div>
                                    <div class="inspector-row">
                                        <span class="lbl">"Last Observed"</span>
                                        <span class="val">"Unknown — no observation received"</span>
                                    </div>
                                </div>
                            </div>
                        }.into_any(),
                        InspectorTab::RawSpec => view! {
                            <div class="inspector-section">
                                <div class="inspector-spec-header">
                                    <span class="inspector-section-title">{move || if target.get().is_some() { "Canonical SubjectRef JSON" } else { "Unresolved SubjectQuery JSON" }}</span>
                                    <button
                                        type="button"
                                        class="inspector-copy-btn"
                                        on:click=move |_| copy_text_to_clipboard(subject_json(), copied_json)
                                    >
                                        {move || if copied_json.get() {
                                            view! { <Check size=12 /> <span>"Copied"</span> }.into_any()
                                        } else {
                                            view! { <Copy size=12 /> <span>"Copy JSON"</span> }.into_any()
                                        }}
                                    </button>
                                </div>
                                <pre class="inspector-json-block"><code>{move || subject_json()}</code></pre>
                            </div>
                        }.into_any(),
                        InspectorTab::Relations => view! {
                            <div class="inspector-section">
                                <div class="inspector-section-title">"Connected System Relations"</div>
                                <div class="inspector-relations-list">
                                    <div class="relation-item">
                                        <Layers size=12 />
                                        <span class="rel-name">"Unavailable"</span>
                                        <span class="rel-target">"Relations have not been loaded from projection source"</span>
                                    </div>
                                </div>
                            </div>
                        }.into_any(),
                    }}
                </div>

                // Actions Bar
                <div class="inspector-actions">
                    <button
                        class="inspector-btn"
                        on:click=move |_| status_msg.set(Some("Telemetry watch unavailable — no subject projection resolver is connected.".to_string()))
                    >
                        <RefreshCw size=12 />
                        "Watch Telemetry"
                    </button>
                    <button
                        class="inspector-btn primary"
                        on:click=move |_| status_msg.set(Some("Action proposal unavailable — Inspector actions are not connected to Action1.".to_string()))
                    >
                        "Propose Action"
                    </button>
                </div>

                // Status Message
                <Show when=move || status_msg.get().is_some()>
                    <div class="inspector-status" role="status" aria-live="polite">
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
            kicker_icon=Arc::new(|| view! { <Layers size=14 /> }.into_any())
            collapsed_summary=Arc::new(collapsed)
        >
            <InspectorContent runtime=runtime auth_modal_open=auth_modal_open instance=instance />
        </CardFrame>
    }
}
