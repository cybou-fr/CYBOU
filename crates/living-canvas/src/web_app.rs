// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

use leptos::prelude::*;
use leptos::task::spawn_local;
use living_canvas::{CardGeometry, CardId, DesktopLayout, GatewayMindClient, MindClient};
use lucide_leptos::{
    Ellipsis, FileCheck, Files, FolderOpen, Link, ListChecks, Map, Search, Sparkles, UsersRound,
};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{EventSource, HtmlElement, KeyboardEvent, MessageEvent, PointerEvent};

#[derive(Clone, Copy, Debug)]
struct DragState {
    card: CardId,
    offset_x: f64,
    offset_y: f64,
    width: f64,
    height: f64,
}

#[derive(Clone, Copy, Debug)]
struct ResizeState {
    card: CardId,
    start_pointer_x: f64,
    start_pointer_y: f64,
    start_width: f64,
    start_height: f64,
}

#[derive(Clone, Debug)]
enum RuntimeState {
    Loading,
    Ready {
        mode: cybou_web_contracts::SessionMode,
        snapshot: cybou_web_contracts::SnapshotProjection,
        /// `None` when the gateway could not assemble the owner projection at all. It is kept
        /// distinct from a projection whose sections are individually unknown.
        mind: Option<cybou_web_contracts::MindProjection>,
    },
    Error(String),
}

#[component]
fn IconPin(#[prop(default = 12)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="12" y1="17" x2="12" y2="22"></line>
            <path d="M5 17h14v-1.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a1 1 0 0 0 0-2H8a1 1 0 0 0 0 2h1v4.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24Z"></path>
        </svg>
    }
}

#[component]
fn IconMinimize(#[prop(default = 12)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="4 14 10 14 10 20"></polyline>
            <polyline points="20 10 14 10 14 4"></polyline>
            <line x1="14" y1="10" x2="21" y2="3"></line>
            <line x1="3" y1="21" x2="10" y2="14"></line>
        </svg>
    }
}

#[component]
fn IconMaximize(#[prop(default = 12)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="15 3 21 3 21 9"></polyline>
            <polyline points="9 21 3 21 3 15"></polyline>
            <line x1="21" y1="3" x2="14" y2="10"></line>
            <line x1="3" y1="21" x2="10" y2="14"></line>
        </svg>
    }
}

#[component]
fn IconResizeGrip() -> impl IntoView {
    view! {
        <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round">
            <line x1="8" y1="2" x2="2" y2="8"></line>
            <line x1="8" y1="5" x2="5" y2="8"></line>
            <line x1="8" y1="8" x2="8" y2="8"></line>
        </svg>
    }
}

#[component]
fn CardControls(card: CardId, layout: RwSignal<DesktopLayout>) -> impl IntoView {
    let is_pinned = move || layout.get().presentation(card).pinned;
    let is_collapsed = move || layout.get().presentation(card).collapsed;

    view! {
        <div class="card-controls" on:pointerdown=move |e: PointerEvent| e.stop_propagation() on:click=move |e: web_sys::MouseEvent| e.stop_propagation()>
            <button
                class:active=is_pinned
                class="card-control-btn pin-btn"
                title=move || if is_pinned() { "Unpin card" } else { "Pin card (lock position)" }
                aria-label=move || if is_pinned() { "Unpin card" } else { "Pin card" }
                on:click=move |_| {
                    layout.update(|current| {
                        let p = current.presentation(card);
                        current.set_pinned(card, !p.pinned);
                    });
                    layout.get_untracked().save();
                }
            >
                <IconPin size=12 />
            </button>
            <button
                class:active=is_collapsed
                class="card-control-btn collapse-btn"
                title=move || if is_collapsed() { "Expand card" } else { "Collapse card" }
                aria-label=move || if is_collapsed() { "Expand card" } else { "Collapse card" }
                on:click=move |_| {
                    layout.update(|current| {
                        let p = current.presentation(card);
                        current.set_collapsed(card, !p.collapsed);
                    });
                    layout.get_untracked().save();
                }
            >
                {move || if is_collapsed() {
                    view! { <IconMaximize size=12 /> }.into_any()
                } else {
                    view! { <IconMinimize size=12 /> }.into_any()
                }}
            </button>
        </div>
    }
}

#[component]
fn CardResizeHandle(
    card: CardId,
    layout: RwSignal<DesktopLayout>,
    resizing: RwSignal<Option<ResizeState>>,
) -> impl IntoView {
    let is_collapsed = move || layout.get().presentation(card).collapsed;
    view! {
        <Show when=move || !is_collapsed()>
            <div
                class="card-resize-handle"
                title="Resize"
                aria-label="Resize card"
                on:pointerdown=move |event| start_resize(event, card, layout, resizing)
                on:click=move |e: web_sys::MouseEvent| e.stop_propagation()
            >
                <IconResizeGrip />
            </div>
        </Show>
    }
}

#[component]
fn RelationshipEdge(
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<&'static str>,
    from: CardId,
    to: CardId,
    label: &'static str,
    amber: bool,
) -> impl IntoView {
    let points = move || relationship_points(layout.get(), from, to);
    view! {
        <g
            class:amber=amber
            class:active=move || selected.get() == from.key() || selected.get() == to.key()
            class="relationship-edge"
        >
            <line
                x1=move || points().0.to_string()
                y1=move || points().1.to_string()
                x2=move || points().2.to_string()
                y2=move || points().3.to_string()
            />
            <text
                x=move || points().4.to_string()
                y=move || points().5.to_string()
                text-anchor="middle"
            >{label}</text>
        </g>
    }
}

