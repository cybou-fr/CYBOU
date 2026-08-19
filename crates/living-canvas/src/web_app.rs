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

const LAYOUT_KEY: &str = "cybou.living-canvas.layout.v4";

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct Point {
    x: f64,
    y: f64,
    z: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct CanvasLayout {
    artifact: Point,
    collaborators: Point,
    release: Point,
    sources: Point,
    suggestion: Point,
    commitments: Point,
}

impl Default for CanvasLayout {
    fn default() -> Self {
        Self {
            artifact: Point {
                x: 70.0,
                y: 50.0,
                z: 1,
            },
            collaborators: Point {
                x: 55.0,
                y: 300.0,
                z: 2,
            },
            release: Point {
                x: 445.0,
                y: 70.0,
                z: 6,
            },
            sources: Point {
                x: 880.0,
                y: 50.0,
                z: 3,
            },
            suggestion: Point {
                x: 900.0,
                y: 300.0,
                z: 5,
            },
            commitments: Point {
                x: 470.0,
                y: 410.0,
                z: 4,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Panel {
    Artifact,
    Collaborators,
    Release,
    Sources,
    Suggestion,
    Commitments,
}

impl Panel {
    const fn key(self) -> &'static str {
        match self {
            Self::Artifact => "artifact",
            Self::Collaborators => "collaborators",
            Self::Release => "release",
            Self::Sources => "sources",
            Self::Suggestion => "suggestion",
            Self::Commitments => "commitments",
        }
    }

    const fn size(self) -> (f64, f64) {
        match self {
            Self::Artifact => (220.0, 188.0),
            Self::Collaborators => (240.0, 236.0),
            Self::Release => (390.0, 294.0),
            Self::Sources => (245.0, 230.0),
            Self::Suggestion => (335.0, 252.0),
            Self::Commitments => (310.0, 184.0),
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
            Panel::Artifact => self.artifact,
            Panel::Collaborators => self.collaborators,
            Panel::Release => self.release,
            Panel::Sources => self.sources,
            Panel::Suggestion => self.suggestion,
            Panel::Commitments => self.commitments,
        }
    }

    fn set_point(&mut self, panel: Panel, point: Point) {
        match panel {
            Panel::Artifact => self.artifact = point,
            Panel::Collaborators => self.collaborators = point,
            Panel::Release => self.release = point,
            Panel::Sources => self.sources = point,
            Panel::Suggestion => self.suggestion = point,
            Panel::Commitments => self.commitments = point,
        }
    }

    fn bring_forward(&mut self, panel: Panel) {
        let next = [
            self.artifact.z,
            self.collaborators.z,
            self.release.z,
            self.sources.z,
            self.suggestion.z,
            self.commitments.z,
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
        projection_version: u64,
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
    let (selected, set_selected) = signal("release");
    let (runtime_menu_open, set_runtime_menu_open) = signal(false);
    let (minimap_visible, set_minimap_visible) = signal(true);
    let (command_open, set_command_open) = signal(false);
    let (command_query, set_command_query) = signal(String::new());
    let command_input = NodeRef::<leptos::html::Input>::new();
    let layout = RwSignal::new(load_layout());
    let dragging = RwSignal::new(None::<DragState>);
    let runtime = RwSignal::new(RuntimeState::Loading);
    spawn_local(async move {
        let client = GatewayMindClient;
        let result = async {
            let session = client.session().await?;
            let snapshot = client.snapshot().await?;
            Ok::<_, living_canvas::ClientError>((session.mode, snapshot.projection_version))
        }
        .await;
        runtime.set(match result {
            Ok((mode, projection_version)) => RuntimeState::Ready {
                mode,
                projection_version,
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
                    projection_version, ..
                } = state
                {
                    *projection_version = snapshot.projection_version;
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
        RuntimeState::Ready {
            projection_version, ..
        } => format!("Gateway · projection {projection_version}"),
        RuntimeState::Error(error) => error,
    };
    let system_label = move || match runtime.get() {
        RuntimeState::Loading => "Connecting to local gateway…".into(),
        RuntimeState::Ready {
            projection_version, ..
        } => format!("System nominal · projection {projection_version}"),
        RuntimeState::Error(_) => "Gateway unavailable · canvas remains read-only".into(),
    };

    view! {
        <main class="app-shell">
            <header class="topbar">
                <a class="brand" href="#canvas" aria-label="Living Canvas home">
                    <img class="brand-mark" src="/cybou-mark.svg" alt="" />
                    <span>"Living Canvas"</span>
                </a>
                <p class="path">"Cybou Workspace / Programs / Cybou 0.8 release"</p>
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
                            <header><strong>"Cybou"</strong><small>"All data stays local"</small></header>
                            <button><FileCheck size=15 /><span>"New artifact"</span></button>
                            <button><ListChecks size=15 /><span>"New commitment"</span></button>
                            <button><UsersRound size=15 /><span>"Invite collaborator"</span></button>
                            <button><Sparkles size=15 /><span>"Open mind"</span></button>
                            <hr />
                            <button><Map size=15 /><span>"Canvas view"</span></button>
                            <button
                                aria-pressed=move || minimap_visible.get().to_string()
                                on:click=move |_| {
                                    set_minimap_visible.set(!minimap_visible.get_untracked());
                                    set_runtime_menu_open.set(false);
                                }
                            ><Map size=15 /><span>"Minimap"</span></button>
                            <hr />
                            <button><Sparkles size=15 /><span>"System status"</span></button>
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
                    <RelationshipEdge layout=layout selected=selected from=Panel::Artifact to=Panel::Release label="delivers" amber=false />
                    <RelationshipEdge layout=layout selected=selected from=Panel::Collaborators to=Panel::Release label="involves" amber=false />
                    <RelationshipEdge layout=layout selected=selected from=Panel::Release to=Panel::Sources label="validated by" amber=false />
                    <RelationshipEdge layout=layout selected=selected from=Panel::Release to=Panel::Suggestion label="depends on" amber=true />
                    <RelationshipEdge layout=layout selected=selected from=Panel::Release to=Panel::Commitments label="tracked by" amber=false />
                </svg>
                <button
                    class:selected=move || selected.get() == "artifact"
                    class="object artifact"
                    style=move || panel_style(layout.get(), Panel::Artifact)
                    aria-label="Artifact panel. Drag to reposition; use arrow keys for keyboard movement."
                    on:pointerdown=move |event| start_drag(event, Panel::Artifact, layout, dragging)
                    on:keydown=move |event| keyboard_move(event, Panel::Artifact, layout)
                    on:click=move |_| set_selected.set("artifact")
                >
                    <small class="panel-kicker"><FileCheck size=14 /><span>"Verified artifact"</span></small>
                    <strong>"cybou-0.8.0.tar.zst"</strong>
                    <span class="artifact-digest">"SHA256: a3f2…9c7b"</span>
                    <span class="artifact-badges"><i>"Reproducible"</i><i>"Signed"</i></span>
                    <span class="artifact-meta">"Build · 10m ago · Local"</span>
                </button>

                <Show when=move || selected.get() == "release">
                    <nav
                        class="object-actions"
                        style=move || release_actions_style(layout.get())
                        aria-label="Selected release actions"
                    >
                        <button><FolderOpen size=15 /><span>"Open"</span></button>
                        <button><ListChecks size=15 /><span>"Plan"</span></button>
                        <button class="active"><Sparkles size=15 /><span>"Mind"</span></button>
                        <button><Link size=15 /><span>"Link"</span></button>
                        <button aria-label="More release actions"><Ellipsis size=16 /><span class="sr-only">"More"</span></button>
                    </nav>
                </Show>

                <button
                    class:selected=move || selected.get() == "collaborators"
                    class="object collaborators"
                    style=move || panel_style(layout.get(), Panel::Collaborators)
                    aria-label="Collaborators panel. Drag to reposition; use arrow keys for keyboard movement."
                    on:pointerdown=move |event| start_drag(event, Panel::Collaborators, layout, dragging)
                    on:keydown=move |event| keyboard_move(event, Panel::Collaborators, layout)
                    on:click=move |_| set_selected.set("collaborators")
                >
                    <small class="panel-kicker"><UsersRound size=14 /><span>"Collaborators"</span></small>
                    <strong>"Release team"</strong>
                    <span class="row"><b>"Ari N."</b><i>"Owner"</i></span>
                    <span class="row"><b>"Mina K."</b><i>"Release lead"</i></span>
                    <span class="row"><b>"Jonas L."</b><i>"QA lead"</i></span>
                    <span class="row"><b>"Priya S."</b><i>"Security"</i></span>
                    <span class="row"><b>"Devon R."</b><i>"Docs"</i></span>
                    <span class="panel-link">"Invite collaborator"</span>
                </button>

                <button
                    class:selected=move || selected.get() == "release"
                    class="object release"
                    style=move || panel_style(layout.get(), Panel::Release)
                    aria-label="Release plan panel. Drag to reposition; use arrow keys for keyboard movement."
                    on:pointerdown=move |event| start_drag(event, Panel::Release, layout, dragging)
                    on:keydown=move |event| keyboard_move(event, Panel::Release, layout)
                    on:click=move |_| set_selected.set("release")
                >
                    <small class="panel-kicker"><Sparkles size=14 /><span>"Release plan"</span></small>
                    <h1>"Cybou 0.8 release"</h1>
                    <span class="release-kind">"Release plan"</span>
                    <p>"Stable release with local-first guarantees, improved reliability, and rollback safety."</p>
                    <div class="progress-label"><span>"Progress"</span><strong>"68%"</strong></div>
                    <div class="progress" aria-label="Release progress 68 percent"><span></span></div>
                    <footer class="release-meta">
                        <span><small>"Target"</small><b>"May 30, 2025"</b></span>
                        <span><small>"Owner"</small><b>"Ari N."</b></span>
                        <span><small>"State"</small><b class="nominal">"On track"</b></span>
                    </footer>
                </button>

                <button
                    class:selected=move || selected.get() == "sources"
                    class="object sources"
                    style=move || panel_style(layout.get(), Panel::Sources)
                    aria-label="Sources panel. Drag to reposition; use arrow keys for keyboard movement."
                    on:pointerdown=move |event| start_drag(event, Panel::Sources, layout, dragging)
                    on:keydown=move |event| keyboard_move(event, Panel::Sources, layout)
                    on:click=move |_| set_selected.set("sources")
                >
                    <small class="panel-kicker"><Files size=14 /><span>"Sources"</span></small>
                    <strong>"Validated inputs"</strong>
                    <span class="row"><b>"Design doc"</b><i>"v4"</i></span>
                    <span class="row"><b>"Changelog"</b><i>"v3"</i></span>
                    <span class="row"><b>"Test results"</b><i>"128"</i></span>
                    <span class="row"><b>"Threat model"</b><i>"v2"</i></span>
                    <span class="row"><b>"Dependencies"</b><i>"52"</i></span>
                    <span class="sources-footer"><i>"5 items"</i><b>"View all"</b></span>
                </button>

                <article
                    class:selected=move || selected.get() == "suggestion"
                    class="object suggestion"
                    style=move || panel_style(layout.get(), Panel::Suggestion)
                    tabindex="0"
                    aria-label="Mind suggestion panel. Drag to reposition; use arrow keys for keyboard movement."
                    on:pointerdown=move |event| start_drag(event, Panel::Suggestion, layout, dragging)
                    on:keydown=move |event| keyboard_move(event, Panel::Suggestion, layout)
                    on:click=move |_| set_selected.set("suggestion")
                >
                    <header class="suggestion-heading">
                        <small class="panel-kicker"><Sparkles size=14 /><span>"Mind suggests"</span></small>
                        <b>"High impact"</b>
                    </header>
                    <strong>"Add rollback verification"</strong>
                    <p>"No rollback test detected in plan. Adds safety and release confidence."</p>
                    <div class="suggestion-actions">
                        <button on:pointerdown=move |event: PointerEvent| event.stop_propagation()>"Review evidence"</button>
                        <button class="primary" on:pointerdown=move |event: PointerEvent| event.stop_propagation()>"Add to plan"</button>
                    </div>
                    <span class="suggestion-source">"Source: test-results.json, design-doc.md"</span>
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
                    <small class="panel-kicker"><ListChecks size=14 /><span>"3 commitments"</span></small>
                    <span class="check-row"><b>"Complete test matrix"</b><i>"May 20"</i></span>
                    <span class="check-row"><b>"Security review"</b><i>"May 22"</i></span>
                    <span class="check-row"><b>"Docs & migration guide"</b><i>"May 26"</i></span>
                    <span class="panel-link">"View commitments"</span>
                </button>

                <Show when=move || command_open.get()>
                    <nav class="command-palette" aria-label="Canvas commands">
                        <small>"Jump to"</small>
                        <button
                            class:hidden=move || !command_matches(&command_query.get(), "release plan")
                            on:click=move |_| select_from_command("release", set_selected, set_command_open, set_command_query)
                        ><Sparkles size=15 /><span><b>"Release plan"</b><i>"Central object"</i></span></button>
                        <button
                            class:hidden=move || !command_matches(&command_query.get(), "verified artifact evidence")
                            on:click=move |_| select_from_command("artifact", set_selected, set_command_open, set_command_query)
                        ><FileCheck size=15 /><span><b>"Verified artifact"</b><i>"Evidence and provenance"</i></span></button>
                        <button
                            class:hidden=move || !command_matches(&command_query.get(), "collaborators release team")
                            on:click=move |_| select_from_command("collaborators", set_selected, set_command_open, set_command_query)
                        ><UsersRound size=15 /><span><b>"Collaborators"</b><i>"Release team"</i></span></button>
                        <button
                            class:hidden=move || !command_matches(&command_query.get(), "sources validated inputs")
                            on:click=move |_| select_from_command("sources", set_selected, set_command_open, set_command_query)
                        ><Files size=15 /><span><b>"Sources"</b><i>"Validated inputs"</i></span></button>
                        <button
                            class:hidden=move || !command_matches(&command_query.get(), "mind suggestion rollback")
                            on:click=move |_| select_from_command("suggestion", set_selected, set_command_open, set_command_query)
                        ><Sparkles size=15 /><span><b>"Mind suggestion"</b><i>"Rollback verification"</i></span></button>
                        <button
                            class:hidden=move || !command_matches(&command_query.get(), "commitments tasks")
                            on:click=move |_| select_from_command("commitments", set_selected, set_command_open, set_command_query)
                        ><ListChecks size=15 /><span><b>"Commitments"</b><i>"Three tracked tasks"</i></span></button>
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
                        <header><Map size=15 /><strong>"Canvas map"</strong><span>"79%"</span></header>
                        <div class="minimap-field">
                            <button
                                class:selected=move || selected.get() == "artifact"
                                class="mini-node artifact-node"
                                style=move || minimap_style(layout.get().artifact)
                                aria-label="Select artifact panel"
                                on:click=move |_| set_selected.set("artifact")
                            ></button>
                            <button
                                class:selected=move || selected.get() == "collaborators"
                                class="mini-node collaborators-node"
                                style=move || minimap_style(layout.get().collaborators)
                                aria-label="Select collaborators panel"
                                on:click=move |_| set_selected.set("collaborators")
                            ></button>
                            <button
                                class:selected=move || selected.get() == "release"
                                class="mini-node release-node"
                                style=move || minimap_style(layout.get().release)
                                aria-label="Select release panel"
                                on:click=move |_| set_selected.set("release")
                            ></button>
                            <button
                                class:selected=move || selected.get() == "sources"
                                class="mini-node sources-node"
                                style=move || minimap_style(layout.get().sources)
                                aria-label="Select sources panel"
                                on:click=move |_| set_selected.set("sources")
                            ></button>
                            <button
                                class:selected=move || selected.get() == "suggestion"
                                class="mini-node suggestion-node"
                                style=move || minimap_style(layout.get().suggestion)
                                aria-label="Select mind suggestion panel"
                                on:click=move |_| set_selected.set("suggestion")
                            ></button>
                            <button
                                class:selected=move || selected.get() == "commitments"
                                class="mini-node commitments-node"
                                style=move || minimap_style(layout.get().commitments)
                                aria-label="Select commitments panel"
                                on:click=move |_| set_selected.set("commitments")
                            ></button>
                        </div>
                    </nav>
                </Show>

                <aside class="system-state" aria-label="System state">
                    <span class="status-dot" aria-hidden="true"></span>
                    {system_label}
                </aside>
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

fn command_matches(query: &str, haystack: &str) -> bool {
    let query = query.trim().to_ascii_lowercase();
    query.is_empty() || haystack.contains(&query)
}

fn first_command_match(query: &str) -> Option<&'static str> {
    [
        ("release", "release plan"),
        ("artifact", "verified artifact evidence"),
        ("collaborators", "collaborators release team"),
        ("sources", "sources validated inputs"),
        ("suggestion", "mind suggestion rollback"),
        ("commitments", "commitments tasks"),
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

fn release_actions_style(layout: CanvasLayout) -> String {
    let point = layout.release;
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
