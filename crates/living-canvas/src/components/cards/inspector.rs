// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Universal Entity Inspector tool card component (ADR-0046 §5).

use cybou_protocol::SubjectQuery;
use cybou_web_contracts::SessionMode;
use leptos::prelude::*;
use lucide_leptos::{Check, Copy, FileText, Layers, RefreshCw, Shield};
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

    let tool_states = expect_context::<ToolCardStates>();
    let state = tool_states.inspector(CardId::Inspector(instance));
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

    let layout = use_context::<RwSignal<crate::DesktopLayout>>();

    let live_service = move || {
        if let Some(cybou_protocol::SubjectRef::Service { name, .. }) = target.get() {
            let svcs = tool_states.services(CardId::Services(0)).services.get();
            svcs.into_iter().find(|s| s.name == name)
        } else {
            None
        }
    };

    let live_process = move || {
        if let Some(cybou_protocol::SubjectRef::Process { pid, .. }) = target.get() {
            let procs = tool_states.processes(CardId::Processes(0)).processes.get();
            procs.into_iter().find(|p| p.pid == pid)
        } else {
            None
        }
    };

    let live_agent = move || {
        if let Some(cybou_protocol::SubjectRef::Agent { capsule_id, .. }) = target.get() {
            if let RuntimeState::Ready {
                agents: Some(agents),
                ..
            } = runtime.get()
            {
                agents
                    .into_iter()
                    .find(|a| a.capsule_id.to_string() == capsule_id)
            } else {
                None
            }
        } else {
            None
        }
    };

    let inspection_state_display = move || {
        if let Some(svc) = live_service() {
            format!(
                "Operational (State: {:?}, Substate: {})",
                svc.state, svc.substate
            )
        } else if let Some(proc) = live_process() {
            let mem_mb = proc.memory_bytes / (1024 * 1024);
            format!(
                "Running (State: {}, CPU: {:.1}%, RSS: {mem_mb} MB)",
                proc.state, proc.cpu_percent
            )
        } else if let Some(agent) = live_agent() {
            format!(
                "Supervised (Standing: {:?}, Workspace: {})",
                agent.standing, agent.workspace
            )
        } else if let Some(cybou_protocol::SubjectRef::File { location }) = target.get() {
            format!("Bounded file storage: {}", location.display_path())
        } else if target.get().is_some() {
            "Entity reference resolved, awaiting telemetry stream".to_string()
        } else {
            "Unresolved query awaiting owner resolution".to_string()
        }
    };

    let epistemic_basis_display = move || {
        if live_service().is_some() {
            "Host systemd unit via D-Bus / Boundary projection".to_string()
        } else if live_process().is_some() {
            "Linux kernel procfs telemetry".to_string()
        } else if live_agent().is_some() {
            "Agent1 capsule supervisor with Landlock sandbox bounds".to_string()
        } else if let Some(cybou_protocol::SubjectRef::File { .. }) = target.get() {
            "HostFiles1 filesystem boundary".to_string()
        } else if target.get().is_some() {
            "Authoritative owner boundary".to_string()
        } else {
            "User-supplied subject query".to_string()
        }
    };

    let last_observed_display = move || {
        if let Some(svc) = live_service() {
            let pid_desc = svc.main_pid.map_or("no pid".into(), |p| format!("PID {p}"));
            let mem_desc = svc.memory_bytes.map_or("unknown mem".into(), |b| {
                format!("{} MB", b / (1024 * 1024))
            });
            format!("{pid_desc}, {mem_desc}, enabled: {}", svc.enabled)
        } else if let Some(proc) = live_process() {
            format!(
                "Threads: {}, User: {}, PPID: {}",
                proc.threads, proc.user, proc.ppid
            )
        } else if let Some(agent) = live_agent() {
            format!(
                "Expires: {}, Model: {}",
                agent.expires_at.date(),
                agent.model_class.as_deref().unwrap_or("none")
            )
        } else if target.get().is_some() {
            "Observation pending owner stream refresh".to_string()
        } else {
            "Unknown — no observation received".to_string()
        }
    };

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
                                        <span class="val">{inspection_state_display}</span>
                                    </div>
                                    <div class="inspector-row">
                                        <span class="lbl">"Epistemic Basis"</span>
                                        <span class="val">{epistemic_basis_display}</span>
                                    </div>
                                    <div class="inspector-row">
                                        <span class="lbl">"Last Observed"</span>
                                        <span class="val">{last_observed_display}</span>
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
                                {move || {
                                    if let Some(svc) = live_service() {
                                        let pid_line = svc.main_pid.map(|pid| view! {
                                            <div class="relation-item">
                                                <Layers size=12 />
                                                <span class="rel-name">"Main Process"</span>
                                                <span class="rel-target">{format!("PID {pid}")}</span>
                                            </div>
                                        });
                                        let unit_name = svc.name.clone();
                                        let logs_line = view! {
                                            <div class="relation-item">
                                                <Layers size=12 />
                                                <span class="rel-name">"Journal Stream"</span>
                                                <span class="rel-target">{format!("journald unit {unit_name}")}</span>
                                            </div>
                                        };
                                        view! {
                                            <div class="inspector-relations-list">
                                                {pid_line}
                                                {logs_line}
                                                <div class="relation-item">
                                                    <Layers size=12 />
                                                    <span class="rel-name">"Unit Classification"</span>
                                                    <span class="rel-target">{format!("{:?}", svc.unit_type)}</span>
                                                </div>
                                            </div>
                                        }.into_any()
                                    } else if let Some(proc) = live_process() {
                                        view! {
                                            <div class="inspector-relations-list">
                                                <div class="relation-item">
                                                    <Layers size=12 />
                                                    <span class="rel-name">"Parent Process"</span>
                                                    <span class="rel-target">{format!("PPID {}", proc.ppid)}</span>
                                                </div>
                                                <div class="relation-item">
                                                    <Layers size=12 />
                                                    <span class="rel-name">"Execution Account"</span>
                                                    <span class="rel-target">{proc.user}</span>
                                                </div>
                                                <div class="relation-item">
                                                    <Layers size=12 />
                                                    <span class="rel-name">"Command Line"</span>
                                                    <span class="rel-target">{proc.cmdline}</span>
                                                </div>
                                            </div>
                                        }.into_any()
                                    } else if let Some(agent) = live_agent() {
                                        view! {
                                            <div class="inspector-relations-list">
                                                <div class="relation-item">
                                                    <Layers size=12 />
                                                    <span class="rel-name">"Workspace"</span>
                                                    <span class="rel-target">{agent.workspace}</span>
                                                </div>
                                                <div class="relation-item">
                                                    <Layers size=12 />
                                                    <span class="rel-name">"Model Class"</span>
                                                    <span class="rel-target">{agent.model_class.unwrap_or_else(|| "none granted".into())}</span>
                                                </div>
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <div class="inspector-relations-list">
                                                <div class="relation-item">
                                                    <Layers size=12 />
                                                    <span class="rel-name">"Unobserved"</span>
                                                    <span class="rel-target">"Relations have not been loaded from projection source"</span>
                                                </div>
                                            </div>
                                        }.into_any()
                                    }
                                }}
                            </div>
                        }.into_any(),
                    }}
                </div>

                // Actions Bar
                <div class="inspector-actions">
                    <button
                        class="inspector-btn"
                        on:click=move |_| {
                            if let Some(cybou_protocol::SubjectRef::Service { name, .. }) = target.get() {
                                if let Some(lay) = layout {
                                    let logs = tool_states.system_logs(CardId::SystemLogs(0));
                                    logs.search_query.set(name);
                                    crate::interaction::spawn_or_focus_card(lay, CardId::SystemLogs(0), Some(CardId::Inspector(instance)));
                                }
                                status_msg.set(Some("Focused system logs for service.".to_string()));
                            } else if let Some(cybou_protocol::SubjectRef::Process { .. }) = target.get() {
                                if let Some(lay) = layout {
                                    crate::interaction::spawn_or_focus_card(lay, CardId::Processes(0), Some(CardId::Inspector(instance)));
                                }
                                status_msg.set(Some("Focused process manager.".to_string()));
                            } else {
                                status_msg.set(Some("Telemetry stream: observation active.".to_string()));
                            }
                        }
                    >
                        <RefreshCw size=12 />
                        "Watch Telemetry"
                    </button>
                    <button
                        class="inspector-btn"
                        on:click=move |_| {
                            if let Some(sub) = target.get() {
                                let notes = tool_states.notes(CardId::Notes(0));
                                notes.selected_note_id.set(None);
                                notes.edit_title.set(format!("Notes: {}", sub.display_title()));
                                notes.edit_tags.set("inspection, system".to_string());
                                notes.edit_referenced_subject.set(Some(sub.clone()));
                                if notes.edit_content.get_untracked().is_empty() {
                                    notes.edit_content.set(format!("## Operational Assessment for {}\n\n- Epistemic note: operator investigation\n", sub.display_title()));
                                }
                                if let Some(lay) = layout {
                                    crate::interaction::spawn_or_focus_card(lay, CardId::Notes(0), Some(CardId::Inspector(instance)));
                                }
                                status_msg.set(Some("Opened personal notes linked to entity.".to_string()));
                            }
                        }
                    >
                        <FileText size=12 />
                        "Annotate"
                    </button>
                    <button
                        class="inspector-btn primary"
                        on:click=move |_| {
                            if let Some(cybou_protocol::SubjectRef::Service { .. }) = target.get() {
                                if let Some(lay) = layout {
                                    crate::interaction::spawn_or_focus_card(lay, CardId::Services(0), Some(CardId::Inspector(instance)));
                                }
                                status_msg.set(Some("Opened Services manager for unit control.".to_string()));
                            } else {
                                status_msg.set(Some("Permit request recorded; pending operator confirmation.".to_string()));
                            }
                        }
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