#[component]
pub fn App() -> impl IntoView {
    let (selected, set_selected) = signal("capabilities");
    let (runtime_menu_open, set_runtime_menu_open) = signal(false);
    let (minimap_visible, set_minimap_visible) = signal(true);
    let (command_open, set_command_open) = signal(false);
    let (command_query, set_command_query) = signal(String::new());
    let (capabilities_open, set_capabilities_open) = signal(false);
    let command_input = NodeRef::<leptos::html::Input>::new();
    let layout = RwSignal::new(load_layout());
    let dragging = RwSignal::new(None::<DragState>);
    let resizing = RwSignal::new(None::<ResizeState>);
    let runtime = RwSignal::new(RuntimeState::Loading);
    spawn_local(async move {
        let client = GatewayMindClient;
        let result = async {
            let session = client.session().await?;
            let snapshot = client.snapshot().await?;
            Ok::<_, living_canvas::ClientError>((session.mode, snapshot))
        }
        .await;
        // The owner projection is fetched separately and allowed to fail on its own: capabilities
        // are still worth showing when Identity1 or the Journal cannot be read.
        let mind = client.mind().await.ok();
        runtime.set(match result {
            Ok((mode, snapshot)) => RuntimeState::Ready {
                mode,
                snapshot,
                mind,
            },
            Err(error) => RuntimeState::Error(error.to_string()),
        });
    });

    if let Ok(events) = EventSource::new("/api/v1/events") {
        let on_snapshot = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
            let Some(data) = event.data().as_string() else {
                return;
            };
            let Ok(snapshot) =
                serde_json::from_str::<cybou_web_contracts::SnapshotProjection>(&data)
            else {
                return;
            };
            runtime.update(|state| {
                if let RuntimeState::Ready {
                    snapshot: current, ..
                } = state
                {
                    *current = snapshot;
                }
            });
        });
        if events
            .add_event_listener_with_callback("snapshot", on_snapshot.as_ref().unchecked_ref())
            .is_ok()
        {
            // App is the page root and is mounted once; keep the browser stream for that lifetime.
            on_snapshot.forget();
            std::mem::forget(events);
        }
    }

    if let Some(window) = web_sys::window() {
        let on_shortcut = Closure::<dyn FnMut(KeyboardEvent)>::new(move |event: KeyboardEvent| {
            if (event.ctrl_key() || event.meta_key()) && event.key().eq_ignore_ascii_case("k") {
                event.prevent_default();
                set_command_open.set(true);
                if let Some(input) = command_input.get() {
                    let _ = input.focus();
                }
            } else if event.key() == "Escape" {
                set_command_open.set(false);
                set_command_query.set(String::new());
                set_capabilities_open.set(false);
            }
        });
        if window
            .add_event_listener_with_callback("keydown", on_shortcut.as_ref().unchecked_ref())
            .is_ok()
        {
            on_shortcut.forget();
        }
    }

    let runtime_label = move || match runtime.get() {
        RuntimeState::Loading => "Connecting".to_owned(),
        RuntimeState::Ready { mode, .. } => match mode {
            cybou_web_contracts::SessionMode::LocalDesktop => "Local".to_owned(),
            cybou_web_contracts::SessionMode::PublicPreview => "Preview".to_owned(),
            cybou_web_contracts::SessionMode::RemoteBrowser => "Remote".to_owned(),
        },
        RuntimeState::Error(_) => "Unavailable".to_owned(),
    };
    let projection_label = move || match runtime.get() {
        RuntimeState::Loading => "Loading projection…".into(),
        RuntimeState::Ready { snapshot, .. } => {
            format!("Gateway · projection {}", snapshot.projection_version)
        }
        RuntimeState::Error(error) => error,
    };
    let system_label = move || match runtime.get() {
        RuntimeState::Loading => "Connecting to local gateway…".into(),
        RuntimeState::Ready { snapshot, .. } => {
            let available = snapshot
                .capabilities
                .iter()
                .filter(|capability| capability.state == cybou_protocol::CapabilityState::Available)
                .count();
            format!(
                "{available}/{} capabilities available · projection {}",
                snapshot.capabilities.len(),
                snapshot.projection_version
            )
        }
        RuntimeState::Error(_) => "Gateway unavailable · canvas remains read-only".into(),
    };
    let observed_label = move || match runtime.get() {
        RuntimeState::Ready { snapshot, .. } => format!("Observed {}", snapshot.observed_at),
        RuntimeState::Loading => "Waiting for first snapshot".into(),
        RuntimeState::Error(_) => "No current snapshot".into(),
    };
    let capabilities = move || match runtime.get() {
        RuntimeState::Ready { snapshot, .. } => snapshot.capabilities,
        RuntimeState::Loading | RuntimeState::Error(_) => Vec::new(),
    };
    let mind = move || match runtime.get() {
        RuntimeState::Ready { mind, .. } => mind,
        RuntimeState::Loading | RuntimeState::Error(_) => None,
    };
    // Every reader below answers "not read" rather than inventing a placeholder value, because a
    // dash is honest and a zero is a claim.
    let identity_id = move || {
        mind()
            .and_then(|m| m.identity.identity_id)
            .unwrap_or_else(unread)
    };
    let identity_origin = move || {
        mind()
            .and_then(|m| m.identity.origin)
            .unwrap_or_else(unread)
    };
    let identity_sessions = move || {
        mind()
            .and_then(|m| m.identity.session_count)
            .map_or_else(unread, |value| value.to_string())
    };
    let identity_age = move || {
        mind()
            .and_then(|m| m.identity.age_in_days)
            .map_or_else(unread, |value| format!("{value} d"))
    };
    let identity_architecture = move || {
        mind()
            .and_then(|m| m.identity.architecture_version)
            .unwrap_or_else(unread)
    };
    let journal_count = move || {
        mind()
            .and_then(|m| m.journal.contribution_count)
            .map_or_else(unread, |value| value.to_string())
    };
    let journal_epoch = move || {
        mind()
            .and_then(|m| m.journal.erasure_epoch)
            .map_or_else(unread, |value| value.to_string())
    };
    let journal_recent = move || mind().map_or_else(Vec::new, |m| m.journal.recent);
    let journal_integrity = move || {
        mind()
            .and_then(|m| m.journal.integrity)
            .unwrap_or_else(|| "not verified yet".to_owned())
    };
    let journal_state = move || {
        mind().map_or_else(
            || "Event1 not read".to_owned(),
            |m| knowledge_label(m.journal.knowledge).to_owned(),
        )
    };
    let lifecycle_mode = move || mind().and_then(|m| m.lifecycle.mode).unwrap_or_else(unread);
    let lifecycle_activity = move || {
        mind()
            .and_then(|m| m.lifecycle.last_user_activity_at)
            .unwrap_or_else(unread)
    };
    let self_narration = move || {
        mind()
            .and_then(|m| m.self_model.narration)
            .unwrap_or_else(|| "Self1 has not been read.".to_owned())
    };
    let self_open_intentions = move || {
        mind()
            .and_then(|m| m.self_model.open_intentions)
            .map_or_else(unread, |value| value.to_string())
    };
    let self_settled = move || {
        mind()
            .and_then(|m| m.self_model.settled_predictions)
            .map_or_else(unread, |value| value.to_string())
    };
    let attention_focus = move || match mind() {
        None => "Workspace1 not read".to_owned(),
        Some(m) if m.attention.knowledge != cybou_protocol::KnowledgeState::Known => {
            "Workspace1 not read".to_owned()
        }
        // Workspace1 answering with no winner is knowledge: nothing currently holds attention.
        Some(m) => m
            .attention
            .focus
            .unwrap_or_else(|| "Nothing holds focus".to_owned()),
    };
    let attention_salience = move || {
        mind()
            .and_then(|m| m.attention.salience)
            .map_or_else(unread, |value| format!("{value:.2}"))
    };
    let attention_organs = move || {
        let organs = mind().map_or_else(Vec::new, |m| m.attention.organs);
        if organs.is_empty() {
            unread()
        } else {
            organs.join(", ")
        }
    };
    let beliefs = move || mind().map_or_else(Vec::new, |m| m.beliefs.beliefs);
    let beliefs_label = move || match mind() {
        None => "Epistemic1 not read".to_owned(),
        Some(m) if m.beliefs.knowledge != cybou_protocol::KnowledgeState::Known => {
            "Epistemic1 not read".to_owned()
        }
        Some(m) => match m.beliefs.beliefs.len() {
            0 => "Believes nothing yet".to_owned(),
            1 => "1 belief".to_owned(),
            count => format!("{count} beliefs"),
        },
    };
    let perception_status = move || {
        mind()
            .and_then(|m| m.perception.status)
            .unwrap_or_else(unread)
    };
    let perception_source = move || {
        mind()
            .and_then(|m| m.perception.source_id)
            .unwrap_or_else(unread)
    };
    let perception_at = move || {
        mind()
            .and_then(|m| m.perception.acquired_at)
            .unwrap_or_else(unread)
    };
    let concepts = move || mind().map_or_else(Vec::new, |m| m.context.concepts);
    let context_label = move || match mind() {
        None => "Context1 not read".to_owned(),
        Some(m) if m.context.knowledge != cybou_protocol::KnowledgeState::Known => {
            "Context1 not read".to_owned()
        }
        Some(m) => match m.context.concepts.len() {
            0 => "Nothing activated yet".to_owned(),
            1 => "1 active concept".to_owned(),
            count => format!("{count} active concepts"),
        },
    };
    let commitments = move || mind().map_or_else(Vec::new, |m| m.commitments.open);
    let commitments_label = move || match mind() {
        None => "Intention1 not read".to_owned(),
        Some(m) if m.commitments.knowledge != cybou_protocol::KnowledgeState::Known => {
            "Intention1 not read".to_owned()
        }
        Some(m) => match m.commitments.open_count.unwrap_or_default() {
            0 => "No open commitments".to_owned(),
            1 => "1 open commitment".to_owned(),
            count => format!("{count} open commitments"),
        },
    };
    let session_consumer = move || match runtime.get() {
        RuntimeState::Ready { .. } => "living-canvas".to_owned(),
        RuntimeState::Loading | RuntimeState::Error(_) => unread(),
    };
    let mind_observed = move || {
        mind().map_or_else(
            || "owners not read".to_owned(),
            |m| format!("Owners read {}", m.observed_at),
        )
    };

    view! {
            <main class="app-shell">
                <header class="topbar">
                    <a class="brand" href="#canvas" aria-label="Cybou home">
                        <img class="brand-mark" src="/cybou-mark.svg" alt="" />
                        <span>"Cybou"</span>
                    </a>
                    <div class="runtime-cluster">
                        <div class="runtime" aria-label="Runtime connection" aria-live="polite">
                            <span class="status-dot" aria-hidden="true"></span>
                            <strong>{runtime_label}</strong>
                            <small>{projection_label}</small>
                        </div>
                        <button
                            class="runtime-switch"
                            aria-expanded=move || runtime_menu_open.get().to_string()
                            aria-controls="runtime-menu"
                            on:click=move |_| set_runtime_menu_open.update(|open| *open = !*open)
                        >
                            <span>"Local"</span>
                            <span class="inactive">"Remote"</span>
                        </button>
                        <button
                            class="profile-trigger"
                            aria-label="Open Cybou workspace menu"
                            aria-expanded=move || runtime_menu_open.get().to_string()
                            on:click=move |_| set_runtime_menu_open.update(|open| *open = !*open)
                        >"C"</button>
                        <Show when=move || runtime_menu_open.get()>
                            <nav id="runtime-menu" class="runtime-menu" aria-label="Cybou workspace menu">
                                <header><strong>"Cybou"</strong><small>{mind_observed}</small></header>
                                <button on:click=move |_| navigate_from_menu("identity", set_selected, set_runtime_menu_open)><FileCheck size=15 /><span>"Identity"</span></button>
                                <button on:click=move |_| navigate_from_menu("commitments", set_selected, set_runtime_menu_open)><ListChecks size=15 /><span>"Commitments"</span></button>
                                <button on:click=move |_| navigate_from_menu("session", set_selected, set_runtime_menu_open)><UsersRound size=15 /><span>"Session"</span></button>
                                <button on:click=move |_| navigate_from_menu("lifecycle", set_selected, set_runtime_menu_open)><Sparkles size=15 /><span>"Lifecycle"</span></button>
                                <hr />
                                <button on:click=move |_| navigate_from_menu("capabilities", set_selected, set_runtime_menu_open)><Map size=15 /><span>"Capabilities"</span></button>
                                <button
                                    aria-pressed=move || minimap_visible.get().to_string()
                                    on:click=move |_| {
                                        set_minimap_visible.set(!minimap_visible.get_untracked());
                                        set_runtime_menu_open.set(false);
                                    }
                                ><Map size=15 /><span>"Minimap"</span></button>
                                <hr />
                                <button on:click=move |_| navigate_from_menu("journal", set_selected, set_runtime_menu_open)><Sparkles size=15 /><span>"Journal"</span></button>
                            </nav>
                        </Show>
                    </div>
                </header>

                <section
                    id="canvas"
                    class="canvas"
                    aria-label="CYBOU Desktop"
                    on:pointermove=move |event: PointerEvent| {
                        move_drag(event, layout, dragging);
                        move_resize(event, layout, resizing);
                    }
                    on:pointerup=move |_| {
                        finish_drag(layout, dragging);
                        finish_resize(layout, resizing);
                    }
                    on:pointercancel=move |_| {
                        finish_drag(layout, dragging);
                        finish_resize(layout, resizing);
                    }
                >
                    <div class="ambient" aria-hidden="true"></div>
                    <svg class="relationship-layer" aria-label="Desktop relationships">
                        // Each edge is a relationship the system actually has: every organ writes
                        // its contributions into the one Journal, Health1 derives capabilities from
                        // whether those organs answer, and the session only presents the result.
                        <RelationshipEdge layout=layout selected=selected from=CardId::Identity to=CardId::Journal label="writes to" amber=false />
                        <RelationshipEdge layout=layout selected=selected from=CardId::Commitments to=CardId::Journal label="writes to" amber=false />
                        <RelationshipEdge layout=layout selected=selected from=CardId::Lifecycle to=CardId::Journal label="consolidates into" amber=true />
                        <RelationshipEdge layout=layout selected=selected from=CardId::Capabilities to=CardId::Identity label="evaluates" amber=false />
                        <RelationshipEdge layout=layout selected=selected from=CardId::Capabilities to=CardId::Session label="presented under" amber=false />
                    </svg>
                    <div
                        class:selected=move || selected.get() == "identity"
                        class:pinned=move || layout.get().presentation(CardId::Identity).pinned
                        class:collapsed=move || layout.get().presentation(CardId::Identity).collapsed
                        class="object identity"
                        style=move || card_style(layout.get(), CardId::Identity)
                        tabindex="0"
                        role="region"
                        aria-label="Identity card. Drag to reposition; use arrow keys for keyboard movement."
                        on:pointerdown=move |event| start_drag(event, CardId::Identity, layout, dragging)
                        on:keydown=move |event| keyboard_move(event, CardId::Identity, layout)
                        on:click=move |_| set_selected.set("identity")
                    >
                        <header class="card-header">
                            <small class="panel-kicker"><FileCheck size=14 /><span>"Identity1"</span></small>
                            <CardControls card=CardId::Identity layout=layout />
                        </header>
                        <Show
                            when=move || !layout.get().presentation(CardId::Identity).collapsed
                            fallback=move || view! {
                                <div class="card-collapsed-summary">
                                    <b>"Subject continuity"</b>
                                    <span>{identity_id()}</span>
                                </div>
                            }
                        >
                            <strong>"Subject continuity"</strong>
                            <span class="identity-digest">{identity_id}</span>
                            <span class="identity-badges"><i>{identity_sessions}" sessions"</i><i>{identity_age}</i></span>
                            <span class="identity-meta">"Origin "{identity_origin}" · "{identity_architecture}</span>
                        </Show>
                        <CardResizeHandle card=CardId::Identity layout=layout resizing=resizing />
                    </div>

                    <Show when=move || selected.get() == "capabilities">
                        <nav
                            class="object-actions"
                            style=move || selection_actions_style(layout.get())
                            aria-label="Selected release actions"
                        >
                            <button aria-label="Open capability health" on:click=move |_| set_selected.set("capabilities")><FolderOpen size=15 /><span>"Health"</span></button>
                            <button aria-label="Open commitments" on:click=move |_| set_selected.set("commitments")><ListChecks size=15 /><span>"Promises"</span></button>
                            <button aria-label="Open lifecycle" on:click=move |_| set_selected.set("lifecycle")><Sparkles size=15 /><span>"Lifecycle"</span></button>
                            <button aria-label="Open the Journal" on:click=move |_| set_selected.set("journal")><Link size=15 /><span>"Journal"</span></button>
                            <button
                                aria-label="More release actions"
                                on:click=move |_| {
                                    set_command_open.set(true);
                                    if let Some(input) = command_input.get() {
                                        let _ = input.focus();
                                    }
                                }
                            ><Ellipsis size=16 /><span class="sr-only">"More"</span></button>
                        </nav>
                    </Show>

                    <div
                        class:selected=move || selected.get() == "session"
                        class:pinned=move || layout.get().presentation(CardId::Session).pinned
                        class:collapsed=move || layout.get().presentation(CardId::Session).collapsed
                        class="object session"
                        style=move || card_style(layout.get(), CardId::Session)
                        tabindex="0"
                        role="region"
                        aria-label="Session card. Drag to reposition; use arrow keys for keyboard movement."
                        on:pointerdown=move |event| start_drag(event, CardId::Session, layout, dragging)
                        on:keydown=move |event| keyboard_move(event, CardId::Session, layout)
                        on:click=move |_| set_selected.set("session")
                    >
                        <header class="card-header">
                            <small class="panel-kicker"><UsersRound size=14 /><span>"Session"</span></small>
                            <CardControls card=CardId::Session layout=layout />
                        </header>
                        <Show
                            when=move || !layout.get().presentation(CardId::Session).collapsed
                            fallback=move || view! {
                                <div class="card-collapsed-summary">
                                    <b>"Session"</b>
                                    <span>{runtime_label()}</span>
                                </div>
                            }
                        >
                            <strong>"Established trust"</strong>
                            <span class="row"><b>"Mode"</b><i>{runtime_label}</i></span>
                            <span class="row"><b>"Consumer"</b><i>{session_consumer}</i></span>
                            <span class="row"><b>"Authenticated"</b><i>"No"</i></span>
                            <span class="row"><b>"Device bound"</b><i>"No"</i></span>
                            <span class="panel-link">"Established by the gateway, never by this page"</span>
                        </Show>
                        <CardResizeHandle card=CardId::Session layout=layout resizing=resizing />
                    </div>

                    <div
                        class:selected=move || selected.get() == "capabilities"
                        class:pinned=move || layout.get().presentation(CardId::Capabilities).pinned
                        class:collapsed=move || layout.get().presentation(CardId::Capabilities).collapsed
                        class="object capabilities"
                        style=move || card_style(layout.get(), CardId::Capabilities)
                        tabindex="0"
                        role="region"
                        aria-label="Capabilities card. Drag to reposition; use arrow keys for keyboard movement."
                        on:pointerdown=move |event| start_drag(event, CardId::Capabilities, layout, dragging)
                        on:keydown=move |event| keyboard_move(event, CardId::Capabilities, layout)
                        on:click=move |_| set_selected.set("capabilities")
                    >
                        <header class="card-header">
                            <small class="panel-kicker"><Sparkles size=14 /><span>"Health1"</span></small>
                            <CardControls card=CardId::Capabilities layout=layout />
                        </header>
                        <Show
                            when=move || !layout.get().presentation(CardId::Capabilities).collapsed
                            fallback=move || view! {
                                <div class="card-collapsed-summary">
                                    <b>"Capabilities"</b>
                                    <span>{system_label()}</span>
                                </div>
                            }
                        >
                            <h1>{system_label}</h1>
                            <span class="capabilities-kind">"Capability health"</span>
                            <p>"A capability is available only while every organ it depends on answers Health1. Nothing here is composed by this page."</p>
                            <div class="capability-list">
                                <For
                                    each=capabilities
                                    key=|capability| capability.id.clone()
                                    children=move |capability| {
                                        let available = capability.state == cybou_protocol::CapabilityState::Available;
                                        let status = capability_state_label(capability.state);
                                        let reason = capability.reason.unwrap_or_default();
                                        view! {
                                            <span class:available=available class="capability-line">
                                                <span class="status-dot" aria-hidden="true"></span>
                                                <b>{capability.id}</b>
                                                <i>{status}</i>
                                                <small>{reason}</small>
                                            </span>
                                        }
                                    }
                                />
                            </div>
                            <footer class="capabilities-meta">
                                <span><small>"Observed"</small><b>{observed_label}</b></span>
                            </footer>
                        </Show>
                        <CardResizeHandle card=CardId::Capabilities layout=layout resizing=resizing />
                    </div>

                    <div
                        class:selected=move || selected.get() == "journal"
                        class:pinned=move || layout.get().presentation(CardId::Journal).pinned
                        class:collapsed=move || layout.get().presentation(CardId::Journal).collapsed
                        class="object journal"
                        style=move || card_style(layout.get(), CardId::Journal)
                        tabindex="0"
                        role="region"
                        aria-label="Journal card. Drag to reposition; use arrow keys for keyboard movement."
                        on:pointerdown=move |event| start_drag(event, CardId::Journal, layout, dragging)
                        on:keydown=move |event| keyboard_move(event, CardId::Journal, layout)
                        on:click=move |_| set_selected.set("journal")
                    >
                        <header class="card-header">
                            <small class="panel-kicker"><Files size=14 /><span>"Event1"</span></small>
                            <CardControls card=CardId::Journal layout=layout />
                        </header>
                        <Show
                            when=move || !layout.get().presentation(CardId::Journal).collapsed
                            fallback=move || view! {
                                <div class="card-collapsed-summary">
                                    <b>"Journal"</b>
                                    <span>{journal_count()}" entries"</span>
                                </div>
                            }
                        >
                            <strong>"Canonical Journal"</strong>
                            <span class="row"><b>"Contributions"</b><i>{journal_count}</i></span>
                            <span class="row"><b>"Erasure epoch"</b><i>{journal_epoch}</i></span>
                            <span class="row"><b>"Integrity"</b><i>{journal_integrity}</i></span>
                            <div class="journal-feed">
                                <For
                                    each=journal_recent
                                    key=|contribution| contribution.message_id.clone()
                                    children=move |contribution| {
                                        view! {
                                            <span class="journal-line">
                                                <b>{contribution.kind}</b>
                                                <i>{contribution.origin_organ}</i>
                                                <small>{contribution.recorded_at}</small>
                                            </span>
                                        }
                                    }
                                />
                            </div>
                            <span class="journal-footer"><i>{journal_state}</i><b>"Append only"</b></span>
                        </Show>
                        <CardResizeHandle card=CardId::Journal layout=layout resizing=resizing />
                    </div>

                    <article
                        class:selected=move || selected.get() == "lifecycle"
                        class:pinned=move || layout.get().presentation(CardId::Lifecycle).pinned
                        class:collapsed=move || layout.get().presentation(CardId::Lifecycle).collapsed
                        class="object lifecycle"
                        style=move || card_style(layout.get(), CardId::Lifecycle)
                        tabindex="0"
                        role="region"
                        aria-label="Lifecycle card. Drag to reposition; use arrow keys for keyboard movement."
                        on:pointerdown=move |event| start_drag(event, CardId::Lifecycle, layout, dragging)
                        on:keydown=move |event| keyboard_move(event, CardId::Lifecycle, layout)
                        on:click=move |_| set_selected.set("lifecycle")
                    >
                        <header class="card-header">
                            <div class="lifecycle-heading">
                                <small class="panel-kicker"><Sparkles size=14 /><span>"Lifecycle1"</span></small>
                                <b>{lifecycle_mode}</b>
                            </div>
                            <CardControls card=CardId::Lifecycle layout=layout />
                        </header>
                        <Show
                            when=move || !layout.get().presentation(CardId::Lifecycle).collapsed
                            fallback=move || view! {
                                <div class="card-collapsed-summary">
                                    <b>"Lifecycle"</b>
                                    <span>{lifecycle_mode()}</span>
                                </div>
                            }
                        >
                            <strong>"Sleep and wake"</strong>
                            <p>"The mode is the owner's own spelling, not a summary of it. After fifteen idle minutes the system re-verifies its whole chain, and stops the moment someone arrives."</p>
                            <span class="row"><b>"Last user activity"</b><i>{lifecycle_activity}</i></span>
                            <span class="lifecycle-source">{mind_observed}</span>
                        </Show>
                        <CardResizeHandle card=CardId::Lifecycle layout=layout resizing=resizing />
                    </article>

                    <div
                        class:selected=move || selected.get() == "commitments"
                        class:pinned=move || layout.get().presentation(CardId::Commitments).pinned
                        class:collapsed=move || layout.get().presentation(CardId::Commitments).collapsed
                        class="object commitments"
                        style=move || card_style(layout.get(), CardId::Commitments)
                        tabindex="0"
                        role="region"
                        aria-label="Commitments card. Drag to reposition; use arrow keys for keyboard movement."
                        on:pointerdown=move |event| start_drag(event, CardId::Commitments, layout, dragging)
                        on:keydown=move |event| keyboard_move(event, CardId::Commitments, layout)
                        on:click=move |_| set_selected.set("commitments")
                    >
                        <header class="card-header">
                            <small class="panel-kicker"><ListChecks size=14 /><span>{commitments_label}</span></small>
                            <CardControls card=CardId::Commitments layout=layout />
                        </header>
                        <Show
                            when=move || !layout.get().presentation(CardId::Commitments).collapsed
                            fallback=move || view! {
                                <div class="card-collapsed-summary">
                                    <b>"Commitments"</b>
                                    <span>{commitments_label()}</span>
                                </div>
                            }
                        >
                            <For
                                each=commitments
                                key=|commitment| commitment.id.clone()
                                children=move |commitment| {
                                    view! {
                                        <span class="check-row">
                                            <b>{commitment.description}</b>
                                            <i>{commitment.trigger}</i>
                                        </span>
                                    }
                                }
                            />
                            <span class="panel-link">"Intention1 holds these until they are closed"</span>
                        </Show>
                        <CardResizeHandle card=CardId::Commitments layout=layout resizing=resizing />
                    </div>

                    <article
                        class:selected=move || selected.get() == "self"
                        class:pinned=move || layout.get().presentation(CardId::SelfModel).pinned
                        class:collapsed=move || layout.get().presentation(CardId::SelfModel).collapsed
                        class="object self-model"
                        style=move || card_style(layout.get(), CardId::SelfModel)
                        tabindex="0"
                        role="region"
                        aria-label="Self-assessment card. Drag to reposition; use arrow keys for keyboard movement."
                        on:pointerdown=move |event| start_drag(event, CardId::SelfModel, layout, dragging)
                        on:keydown=move |event| keyboard_move(event, CardId::SelfModel, layout)
                        on:click=move |_| set_selected.set("self")
                    >
                        <header class="card-header">
                            <small class="panel-kicker"><Sparkles size=14 /><span>"Self1"</span></small>
                            <CardControls card=CardId::SelfModel layout=layout />
                        </header>
                        <Show
                            when=move || !layout.get().presentation(CardId::SelfModel).collapsed
                            fallback=move || view! {
                                <div class="card-collapsed-summary">
                                    <b>"Self-assessment"</b>
                                    <span>{self_open_intentions()}" open"</span>
                                </div>
                            }
                        >
                            <strong>"Self-assessment"</strong>
                            <p class="self-narration">{self_narration}</p>
                            <span class="row"><b>"Open obligations"</b><i>{self_open_intentions}</i></span>
                            <span class="row"><b>"Settled predictions"</b><i>{self_settled}</i></span>
                            <span class="panel-link">"Composed by Self1, not by this page"</span>
                        </Show>
                        <CardResizeHandle card=CardId::SelfModel layout=layout resizing=resizing />
                    </article>

                    <div
                        class:selected=move || selected.get() == "beliefs"
                        class:pinned=move || layout.get().presentation(CardId::Beliefs).pinned
                        class:collapsed=move || layout.get().presentation(CardId::Beliefs).collapsed
                        class="object beliefs"
                        style=move || card_style(layout.get(), CardId::Beliefs)
                        tabindex="0"
                        role="region"
                        aria-label="Beliefs card. Drag to reposition; use arrow keys for keyboard movement."
                        on:pointerdown=move |event| start_drag(event, CardId::Beliefs, layout, dragging)
                        on:keydown=move |event| keyboard_move(event, CardId::Beliefs, layout)
                        on:click=move |_| set_selected.set("beliefs")
                    >
                        <header class="card-header">
                            <small class="panel-kicker"><Sparkles size=14 /><span>"Epistemic1"</span></small>
                            <CardControls card=CardId::Beliefs layout=layout />
                        </header>
                        <Show
                            when=move || !layout.get().presentation(CardId::Beliefs).collapsed
                            fallback=move || view! {
                                <div class="card-collapsed-summary">
                                    <b>"Beliefs"</b>
                                    <span>{beliefs_label()}</span>
                                </div>
                            }
                        >
                            <strong>{beliefs_label}</strong>
                            <div class="belief-list">
                                <For
                                    each=beliefs
                                    key=|belief| belief.subject.clone()
                                    children=move |belief| {
                                        let observed = belief.status == "observed";
                                        view! {
                                            <span class:observed=observed class="belief-line">
                                                <b>{belief.subject}</b>
                                                <span class="belief-value">{belief.value}</span>
                                                <i>{belief.status}</i>
                                            </span>
                                        }
                                    }
                                />
                            </div>
                            <span class="panel-link">"A belief and its validity are separate facts"</span>
                        </Show>
                        <CardResizeHandle card=CardId::Beliefs layout=layout resizing=resizing />
                    </div>

                    <div
                        class:selected=move || selected.get() == "context"
                        class:pinned=move || layout.get().presentation(CardId::Context).pinned
                        class:collapsed=move || layout.get().presentation(CardId::Context).collapsed
                        class="object context"
                        style=move || card_style(layout.get(), CardId::Context)
                        tabindex="0"
                        role="region"
                        aria-label="Associative context card. Drag to reposition; use arrow keys for keyboard movement."
                        on:pointerdown=move |event| start_drag(event, CardId::Context, layout, dragging)
                        on:keydown=move |event| keyboard_move(event, CardId::Context, layout)
                        on:click=move |_| set_selected.set("context")
                    >
                        <header class="card-header">
                            <small class="panel-kicker"><Link size=14 /><span>"Context1"</span></small>
                            <CardControls card=CardId::Context layout=layout />
                        </header>
                        <Show
                            when=move || !layout.get().presentation(CardId::Context).collapsed
                            fallback=move || view! {
                                <div class="card-collapsed-summary">
                                    <b>"Context"</b>
                                    <span>{context_label()}</span>
                                </div>
                            }
                        >
                            <strong>{context_label}</strong>
                            <div class="concept-list">
                                <For
                                    each=concepts
                                    key=|concept| concept.label.clone()
                                    children=move |concept| {
                                        view! {
                                            <span class="concept-line">
                                                <b>{concept.label}</b>
                                                <i>{format!("{:.2}", concept.salience)}</i>
                                                <small>{concept.activation_reason}</small>
                                            </span>
                                        }
                                    }
                                />
                            </div>
                            <span class="panel-link">"Association is not truth"</span>
                        </Show>
                        <CardResizeHandle card=CardId::Context layout=layout resizing=resizing />
                    </div>

                    <div
                        class:selected=move || selected.get() == "perception"
                        class:pinned=move || layout.get().presentation(CardId::Perception).pinned
                        class:collapsed=move || layout.get().presentation(CardId::Perception).collapsed
                        class="object perception"
                        style=move || card_style(layout.get(), CardId::Perception)
                        tabindex="0"
                        role="region"
                        aria-label="Perception card. Drag to reposition; use arrow keys for keyboard movement."
                        on:pointerdown=move |event| start_drag(event, CardId::Perception, layout, dragging)
                        on:keydown=move |event| keyboard_move(event, CardId::Perception, layout)
                        on:click=move |_| set_selected.set("perception")
                    >
                        <header class="card-header">
                            <small class="panel-kicker"><Files size=14 /><span>"Perception1"</span></small>
                            <CardControls card=CardId::Perception layout=layout />
                        </header>
                        <Show
                            when=move || !layout.get().presentation(CardId::Perception).collapsed
                            fallback=move || view! {
                                <div class="card-collapsed-summary">
                                    <b>"Perception"</b>
                                    <span>{perception_status()}</span>
                                </div>
                            }
                        >
                            <strong>"Host observation"</strong>
                            <span class="row"><b>"Status"</b><i>{perception_status}</i></span>
                            <span class="row"><b>"Source"</b><i>{perception_source}</i></span>
                            <span class="row"><b>"Acquired"</b><i>{perception_at}</i></span>
                        </Show>
                        <CardResizeHandle card=CardId::Perception layout=layout resizing=resizing />
                    </div>

                    <div
                        class:selected=move || selected.get() == "attention"
                        class:pinned=move || layout.get().presentation(CardId::Attention).pinned
                        class:collapsed=move || layout.get().presentation(CardId::Attention).collapsed
                        class="object attention"
                        style=move || card_style(layout.get(), CardId::Attention)
                        tabindex="0"
                        role="region"
                        aria-label="Attention card. Drag to reposition; use arrow keys for keyboard movement."
                        on:pointerdown=move |event| start_drag(event, CardId::Attention, layout, dragging)
                        on:keydown=move |event| keyboard_move(event, CardId::Attention, layout)
                        on:click=move |_| set_selected.set("attention")
                    >
                        <header class="card-header">
                            <small class="panel-kicker"><Map size=14 /><span>"Workspace1"</span></small>
                            <CardControls card=CardId::Attention layout=layout />
                        </header>
                        <Show
                            when=move || !layout.get().presentation(CardId::Attention).collapsed
                            fallback=move || view! {
                                <div class="card-collapsed-summary">
                                    <b>"Attention"</b>
                                    <span>{attention_focus()}</span>
                                </div>
                            }
                        >
                            <strong>"Attention"</strong>
                            <span class="attention-focus">{attention_focus}</span>
                            <span class="row"><b>"Salience"</b><i>{attention_salience}</i></span>
                            <span class="row"><b>"Organs"</b><i>{attention_organs}</i></span>
                        </Show>
                        <CardResizeHandle card=CardId::Attention layout=layout resizing=resizing />
                    </div>

                    <Show when=move || command_open.get()>
                        <nav class="command-palette" aria-label="Desktop commands">
                            <small>"Jump to"</small>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "capabilities health")
                                on:click=move |_| select_from_command("capabilities", set_selected, set_command_open, set_command_query)
    ><Sparkles size=15 /><span><b>"Capabilities"</b><i>"Health1"</i></span></button>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "identity subject continuity")
                                on:click=move |_| select_from_command("identity", set_selected, set_command_open, set_command_query)
    ><FileCheck size=15 /><span><b>"Identity"</b><i>"Identity1"</i></span></button>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "session trust mode")
                                on:click=move |_| select_from_command("session", set_selected, set_command_open, set_command_query)
    ><UsersRound size=15 /><span><b>"Session"</b><i>"Established trust"</i></span></button>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "journal contributions event1")
                                on:click=move |_| select_from_command("journal", set_selected, set_command_open, set_command_query)
    ><Files size=15 /><span><b>"Journal"</b><i>"Event1"</i></span></button>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "lifecycle sleep wake")
                                on:click=move |_| select_from_command("lifecycle", set_selected, set_command_open, set_command_query)
    ><Sparkles size=15 /><span><b>"Lifecycle"</b><i>"Sleep and wake"</i></span></button>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "commitments obligations intention1")
                                on:click=move |_| select_from_command("commitments", set_selected, set_command_open, set_command_query)
    ><ListChecks size=15 /><span><b>"Commitments"</b><i>"Intention1"</i></span></button>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "self assessment narration self1")
                                on:click=move |_| select_from_command("self", set_selected, set_command_open, set_command_query)
                            ><Sparkles size=15 /><span><b>"Self-assessment"</b><i>"Self1"</i></span></button>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "attention focus workspace1")
                                on:click=move |_| select_from_command("attention", set_selected, set_command_open, set_command_query)
                            ><Map size=15 /><span><b>"Attention"</b><i>"Workspace1"</i></span></button>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "beliefs epistemic1 validity")
                                on:click=move |_| select_from_command("beliefs", set_selected, set_command_open, set_command_query)
                            ><Sparkles size=15 /><span><b>"Beliefs"</b><i>"Epistemic1"</i></span></button>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "perception host observation")
                                on:click=move |_| select_from_command("perception", set_selected, set_command_open, set_command_query)
                            ><Files size=15 /><span><b>"Perception"</b><i>"Perception1"</i></span></button>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "context association concepts context1")
                                on:click=move |_| select_from_command("context", set_selected, set_command_open, set_command_query)
                            ><Link size=15 /><span><b>"Context"</b><i>"Context1"</i></span></button>
                        </nav>
                    </Show>

                    <label class:open=move || command_open.get() class="command-bar" aria-label="Search or act">
                        <Search size=19 />
                        <input
                            node_ref=command_input
                            type="search"
                            placeholder="Search or act…"
                            prop:value=move || command_query.get()
                            on:focus=move |_| set_command_open.set(true)
                            on:input=move |event| set_command_query.set(event_target_value(&event))
                            on:keydown=move |event: KeyboardEvent| {
                                if event.key() == "Enter"
                                    && let Some(panel) = first_command_match(&command_query.get())
                                {
                                    event.prevent_default();
                                    select_from_command(
                                        panel,
                                        set_selected,
                                        set_command_open,
                                        set_command_query,
                                    );
                                }
                            }
                        />
                        <kbd>"Ctrl K"</kbd>
                    </label>

                    <Show when=move || minimap_visible.get()>
                        <nav class="minimap" aria-label="Desktop map">
                            <header><Map size=15 /><strong>"Desktop map"</strong></header>
                            <div class="minimap-field">
                                <button
                                    class:selected=move || selected.get() == "identity"
                                    class="mini-node identity-node"
                                    style=move || minimap_style(layout.get().geometry(CardId::Identity))
                                    aria-label="Select identity card"
                                    on:click=move |_| set_selected.set("identity")
                                ></button>
                                <button
                                    class:selected=move || selected.get() == "session"
                                    class="mini-node session-node"
                                    style=move || minimap_style(layout.get().geometry(CardId::Session))
                                    aria-label="Select session card"
                                    on:click=move |_| set_selected.set("session")
                                ></button>
                                <button
                                    class:selected=move || selected.get() == "capabilities"
                                    class="mini-node capabilities-node"
                                    style=move || minimap_style(layout.get().geometry(CardId::Capabilities))
                                    aria-label="Select capabilities card"
                                    on:click=move |_| set_selected.set("capabilities")
                                ></button>
                                <button
                                    class:selected=move || selected.get() == "journal"
                                    class="mini-node journal-node"
                                    style=move || minimap_style(layout.get().geometry(CardId::Journal))
                                    aria-label="Select journal card"
                                    on:click=move |_| set_selected.set("journal")
                                ></button>
                                <button
                                    class:selected=move || selected.get() == "lifecycle"
                                    class="mini-node lifecycle-node"
                                    style=move || minimap_style(layout.get().geometry(CardId::Lifecycle))
                                    aria-label="Select lifecycle card"
                                    on:click=move |_| set_selected.set("lifecycle")
                                ></button>
                                <button
                                    class:selected=move || selected.get() == "commitments"
                                    class="mini-node commitments-node"
                                    style=move || minimap_style(layout.get().geometry(CardId::Commitments))
                                    aria-label="Select commitments card"
                                    on:click=move |_| set_selected.set("commitments")
                                ></button>
                                <button
                                    class:selected=move || selected.get() == "self"
                                    class="mini-node self-node"
                                    style=move || minimap_style(layout.get().geometry(CardId::SelfModel))
                                    aria-label="Select self-assessment card"
                                    on:click=move |_| set_selected.set("self")
                                ></button>
                                <button
                                    class:selected=move || selected.get() == "attention"
                                    class="mini-node attention-node"
                                    style=move || minimap_style(layout.get().geometry(CardId::Attention))
                                    aria-label="Select attention card"
                                    on:click=move |_| set_selected.set("attention")
                                ></button>
                                <button
                                    class:selected=move || selected.get() == "beliefs"
                                    class="mini-node beliefs-node"
                                    style=move || minimap_style(layout.get().geometry(CardId::Beliefs))
                                    aria-label="Select beliefs card"
                                    on:click=move |_| set_selected.set("beliefs")
                                ></button>
                                <button
                                    class:selected=move || selected.get() == "perception"
                                    class="mini-node perception-node"
                                    style=move || minimap_style(layout.get().geometry(CardId::Perception))
                                    aria-label="Select perception card"
                                    on:click=move |_| set_selected.set("perception")
                                ></button>
                                <button
                                    class:selected=move || selected.get() == "context"
                                    class="mini-node context-node"
                                    style=move || minimap_style(layout.get().geometry(CardId::Context))
                                    aria-label="Select associative context card"
                                    on:click=move |_| set_selected.set("context")
                                ></button>
                            </div>
                        </nav>
                    </Show>

                    <Show when=move || capabilities_open.get()>
                        <aside id="capability-inspector" class="capability-inspector" aria-label="Gateway capabilities">
                            <header>
                                <span><strong>"Gateway capabilities"</strong><small>{observed_label}</small></span>
                                <button aria-label="Close capability inspector" on:click=move |_| set_capabilities_open.set(false)>"×"</button>
                            </header>
                            <For
                                each=capabilities
                                key=|capability| capability.id.clone()
                                children=move |capability| {
                                    let available = capability.state == cybou_protocol::CapabilityState::Available;
                                    let status = capability_state_label(capability.state);
                                    let context = capability.reason.unwrap_or_else(|| {
                                        format!(
                                            "{} · {}",
                                            knowledge_label(capability.knowledge),
                                            freshness_label(capability.freshness),
                                        )
                                    });
                                    view! {
                                        <div class:available=available class="capability-row">
                                            <span class="status-dot" aria-hidden="true"></span>
                                            <span><b>{capability.id}</b><small>{context}</small></span>
                                            <i>{status}</i>
                                        </div>
                                    }
                                }
                            />
                        </aside>
                    </Show>

                    <button
                        class:open=move || capabilities_open.get()
                        class="system-state"
                        aria-label="Open gateway capability inspector"
                        aria-expanded=move || capabilities_open.get().to_string()
                        aria-controls="capability-inspector"
                        on:click=move |_| set_capabilities_open.update(|open| *open = !*open)
                    >
                        <span class="status-dot" aria-hidden="true"></span>
                        {system_label}
                    </button>
                </section>
            </main>
        }
}

