// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

use leptos::prelude::*;
use leptos::task::spawn_local;
use living_canvas::{GatewayMindClient, MindClient};
use lucide_leptos::{
    Ellipsis, FileCheck, Files, FolderOpen, Link, ListChecks, Map, Search, Sparkles, UsersRound,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{EventSource, HtmlElement, KeyboardEvent, MessageEvent, PointerEvent};

const LAYOUT_KEY: &str = "cybou.living-canvas.layout.v6";

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct Point {
    x: f64,
    y: f64,
    z: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct CanvasLayout {
    identity: Point,
    session: Point,
    capabilities: Point,
    journal: Point,
    lifecycle: Point,
    commitments: Point,
    self_model: Point,
    attention: Point,
}

impl Default for CanvasLayout {
    fn default() -> Self {
        Self {
            identity: Point {
                x: 70.0,
                y: 50.0,
                z: 1,
            },
            session: Point {
                x: 55.0,
                y: 300.0,
                z: 2,
            },
            capabilities: Point {
                x: 445.0,
                y: 70.0,
                z: 6,
            },
            journal: Point {
                x: 880.0,
                y: 50.0,
                z: 3,
            },
            lifecycle: Point {
                x: 900.0,
                y: 340.0,
                z: 5,
            },
            commitments: Point {
                x: 470.0,
                y: 410.0,
                z: 4,
            },
            self_model: Point {
                x: 55.0,
                y: 600.0,
                z: 7,
            },
            attention: Point {
                x: 470.0,
                y: 620.0,
                z: 8,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Panel {
    Identity,
    Session,
    Capabilities,
    Journal,
    Lifecycle,
    Commitments,
    SelfModel,
    Attention,
}

impl Panel {
    const fn key(self) -> &'static str {
        match self {
            Self::Identity => "identity",
            Self::Session => "session",
            Self::Capabilities => "capabilities",
            Self::Journal => "journal",
            Self::Lifecycle => "lifecycle",
            Self::Commitments => "commitments",
            Self::SelfModel => "self",
            Self::Attention => "attention",
        }
    }

    const fn size(self) -> (f64, f64) {
        match self {
            Self::Identity => (220.0, 188.0),
            Self::Session => (240.0, 236.0),
            Self::Capabilities => (390.0, 294.0),
            Self::Journal => (300.0, 285.0),
            Self::Lifecycle => (335.0, 252.0),
            Self::Commitments => (310.0, 184.0),
            Self::SelfModel => (330.0, 210.0),
            Self::Attention => (320.0, 170.0),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct DragState {
    panel: Panel,
    offset_x: f64,
    offset_y: f64,
    width: f64,
    height: f64,
}

impl CanvasLayout {
    const fn point(self, panel: Panel) -> Point {
        match panel {
            Panel::Identity => self.identity,
            Panel::Session => self.session,
            Panel::Capabilities => self.capabilities,
            Panel::Journal => self.journal,
            Panel::Lifecycle => self.lifecycle,
            Panel::Commitments => self.commitments,
            Panel::SelfModel => self.self_model,
            Panel::Attention => self.attention,
        }
    }

    fn set_point(&mut self, panel: Panel, point: Point) {
        match panel {
            Panel::Identity => self.identity = point,
            Panel::Session => self.session = point,
            Panel::Capabilities => self.capabilities = point,
            Panel::Journal => self.journal = point,
            Panel::Lifecycle => self.lifecycle = point,
            Panel::Commitments => self.commitments = point,
            Panel::SelfModel => self.self_model = point,
            Panel::Attention => self.attention = point,
        }
    }

    fn bring_forward(&mut self, panel: Panel) {
        let next = [
            self.identity.z,
            self.session.z,
            self.capabilities.z,
            self.journal.z,
            self.lifecycle.z,
            self.commitments.z,
            self.self_model.z,
            self.attention.z,
        ]
        .into_iter()
        .max()
        .unwrap_or_default()
            + 1;
        self.set_point(
            panel,
            Point {
                z: next,
                ..self.point(panel)
            },
        );
    }
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
fn RelationshipEdge(
    layout: RwSignal<CanvasLayout>,
    selected: ReadSignal<&'static str>,
    from: Panel,
    to: Panel,
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
                    aria-label="Cybou living canvas"
                    on:pointermove=move |event: PointerEvent| move_drag(event, layout, dragging)
                    on:pointerup=move |_| finish_drag(layout, dragging)
                    on:pointercancel=move |_| finish_drag(layout, dragging)
                >
                    <div class="ambient" aria-hidden="true"></div>
                    <svg class="relationship-layer" aria-label="Canvas relationships">
                        // Each edge is a relationship the system actually has: every organ writes
                        // its contributions into the one Journal, Health1 derives capabilities from
                        // whether those organs answer, and the session only presents the result.
                        <RelationshipEdge layout=layout selected=selected from=Panel::Identity to=Panel::Journal label="writes to" amber=false />
                        <RelationshipEdge layout=layout selected=selected from=Panel::Commitments to=Panel::Journal label="writes to" amber=false />
                        <RelationshipEdge layout=layout selected=selected from=Panel::Lifecycle to=Panel::Journal label="consolidates into" amber=true />
                        <RelationshipEdge layout=layout selected=selected from=Panel::Capabilities to=Panel::Identity label="evaluates" amber=false />
                        <RelationshipEdge layout=layout selected=selected from=Panel::Capabilities to=Panel::Session label="presented under" amber=false />
                    </svg>
                    <button
                        class:selected=move || selected.get() == "identity"
                        class="object identity"
                        style=move || panel_style(layout.get(), Panel::Identity)
                        aria-label="Artifact panel. Drag to reposition; use arrow keys for keyboard movement."
                        on:pointerdown=move |event| start_drag(event, Panel::Identity, layout, dragging)
                        on:keydown=move |event| keyboard_move(event, Panel::Identity, layout)
                        on:click=move |_| set_selected.set("identity")
                    >
                        <small class="panel-kicker"><FileCheck size=14 /><span>"Identity1"</span></small>
                        <strong>"Subject continuity"</strong>
                        <span class="identity-digest">{identity_id}</span>
                        <span class="identity-badges"><i>{identity_sessions}" sessions"</i><i>{identity_age}</i></span>
                        <span class="identity-meta">"Origin "{identity_origin}" · "{identity_architecture}</span>
                    </button>

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

                    <button
                        class:selected=move || selected.get() == "session"
                        class="object session"
                        style=move || panel_style(layout.get(), Panel::Session)
                        aria-label="Collaborators panel. Drag to reposition; use arrow keys for keyboard movement."
                        on:pointerdown=move |event| start_drag(event, Panel::Session, layout, dragging)
                        on:keydown=move |event| keyboard_move(event, Panel::Session, layout)
                        on:click=move |_| set_selected.set("session")
                    >
                        <small class="panel-kicker"><UsersRound size=14 /><span>"Session"</span></small>
                        <strong>"Established trust"</strong>
                        <span class="row"><b>"Mode"</b><i>{runtime_label}</i></span>
                        <span class="row"><b>"Consumer"</b><i>{session_consumer}</i></span>
                        <span class="row"><b>"Authenticated"</b><i>"No"</i></span>
                        <span class="row"><b>"Device bound"</b><i>"No"</i></span>
                        <span class="panel-link">"Established by the gateway, never by this page"</span>
                    </button>

                    <button
                        class:selected=move || selected.get() == "capabilities"
                        class="object capabilities"
                        style=move || panel_style(layout.get(), Panel::Capabilities)
                        aria-label="Release plan panel. Drag to reposition; use arrow keys for keyboard movement."
                        on:pointerdown=move |event| start_drag(event, Panel::Capabilities, layout, dragging)
                        on:keydown=move |event| keyboard_move(event, Panel::Capabilities, layout)
                        on:click=move |_| set_selected.set("capabilities")
                    >
                        <small class="panel-kicker"><Sparkles size=14 /><span>"Health1"</span></small>
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
                    </button>

                    <button
                        class:selected=move || selected.get() == "journal"
                        class="object journal"
                        style=move || panel_style(layout.get(), Panel::Journal)
                        aria-label="Sources panel. Drag to reposition; use arrow keys for keyboard movement."
                        on:pointerdown=move |event| start_drag(event, Panel::Journal, layout, dragging)
                        on:keydown=move |event| keyboard_move(event, Panel::Journal, layout)
                        on:click=move |_| set_selected.set("journal")
                    >
                        <small class="panel-kicker"><Files size=14 /><span>"Event1"</span></small>
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
                    </button>

                    <article
                        class:selected=move || selected.get() == "lifecycle"
                        class="object lifecycle"
                        style=move || panel_style(layout.get(), Panel::Lifecycle)
                        tabindex="0"
                        aria-label="Lifecycle panel. Drag to reposition; use arrow keys for keyboard movement."
                        on:pointerdown=move |event| start_drag(event, Panel::Lifecycle, layout, dragging)
                        on:keydown=move |event| keyboard_move(event, Panel::Lifecycle, layout)
                        on:click=move |_| set_selected.set("lifecycle")
                    >
                        <header class="lifecycle-heading">
                            <small class="panel-kicker"><Sparkles size=14 /><span>"Lifecycle1"</span></small>
                            <b>{lifecycle_mode}</b>
                        </header>
                        <strong>"Sleep and wake"</strong>
                        <p>"Consolidation runs while nobody is present. The mode is the owner's own spelling, not a summary of it."</p>
                        <span class="row"><b>"Last user activity"</b><i>{lifecycle_activity}</i></span>
                        <span class="lifecycle-source">{mind_observed}</span>
                    </article>

                    <button
                        class:selected=move || selected.get() == "commitments"
                        class="object commitments"
                        style=move || panel_style(layout.get(), Panel::Commitments)
                        aria-label="Commitments panel. Drag to reposition; use arrow keys for keyboard movement."
                        on:pointerdown=move |event| start_drag(event, Panel::Commitments, layout, dragging)
                        on:keydown=move |event| keyboard_move(event, Panel::Commitments, layout)
                        on:click=move |_| set_selected.set("commitments")
                    >
                        <small class="panel-kicker"><ListChecks size=14 /><span>{commitments_label}</span></small>
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
                    </button>

                    <article
                        class:selected=move || selected.get() == "self"
                        class="object self-model"
                        style=move || panel_style(layout.get(), Panel::SelfModel)
                        tabindex="0"
                        aria-label="Self-assessment panel. Drag to reposition; use arrow keys for keyboard movement."
                        on:pointerdown=move |event| start_drag(event, Panel::SelfModel, layout, dragging)
                        on:keydown=move |event| keyboard_move(event, Panel::SelfModel, layout)
                        on:click=move |_| set_selected.set("self")
                    >
                        <small class="panel-kicker"><Sparkles size=14 /><span>"Self1"</span></small>
                        <strong>"Self-assessment"</strong>
                        <p class="self-narration">{self_narration}</p>
                        <span class="row"><b>"Open obligations"</b><i>{self_open_intentions}</i></span>
                        <span class="row"><b>"Settled predictions"</b><i>{self_settled}</i></span>
                        <span class="panel-link">"Composed by Self1, not by this page"</span>
                    </article>

                    <button
                        class:selected=move || selected.get() == "attention"
                        class="object attention"
                        style=move || panel_style(layout.get(), Panel::Attention)
                        aria-label="Attention panel. Drag to reposition; use arrow keys for keyboard movement."
                        on:pointerdown=move |event| start_drag(event, Panel::Attention, layout, dragging)
                        on:keydown=move |event| keyboard_move(event, Panel::Attention, layout)
                        on:click=move |_| set_selected.set("attention")
                    >
                        <small class="panel-kicker"><Map size=14 /><span>"Workspace1"</span></small>
                        <strong>"Attention"</strong>
                        <span class="attention-focus">{attention_focus}</span>
                        <span class="row"><b>"Salience"</b><i>{attention_salience}</i></span>
                        <span class="row"><b>"Organs"</b><i>{attention_organs}</i></span>
                    </button>

                    <Show when=move || command_open.get()>
                        <nav class="command-palette" aria-label="Canvas commands">
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
                        <nav class="minimap" aria-label="Canvas minimap">
                            <header><Map size=15 /><strong>"Canvas map"</strong></header>
                            <div class="minimap-field">
                                <button
                                    class:selected=move || selected.get() == "identity"
                                    class="mini-node identity-node"
                                    style=move || minimap_style(layout.get().identity)
                                    aria-label="Select artifact panel"
                                    on:click=move |_| set_selected.set("identity")
                                ></button>
                                <button
                                    class:selected=move || selected.get() == "session"
                                    class="mini-node session-node"
                                    style=move || minimap_style(layout.get().session)
                                    aria-label="Select collaborators panel"
                                    on:click=move |_| set_selected.set("session")
                                ></button>
                                <button
                                    class:selected=move || selected.get() == "capabilities"
                                    class="mini-node capabilities-node"
                                    style=move || minimap_style(layout.get().capabilities)
                                    aria-label="Select release panel"
                                    on:click=move |_| set_selected.set("capabilities")
                                ></button>
                                <button
                                    class:selected=move || selected.get() == "journal"
                                    class="mini-node journal-node"
                                    style=move || minimap_style(layout.get().journal)
                                    aria-label="Select sources panel"
                                    on:click=move |_| set_selected.set("journal")
                                ></button>
                                <button
                                    class:selected=move || selected.get() == "lifecycle"
                                    class="mini-node lifecycle-node"
                                    style=move || minimap_style(layout.get().lifecycle)
                                    aria-label="Select mind suggestion panel"
                                    on:click=move |_| set_selected.set("lifecycle")
                                ></button>
                                <button
                                    class:selected=move || selected.get() == "commitments"
                                    class="mini-node commitments-node"
                                    style=move || minimap_style(layout.get().commitments)
                                    aria-label="Select commitments panel"
                                    on:click=move |_| set_selected.set("commitments")
                                ></button>
                                <button
                                    class:selected=move || selected.get() == "self"
                                    class="mini-node self-node"
                                    style=move || minimap_style(layout.get().self_model)
                                    aria-label="Select self-assessment panel"
                                    on:click=move |_| set_selected.set("self")
                                ></button>
                                <button
                                    class:selected=move || selected.get() == "attention"
                                    class="mini-node attention-node"
                                    style=move || minimap_style(layout.get().attention)
                                    aria-label="Select attention panel"
                                    on:click=move |_| set_selected.set("attention")
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

fn panel_style(layout: CanvasLayout, panel: Panel) -> String {
    let point = layout.point(panel);
    format!(
        "left:{:.1}px;top:{:.1}px;z-index:{}",
        point.x, point.y, point.z
    )
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

fn selection_actions_style(layout: CanvasLayout) -> String {
    let point = layout.capabilities;
    format!(
        "left:{:.1}px;top:{:.1}px;z-index:{}",
        point.x + 18.0,
        point.y + 294.0,
        point.z + 1
    )
}

fn minimap_style(point: Point) -> String {
    let x = 10.0 + point.x / 1_280.0 * 180.0;
    let y = 8.0 + point.y / 650.0 * 92.0;
    format!("left:{x:.1}px;top:{y:.1}px")
}

fn relationship_points(
    layout: CanvasLayout,
    from: Panel,
    to: Panel,
) -> (f64, f64, f64, f64, f64, f64) {
    let from_point = layout.point(from);
    let to_point = layout.point(to);
    let from_size = from.size();
    let to_size = to.size();
    let from_center = (
        from_point.x + from_size.0 / 2.0,
        from_point.y + from_size.1 / 2.0,
    );
    let to_center = (to_point.x + to_size.0 / 2.0, to_point.y + to_size.1 / 2.0);
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
    panel: Panel,
    layout: RwSignal<CanvasLayout>,
    dragging: RwSignal<Option<DragState>>,
) {
    if event.button() != 0 {
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
    layout.update(|current| current.bring_forward(panel));
    dragging.set(Some(DragState {
        panel,
        offset_x: f64::from(event.client_x()) - rect.left(),
        offset_y: f64::from(event.client_y()) - rect.top(),
        width: rect.width(),
        height: rect.height(),
    }));
    event.prevent_default();
}

fn move_drag(
    event: PointerEvent,
    layout: RwSignal<CanvasLayout>,
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
        current.set_point(
            drag.panel,
            Point {
                x,
                y,
                ..current.point(drag.panel)
            },
        );
    });
}

fn finish_drag(layout: RwSignal<CanvasLayout>, dragging: RwSignal<Option<DragState>>) {
    if dragging.get_untracked().is_some() {
        dragging.set(None);
        save_layout(layout.get_untracked());
    }
}

fn keyboard_move(event: KeyboardEvent, panel: Panel, layout: RwSignal<CanvasLayout>) {
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
        current.bring_forward(panel);
        let point = current.point(panel);
        current.set_point(
            panel,
            Point {
                x: (point.x + dx).max(12.0),
                y: (point.y + dy).max(12.0),
                ..point
            },
        );
    });
    save_layout(layout.get_untracked());
}

fn load_layout() -> CanvasLayout {
    web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item(LAYOUT_KEY).ok().flatten())
        .and_then(|value| serde_json::from_str(&value).ok())
        .unwrap_or_default()
}

fn save_layout(layout: CanvasLayout) {
    let Some(storage) = web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    else {
        return;
    };
    if let Ok(value) = serde_json::to_string(&layout) {
        let _ = storage.set_item(LAYOUT_KEY, &value);
    }
}