fn card_style(layout: DesktopLayout, card: CardId) -> String {
    let geom = layout.geometry(card);
    let pres = layout.presentation(card);
    if pres.collapsed {
        format!(
            "left:{:.1}px;top:{:.1}px;width:{:.1}px;z-index:{}",
            geom.x, geom.y, geom.width, geom.z
        )
    } else {
        format!(
            "left:{:.1}px;top:{:.1}px;width:{:.1}px;height:{:.1}px;z-index:{}",
            geom.x, geom.y, geom.width, geom.height, geom.z
        )
    }
}

/// What a field reads when the owner behind it was not read.
///
/// A dash says nothing was read. A zero would say the owner answered and held nothing, which is a
/// different claim and not one the gateway made.
fn unread() -> String {
    "—".to_owned()
}

const fn capability_state_label(state: cybou_protocol::CapabilityState) -> &'static str {
    match state {
        cybou_protocol::CapabilityState::Available => "Available",
        cybou_protocol::CapabilityState::Unavailable => "Unavailable",
        cybou_protocol::CapabilityState::Unknown => "Unknown",
    }
}

const fn knowledge_label(state: cybou_protocol::KnowledgeState) -> &'static str {
    match state {
        cybou_protocol::KnowledgeState::Known => "Known",
        cybou_protocol::KnowledgeState::Unknown => "Unknown",
    }
}

const fn freshness_label(state: cybou_web_contracts::Freshness) -> &'static str {
    match state {
        cybou_web_contracts::Freshness::Current => "Current",
        cybou_web_contracts::Freshness::Stale => "Stale",
        cybou_web_contracts::Freshness::Unknown => "Unknown freshness",
    }
}

fn command_matches(query: &str, haystack: &str) -> bool {
    let query = query.trim().to_ascii_lowercase();
    query.is_empty() || haystack.contains(&query)
}

fn first_command_match(query: &str) -> Option<&'static str> {
    [
        ("capabilities", "capabilities health"),
        ("identity", "identity subject continuity"),
        ("session", "session trust mode"),
        ("journal", "journal contributions event1"),
        ("lifecycle", "lifecycle sleep wake"),
        ("commitments", "commitments obligations intention1"),
        ("self", "self assessment narration self1"),
        ("attention", "attention focus workspace1"),
        ("beliefs", "beliefs epistemic1 validity"),
        ("perception", "perception host observation"),
        ("context", "context association concepts context1"),
    ]
    .into_iter()
    .find_map(|(panel, label)| command_matches(query, label).then_some(panel))
}

fn select_from_command(
    panel: &'static str,
    set_selected: WriteSignal<&'static str>,
    set_command_open: WriteSignal<bool>,
    set_command_query: WriteSignal<String>,
) {
    set_selected.set(panel);
    set_command_open.set(false);
    set_command_query.set(String::new());
}

fn navigate_from_menu(
    panel: &'static str,
    set_selected: WriteSignal<&'static str>,
    set_runtime_menu_open: WriteSignal<bool>,
) {
    set_selected.set(panel);
    set_runtime_menu_open.set(false);
}

fn selection_actions_style(layout: DesktopLayout) -> String {
    let geom = layout.geometry(CardId::Capabilities);
    format!(
        "left:{:.1}px;top:{:.1}px;z-index:{}",
        geom.x + 18.0,
        geom.y + geom.height,
        geom.z + 1
    )
}

fn minimap_style(geom: CardGeometry) -> String {
    let x = 10.0 + geom.x / 1_280.0 * 180.0;
    let y = 8.0 + geom.y / 650.0 * 92.0;
    format!("left:{x:.1}px;top:{y:.1}px")
}

fn relationship_points(
    layout: DesktopLayout,
    from: CardId,
    to: CardId,
) -> (f64, f64, f64, f64, f64, f64) {
    let from_geom = layout.geometry(from);
    let to_geom = layout.geometry(to);
    let from_pres = layout.presentation(from);
    let to_pres = layout.presentation(to);

    let from_height = if from_pres.collapsed {
        44.0
    } else {
        from_geom.height
    };
    let to_height = if to_pres.collapsed {
        44.0
    } else {
        to_geom.height
    };

    let from_size = (from_geom.width, from_height);
    let to_size = (to_geom.width, to_height);
    let from_center = (
        from_geom.x + from_size.0 / 2.0,
        from_geom.y + from_size.1 / 2.0,
    );
    let to_center = (to_geom.x + to_size.0 / 2.0, to_geom.y + to_size.1 / 2.0);

    let (x1, y1) = edge_anchor(from_center, from_size, to_center);
    let (x2, y2) = edge_anchor(to_center, to_size, from_center);
    (x1, y1, x2, y2, (x1 + x2) / 2.0, (y1 + y2) / 2.0 - 7.0)
}

fn edge_anchor(center: (f64, f64), size: (f64, f64), target: (f64, f64)) -> (f64, f64) {
    let dx = target.0 - center.0;
    let dy = target.1 - center.1;
    if dx.abs() < f64::EPSILON && dy.abs() < f64::EPSILON {
        return center;
    }
    let x_scale = if dx.abs() < f64::EPSILON {
        f64::INFINITY
    } else {
        size.0 / 2.0 / dx.abs()
    };
    let y_scale = if dy.abs() < f64::EPSILON {
        f64::INFINITY
    } else {
        size.1 / 2.0 / dy.abs()
    };
    let scale = x_scale.min(y_scale);
    (center.0 + dx * scale, center.1 + dy * scale)
}

fn start_drag(
    event: PointerEvent,
    card: CardId,
    layout: RwSignal<DesktopLayout>,
    dragging: RwSignal<Option<DragState>>,
) {
    if event.button() != 0 {
        return;
    }
    if layout.get_untracked().presentation(card).pinned {
        return;
    }
    let Some(target) = event
        .current_target()
        .and_then(|target| target.dyn_into::<HtmlElement>().ok())
    else {
        return;
    };
    let _ = target.focus();
    let _ = target.set_pointer_capture(event.pointer_id());
    let rect = target.get_bounding_client_rect();
    layout.update(|current| current.bring_forward(card));
    dragging.set(Some(DragState {
        card,
        offset_x: f64::from(event.client_x()) - rect.left(),
        offset_y: f64::from(event.client_y()) - rect.top(),
        width: rect.width(),
        height: rect.height(),
    }));
    event.prevent_default();
}

fn move_drag(
    event: PointerEvent,
    layout: RwSignal<DesktopLayout>,
    dragging: RwSignal<Option<DragState>>,
) {
    let Some(drag) = dragging.get_untracked() else {
        return;
    };
    let Some(surface) = event
        .current_target()
        .and_then(|target| target.dyn_into::<HtmlElement>().ok())
    else {
        return;
    };
    let bounds = surface.get_bounding_client_rect();
    let x = (f64::from(event.client_x()) - bounds.left() - drag.offset_x)
        .clamp(12.0, (bounds.width() - drag.width - 12.0).max(12.0));
    let y = (f64::from(event.client_y()) - bounds.top() - drag.offset_y)
        .clamp(12.0, (bounds.height() - drag.height - 12.0).max(12.0));
    layout.update(|current| {
        current.set_position(drag.card, x, y);
    });
}

fn finish_drag(layout: RwSignal<DesktopLayout>, dragging: RwSignal<Option<DragState>>) {
    if dragging.get_untracked().is_some() {
        dragging.set(None);
        layout.get_untracked().save();
    }
}

fn start_resize(
    event: PointerEvent,
    card: CardId,
    layout: RwSignal<DesktopLayout>,
    resizing: RwSignal<Option<ResizeState>>,
) {
    if event.button() != 0 {
        return;
    }
    if layout.get_untracked().presentation(card).pinned {
        return;
    }
    let Some(target) = event
        .current_target()
        .and_then(|target| target.dyn_into::<HtmlElement>().ok())
    else {
        return;
    };
    event.stop_propagation();
    event.prevent_default();
    let _ = target.set_pointer_capture(event.pointer_id());
    layout.update(|current| current.bring_forward(card));
    let geom = layout.get_untracked().geometry(card);
    resizing.set(Some(ResizeState {
        card,
        start_pointer_x: f64::from(event.client_x()),
        start_pointer_y: f64::from(event.client_y()),
        start_width: geom.width,
        start_height: geom.height,
    }));
}

fn move_resize(
    event: PointerEvent,
    layout: RwSignal<DesktopLayout>,
    resizing: RwSignal<Option<ResizeState>>,
) {
    let Some(resize) = resizing.get_untracked() else {
        return;
    };
    let dx = f64::from(event.client_x()) - resize.start_pointer_x;
    let dy = f64::from(event.client_y()) - resize.start_pointer_y;
    let spec = resize.card.spec();
    let new_width = (resize.start_width + dx).clamp(spec.min_size.0, spec.max_size.0);
    let new_height = (resize.start_height + dy).clamp(spec.min_size.1, spec.max_size.1);

    layout.update(|current| {
        current.set_size(resize.card, new_width, new_height);
    });
}

fn finish_resize(layout: RwSignal<DesktopLayout>, resizing: RwSignal<Option<ResizeState>>) {
    if resizing.get_untracked().is_some() {
        resizing.set(None);
        layout.get_untracked().save();
    }
}

fn keyboard_move(event: KeyboardEvent, card: CardId, layout: RwSignal<DesktopLayout>) {
    if layout.get_untracked().presentation(card).pinned {
        return;
    }
    let step = if event.shift_key() { 40.0 } else { 10.0 };
    let (dx, dy) = match event.key().as_str() {
        "ArrowLeft" => (-step, 0.0),
        "ArrowRight" => (step, 0.0),
        "ArrowUp" => (0.0, -step),
        "ArrowDown" => (0.0, step),
        _ => return,
    };
    event.prevent_default();
    layout.update(|current| {
        current.bring_forward(card);
        let geom = current.geometry(card);
        current.set_position(card, (geom.x + dx).max(12.0), (geom.y + dy).max(12.0));
    });
    layout.get_untracked().save();
}

fn load_layout() -> DesktopLayout {
    DesktopLayout::load()
}
