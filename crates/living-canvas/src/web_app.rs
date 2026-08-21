// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! High-level Living Canvas desktop workspace application coordinator.

use leptos::prelude::*;
use leptos::task::spawn_local;
use lucide_leptos::{Ellipsis, FolderOpen, Link, ListChecks, Search, Sparkles};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{EventSource, KeyboardEvent, MessageEvent, PointerEvent};

use living_canvas::{
    ArrangementMode, CardId, ClientError, DesktopLayout, DesktopViewMode, GatewayMindClient,
    LayoutHistory, MindClient, SnapGuide,
    components::{
        AttentionCard, AuthModal, BeliefsCard, CapabilitiesCard, CommitmentsCard, ContextCard,
        DeckContainerView, DesktopDock, FileManagerCard, IconExternalLink, IconGrid, IconLayers,
        IconMaximize, IconMinimize, IconPin, IconRedo, IconRefresh, IconUndo, IdentityCard,
        JournalCard, JournalFeedCard, LifecycleCard, Minimap, PerceptionCard, RelationshipEdge,
        SelfModelCard, SessionCard, ShellCard,
    },
    interaction::{
        DragState, ResizeState, apply_redo, apply_undo, finish_drag, finish_resize, move_drag,
        move_resize, selection_actions_style,
    },
    state::{RuntimeState, command_matches, first_command_match},
};

fn load_layout() -> DesktopLayout {
    DesktopLayout::load()
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

/// Root Living Canvas Desktop Application component.
#[allow(non_snake_case)]
#[must_use]
pub fn App() -> impl IntoView {
    let (selected, set_selected) = signal("capabilities");
    let (runtime_menu_open, set_runtime_menu_open) = signal(false);
    let (minimap_visible, set_minimap_visible) = signal(true);
    let (command_open, set_command_open) = signal(false);
    let (command_query, set_command_query) = signal(String::new());
    let auth_modal_open = RwSignal::new(false);
    let (zoom, set_zoom) = signal(1.0f64);
    let (pan, set_pan) = signal((0.0f64, 0.0f64));
    let (panning, set_panning) = signal(Option::<(f64, f64, f64, f64)>::None);
    let command_input = NodeRef::<leptos::html::Input>::new();
    let view_mode = RwSignal::new(DesktopViewMode::Spatial);
    provide_context(view_mode);
    let layout = RwSignal::new(load_layout());
    let history = RwSignal::new(LayoutHistory::new());
    let dragging = RwSignal::new(None::<DragState>);
    let resizing = RwSignal::new(None::<ResizeState>);
    let snap_guides = RwSignal::new(Vec::<SnapGuide>::new());
    let runtime = RwSignal::new(RuntimeState::Loading);

    spawn_local(async move {
        let client = GatewayMindClient;
        let result = async {
            let session = client.session().await?;
            let snapshot = client.snapshot().await?;
            Ok::<_, ClientError>((session, snapshot))
        }
        .await;
        let mind = client.mind().await.ok();
        runtime.set(match result {
            Ok((session, snapshot)) => RuntimeState::Ready {
                mode: session.mode,
                session,
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
            } else if (event.ctrl_key() || event.meta_key())
                && event.key().eq_ignore_ascii_case("z")
            {
                event.prevent_default();
                if event.shift_key() {
                    apply_redo(history, layout);
                } else {
                    apply_undo(history, layout);
                }
            } else if (event.ctrl_key() || event.meta_key())
                && event.key().eq_ignore_ascii_case("y")
            {
                event.prevent_default();
                apply_redo(history, layout);
            } else if (event.ctrl_key() || event.meta_key()) && event.key() == "0" {
                event.prevent_default();
                if let Some(bbox) = layout.get_untracked().bounding_rect() {
                    let (w, h) = (
                        web_sys::window().and_then(|w| w.inner_width().ok()).and_then(|v| v.as_f64()).unwrap_or(1440.0),
                        web_sys::window().and_then(|w| w.inner_height().ok()).and_then(|v| v.as_f64()).unwrap_or(900.0),
                    );
                    let (z, (px, py)) = DesktopLayout::fit_to_viewport(bbox, w, h, 60.0);
                    set_zoom.set(z);
                    set_pan.set((px, py));
                } else {
                    set_zoom.set(1.0);
                    set_pan.set((0.0, 0.0));
                }
            } else if (event.ctrl_key() || event.meta_key()) && (event.key() == "=" || event.key() == "+") {
                event.prevent_default();
                set_zoom.update(|z| *z = (*z + 0.1).min(2.0));
            } else if (event.ctrl_key() || event.meta_key()) && (event.key() == "-" || event.key() == "_") {
                event.prevent_default();
                set_zoom.update(|z| *z = (*z - 0.1).max(0.4));
            }
        });
        let _ = window.add_event_listener_with_callback(
            "keydown",
            on_shortcut.as_ref().unchecked_ref(),
        );
        on_shortcut.forget();
    }

    let runtime_label = move || match runtime.get() {
        RuntimeState::Loading => "Connecting".to_owned(),
        RuntimeState::Ready { mode, .. } => match mode {
            cybou_web_contracts::SessionMode::LocalDesktop => "Local desktop".to_owned(),
            cybou_web_contracts::SessionMode::PublicPreview => "Public surface".to_owned(),
            cybou_web_contracts::SessionMode::RemoteBrowser => "Remote browser".to_owned(),
        },
        RuntimeState::Error(_) => "Unavailable".to_owned(),
    };

    let projection_label = move || match runtime.get() {
        RuntimeState::Loading => "Awaiting server-established session…".to_owned(),
        RuntimeState::Ready {
            snapshot, session, ..
        } => format!(
            "Projection v{} · Cursor {} · Expires {}",
            snapshot.projection_version, snapshot.cursor, session.expires_at
        ),
        RuntimeState::Error(message) => message,
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
                    {move || match runtime.get() {
                        RuntimeState::Ready { mode, .. } if mode != cybou_web_contracts::SessionMode::PublicPreview => {
                            view! {
                                <button
                                    class="topbar-auth-btn sign-out-btn"
                                    title="Sign out from host session"
                                    on:click=move |_| {
                                        spawn_local(async move {
                                            let _ = GatewayMindClient.logout().await;
                                            if let Some(w) = web_sys::window() {
                                                let _ = w.location().reload();
                                            }
                                        });
                                    }
                                >
                                    "Sign out"
                                </button>
                            }.into_any()
                        }
                        RuntimeState::Ready { mode, .. } if mode == cybou_web_contracts::SessionMode::PublicPreview => {
                            view! {
                                <button
                                    class="topbar-auth-btn"
                                    title="Sign in with host Linux account"
                                    on:click=move |_| auth_modal_open.set(true)
                                >
                                    "Sign in"
                                </button>
                            }.into_any()
                        }
                        _ => ().into_any(),
                    }}
                    <button
                        class="runtime-trigger"
                        class:open=move || runtime_menu_open.get()
                        aria-label="Open desktop layout and navigation menu"
                        aria-expanded=move || runtime_menu_open.get().to_string()
                        on:click=move |_| set_runtime_menu_open.update(|open| *open = !*open)
                    >
                        "Desktop"
                    </button>
                    <Show when=move || runtime_menu_open.get()>
                        <nav class="runtime-menu" aria-label="Desktop layout and arrangement menu">
                            <button on:click=move |_| navigate_from_menu("capabilities", set_selected, set_runtime_menu_open)><Sparkles size=15 /><span>"Capabilities"</span></button>
                            <button on:click=move |_| navigate_from_menu("identity", set_selected, set_runtime_menu_open)><IconPin size=15 /><span>"Identity"</span></button>
                            <button on:click=move |_| navigate_from_menu("session", set_selected, set_runtime_menu_open)><IconPin size=15 /><span>"Session"</span></button>
                            <button on:click=move |_| navigate_from_menu("journal", set_selected, set_runtime_menu_open)><Link size=15 /><span>"Journal"</span></button>
                            <button on:click=move |_| navigate_from_menu("lifecycle", set_selected, set_runtime_menu_open)><Sparkles size=15 /><span>"Lifecycle"</span></button>
                            <button on:click=move |_| navigate_from_menu("commitments", set_selected, set_runtime_menu_open)><ListChecks size=15 /><span>"Commitments"</span></button>
                            <button on:click=move |_| navigate_from_menu("self", set_selected, set_runtime_menu_open)><Sparkles size=15 /><span>"Self-model"</span></button>
                            <button on:click=move |_| navigate_from_menu("attention", set_selected, set_runtime_menu_open)><Sparkles size=15 /><span>"Attention"</span></button>
                            <button on:click=move |_| navigate_from_menu("beliefs", set_selected, set_runtime_menu_open)><Sparkles size=15 /><span>"Beliefs"</span></button>
                            <button on:click=move |_| navigate_from_menu("perception", set_selected, set_runtime_menu_open)><Link size=15 /><span>"Perception"</span></button>
                            <button on:click=move |_| navigate_from_menu("context", set_selected, set_runtime_menu_open)><Link size=15 /><span>"Context"</span></button>
                            <hr style="border:none;border-top:1px solid rgba(255,255,255,0.08);margin:4px 0;" />
                            <Show when=move || history.get().can_undo()>
                                <button on:click=move |_| {
                                    apply_undo(history, layout);
                                    set_runtime_menu_open.set(false);
                                }><IconUndo size=15 /><span>"Undo layout"</span></button>
                            </Show>
                            <Show when=move || history.get().can_redo()>
                                <button on:click=move |_| {
                                    apply_redo(history, layout);
                                    set_runtime_menu_open.set(false);
                                }><IconRedo size=15 /><span>"Redo layout"</span></button>
                            </Show>
                            <button on:click=move |_| {
                                history.update(|h| h.push(layout.get_untracked()));
                                layout.update(|l| {
                                    let _ = l.create_deck("Mind Core", vec![CardId::Identity, CardId::Session], 70.0, 50.0);
                                });
                                layout.get_untracked().save();
                                set_runtime_menu_open.set(false);
                            }><IconLayers size=15 /><span>"Group: Mind Deck"</span></button>
                            <button on:click=move |_| {
                                history.update(|h| h.push(layout.get_untracked()));
                                layout.update(|l| l.apply_arrangement(ArrangementMode::Home, None));
                                layout.get_untracked().save();
                                set_runtime_menu_open.set(false);
                            }><IconRefresh size=15 /><span>"Arrange: Home"</span></button>
                            <button on:click=move |_| {
                                history.update(|h| h.push(layout.get_untracked()));
                                layout.update(|l| l.apply_arrangement(ArrangementMode::Grid, None));
                                layout.get_untracked().save();
                                set_runtime_menu_open.set(false);
                            }><IconGrid size=15 /><span>"Arrange: Grid"</span></button>
                            <button on:click=move |_| {
                                history.update(|h| h.push(layout.get_untracked()));
                                layout.update(|l| l.apply_arrangement(ArrangementMode::Compact, None));
                                layout.get_untracked().save();
                                set_runtime_menu_open.set(false);
                            }><IconMinimize size=15 /><span>"Arrange: Compact"</span></button>
                            <button on:click=move |_| {
                                history.update(|h| h.push(layout.get_untracked()));
                                layout.update(|l| l.apply_arrangement(ArrangementMode::Relations, None));
                                layout.get_untracked().save();
                                set_runtime_menu_open.set(false);
                            }><Link size=15 /><span>"Arrange: Relations"</span></button>
                            <button on:click=move |_| {
                                history.update(|h| h.push(layout.get_untracked()));
                                layout.set(DesktopLayout::default());
                                layout.get_untracked().save();
                                set_runtime_menu_open.set(false);
                            }><IconRefresh size=15 /><span>"Reset layout"</span></button>
                        </nav>
                    </Show>
                </div>
            </header>

            <section
                id="canvas"
                class="canvas"
                class:panning=move || panning.get().is_some()
                aria-label="CYBOU Desktop"
                style=move || format!("transform: translate({:.1}px, {:.1}px) scale({:.2}); transform-origin: 0 0;", pan.get().0, pan.get().1, zoom.get())
                on:dblclick=move |e: web_sys::MouseEvent| {
                    if e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).map_or(false, |el| el.class_list().contains("canvas") || el.class_list().contains("ambient")) {
                        if let Some(bbox) = layout.get_untracked().bounding_rect() {
                            let (w, h) = (
                                web_sys::window().and_then(|w| w.inner_width().ok()).and_then(|v| v.as_f64()).unwrap_or(1440.0),
                                web_sys::window().and_then(|w| w.inner_height().ok()).and_then(|v| v.as_f64()).unwrap_or(900.0),
                            );
                            let (z, (px, py)) = DesktopLayout::fit_to_viewport(bbox, w, h, 60.0);
                            set_zoom.set(z);
                            set_pan.set((px, py));
                        } else {
                            set_zoom.set(1.0);
                            set_pan.set((0.0, 0.0));
                        }
                    }
                }
                on:wheel=move |event: web_sys::WheelEvent| {
                    if event.ctrl_key() {
                        event.prevent_default();
                        if event.delta_y() < 0.0 {
                            set_zoom.update(|z| *z = (*z + 0.05).min(2.0));
                        } else {
                            set_zoom.update(|z| *z = (*z - 0.05).max(0.4));
                        }
                    } else {
                        event.prevent_default();
                        set_pan.update(|(px, py)| {
                            *px -= event.delta_x() * 0.8;
                            *py -= event.delta_y() * 0.8;
                        });
                    }
                }
                on:pointerdown=move |event: PointerEvent| {
                    let is_canvas_bg = event.target()
                        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                        .map_or(false, |el| el.class_list().contains("canvas") || el.class_list().contains("ambient") || el.tag_name().eq_ignore_ascii_case("svg"));
                    if is_canvas_bg || event.button() == 1 {
                        set_panning.set(Some((event.client_x() as f64, event.client_y() as f64, pan.get().0, pan.get().1)));
                    }
                }
                on:pointermove=move |event: PointerEvent| {
                    if let Some((start_x, start_y, init_px, init_py)) = panning.get() {
                        let cur_x = event.client_x() as f64;
                        let cur_y = event.client_y() as f64;
                        set_pan.set((init_px + (cur_x - start_x), init_py + (cur_y - start_y)));
                    }
                    move_drag(event.clone(), layout, dragging, snap_guides);
                    move_resize(event, layout, resizing);
                }
                on:pointerup=move |_| {
                    set_panning.set(None);
                    finish_drag(layout, history, dragging, snap_guides);
                    finish_resize(layout, resizing);
                }
            >
                <div class="ambient ambient-primary"></div>
                <div class="ambient ambient-secondary"></div>

                <svg class="relationships" aria-hidden="true">
                    <RelationshipEdge layout=layout selected=selected from=CardId::Identity to=CardId::Session label="proves" amber=false />
                    <RelationshipEdge layout=layout selected=selected from=CardId::Capabilities to=CardId::Journal label="audits" amber=false />
                    <RelationshipEdge layout=layout selected=selected from=CardId::Lifecycle to=CardId::Commitments label="suspends" amber=false />
                    <RelationshipEdge layout=layout selected=selected from=CardId::SelfModel to=CardId::Attention label="guides" amber=false />
                    <RelationshipEdge layout=layout selected=selected from=CardId::Beliefs to=CardId::Perception label="updates" amber=false />
                    <RelationshipEdge layout=layout selected=selected from=CardId::Context to=CardId::Attention label="primes" amber=false />
                </svg>

                <svg class="snap-guides-layer" aria-hidden="true">
                    <For
                        each=move || snap_guides.get()
                        key=|g| match g {
                            SnapGuide::Vertical(x) => format!("v-{x}"),
                            SnapGuide::Horizontal(y) => format!("h-{y}"),
                        }
                        children=move |g| {
                            match g {
                                SnapGuide::Vertical(x) => view! {
                                    <line
                                        class="snap-guide-line vertical"
                                        x1=x.to_string()
                                        y1="-2000"
                                        x2=x.to_string()
                                        y2="4000"
                                    />
                                }.into_any(),
                                SnapGuide::Horizontal(y) => view! {
                                    <line
                                        class="snap-guide-line horizontal"
                                        x1="-2000"
                                        y1=y.to_string()
                                        x2="4000"
                                        y2=y.to_string()
                                    />
                                }.into_any(),
                            }
                        }
                    />
                </svg>

                <IdentityCard layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing runtime=runtime />
                <SessionCard layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing runtime=runtime />
                <CapabilitiesCard layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing runtime=runtime />
                <JournalCard layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing runtime=runtime />
                <LifecycleCard layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing runtime=runtime />
                <CommitmentsCard layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing runtime=runtime />
                <SelfModelCard layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing runtime=runtime />
                <AttentionCard layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing runtime=runtime />
                <BeliefsCard layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing runtime=runtime />
                <PerceptionCard layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing runtime=runtime />
                <ContextCard layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing runtime=runtime />

                <ShellCard layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing auth_modal_open=auth_modal_open runtime=runtime />
                <FileManagerCard layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing auth_modal_open=auth_modal_open runtime=runtime />
                <JournalFeedCard layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing />

                <For
                    each=move || layout.get().decks
                    key=|deck| deck.id.clone()
                    children=move |deck| {
                        view! {
                            <DeckContainerView
                                deck_id=deck.id
                                layout=layout
                                history=history
                                dragging=dragging
                                resizing=resizing
                                runtime=runtime
                            />
                        }
                    }
                />

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

                <div class="canvas-controls">
                    <button
                        class="canvas-btn"
                        title="Zoom In (Ctrl +)"
                        aria-label="Zoom In"
                        on:click=move |_| set_zoom.update(|z| *z = (*z + 0.1).min(2.0))
                    >
                        "+"
                    </button>
                    <span class="zoom-indicator">{move || format!("{:.0}%", zoom.get() * 100.0)}</span>
                    <button
                        class="canvas-btn"
                        title="Zoom Out (Ctrl -)"
                        aria-label="Zoom Out"
                        on:click=move |_| set_zoom.update(|z| *z = (*z - 0.1).max(0.4))
                    >
                        "−"
                    </button>
                    <button
                        class="canvas-btn"
                        title="Fit all cards to viewport (Ctrl 0)"
                        aria-label="Fit All"
                        on:click=move |_| {
                            if let Some(bbox) = layout.get_untracked().bounding_rect() {
                                let (w, h) = (
                                    web_sys::window().and_then(|w| w.inner_width().ok()).and_then(|v| v.as_f64()).unwrap_or(1440.0),
                                    web_sys::window().and_then(|w| w.inner_height().ok()).and_then(|v| v.as_f64()).unwrap_or(900.0),
                                );
                                let (z, (px, py)) = DesktopLayout::fit_to_viewport(bbox, w, h, 60.0);
                                set_zoom.set(z);
                                set_pan.set((px, py));
                            } else {
                                set_zoom.set(1.0);
                                set_pan.set((0.0, 0.0));
                            }
                        }
                    >
                        <IconMaximize size=12 />
                    </button>
                    <button
                        class="canvas-btn"
                        class:active=minimap_visible
                        title="Toggle Minimap"
                        aria-label="Toggle Minimap"
                        on:click=move |_| set_minimap_visible.update(|v| *v = !*v)
                    >
                        <IconGrid size=12 />
                    </button>
                </div>
            </section>

            <section class="command-palette" aria-label="Action launcher">
                <Show when=move || command_open.get()>
                    <nav class="command-menu" aria-label="Command palette actions">
                        <button
                            class:hidden=move || !command_matches(&command_query.get(), "capabilities health dependencies")
                            on:click=move |_| select_from_command("capabilities", set_selected, set_command_open, set_command_query)
                        ><Sparkles size=15 /><span><b>"Open Capabilities"</b><i>"Health1 capability dependencies"</i></span></button>
                        <button
                            class:hidden=move || !command_matches(&command_query.get(), "identity subject continuity provenance")
                            on:click=move |_| select_from_command("identity", set_selected, set_command_open, set_command_query)
                        ><IconPin size=15 /><span><b>"Open Identity"</b><i>"Identity1 subject continuity"</i></span></button>
                        <button
                            class:hidden=move || !command_matches(&command_query.get(), "session trust gateway authentication mode")
                            on:click=move |_| select_from_command("session", set_selected, set_command_open, set_command_query)
                        ><IconPin size=15 /><span><b>"Open Session"</b><i>"Gateway trust and session mode"</i></span></button>
                        <button
                            class:hidden=move || !command_matches(&command_query.get(), "journal contributions causal integrity event1")
                            on:click=move |_| select_from_command("journal", set_selected, set_command_open, set_command_query)
                        ><Link size=15 /><span><b>"Open Journal"</b><i>"Event1 canonical event log"</i></span></button>
                        <button
                            class:hidden=move || !command_matches(&command_query.get(), "lifecycle sleep wake consolidation")
                            on:click=move |_| select_from_command("lifecycle", set_selected, set_command_open, set_command_query)
                        ><Sparkles size=15 /><span><b>"Open Lifecycle"</b><i>"Lifecycle1 sleep and wake state"</i></span></button>
                        <button
                            class:hidden=move || !command_matches(&command_query.get(), "commitments obligations intention1")
                            on:click=move |_| select_from_command("commitments", set_selected, set_command_open, set_command_query)
                        ><ListChecks size=15 /><span><b>"Open Commitments"</b><i>"Intention1 open obligations"</i></span></button>
                        <button
                            class:hidden=move || !command_matches(&command_query.get(), "self assessment autobiographical narration self1")
                            on:click=move |_| select_from_command("self", set_selected, set_command_open, set_command_query)
                        ><Sparkles size=15 /><span><b>"Open Self-Model"</b><i>"Self1 autobiographical narration"</i></span></button>
                        <button
                            class:hidden=move || !command_matches(&command_query.get(), "attention focus global workspace theory workspace1")
                            on:click=move |_| select_from_command("attention", set_selected, set_command_open, set_command_query)
                        ><Sparkles size=15 /><span><b>"Open Attention"</b><i>"Workspace1 attention focus"</i></span></button>
                        <button
                            class:hidden=move || !command_matches(&command_query.get(), "beliefs epistemic1 validity propositions")
                            on:click=move |_| select_from_command("beliefs", set_selected, set_command_open, set_command_query)
                        ><Sparkles size=15 /><span><b>"Open Beliefs"</b><i>"Epistemic1 derived propositions"</i></span></button>
                        <button
                            class:hidden=move || !command_matches(&command_query.get(), "perception observations host perception1")
                            on:click=move |_| select_from_command("perception", set_selected, set_command_open, set_command_query)
                        ><Link size=15 /><span><b>"Open Perception"</b><i>"Perception1 host facts"</i></span></button>
                        <button
                            class:hidden=move || !command_matches(&command_query.get(), "context association concepts context1")
                            on:click=move |_| select_from_command("context", set_selected, set_command_open, set_command_query)
                        ><Link size=15 /><span><b>"Open Context"</b><i>"Context1 associative graph"</i></span></button>
                        <button
                            class:hidden=move || !command_matches(&command_query.get(), "shell terminal bash command zone3 body")
                            on:click=move |_| {
                                layout.update(|l| l.open_card(CardId::Shell(0), 400.0, 160.0));
                                layout.get_untracked().save();
                                set_selected.set("shell");
                                set_command_open.set(false);
                                set_command_query.set(String::new());
                            }
                        ><IconExternalLink size=15 /><span><b>"Launch CYBOU Shell"</b><i>"Bounded Zone 3 Body capability"</i></span></button>
                        <button
                            class:hidden=move || !command_matches(&command_query.get(), "files file manager storage browse read-only")
                            on:click=move |_| {
                                layout.update(|l| l.open_card(CardId::FileManager(0), 380.0, 120.0));
                                layout.get_untracked().save();
                                set_selected.set("files");
                                set_command_open.set(false);
                                set_command_query.set(String::new());
                            }
                        ><IconExternalLink size=15 /><span><b>"Launch File Manager"</b><i>"Bounded Zone 3 read-only browser"</i></span></button>
                        <button
                            class:hidden=move || !command_matches(&command_query.get(), "events feed live stream sse journal")
                            on:click=move |_| {
                                layout.update(|l| l.open_card(CardId::JournalFeed(0), 420.0, 150.0));
                                layout.get_untracked().save();
                                set_selected.set("journal-feed");
                                set_command_open.set(false);
                                set_command_query.set(String::new());
                            }
                        ><IconExternalLink size=15 /><span><b>"Open Event Stream"</b><i>"Real-time Journal SSE stream"</i></span></button>
                        <button
                            class:hidden=move || !command_matches(&command_query.get(), "auth sign in login pam")
                            on:click=move |_| {
                                auth_modal_open.set(true);
                                set_command_open.set(false);
                                set_command_query.set(String::new());
                            }
                        ><IconPin size=15 /><span><b>"Authenticate / Sign in"</b><i>"Linux PAM credentials"</i></span></button>
                        <button
                            class:hidden=move || !command_matches(&command_query.get(), "group mind deck cards")
                            on:click=move |_| {
                                history.update(|h| h.push(layout.get_untracked()));
                                layout.update(|l| {
                                    let _ = l.create_deck("Mind Core", vec![CardId::Identity, CardId::Session], 70.0, 50.0);
                                });
                                layout.get_untracked().save();
                                set_command_open.set(false);
                                set_command_query.set(String::new());
                            }
                        ><IconLayers size=15 /><span><b>"Create Deck: Mind Core"</b><i>"Group Identity and Session"</i></span></button>
                        <button
                            class:hidden=move || !command_matches(&command_query.get(), "undo layout revert")
                            on:click=move |_| {
                                apply_undo(history, layout);
                                set_command_open.set(false);
                                set_command_query.set(String::new());
                            }
                        ><IconUndo size=15 /><span><b>"Undo Layout Change"</b><i>"Revert position or deck state"</i></span></button>
                        <button
                            class:hidden=move || !command_matches(&command_query.get(), "redo layout forward")
                            on:click=move |_| {
                                apply_redo(history, layout);
                                set_command_open.set(false);
                                set_command_query.set(String::new());
                            }
                        ><IconRedo size=15 /><span><b>"Redo Layout Change"</b><i>"Re-apply position or deck state"</i></span></button>
                        <button
                            class:hidden=move || !command_matches(&command_query.get(), "arrange home canonical default")
                            on:click=move |_| {
                                history.update(|h| h.push(layout.get_untracked()));
                                layout.update(|l| l.apply_arrangement(ArrangementMode::Home, None));
                                layout.get_untracked().save();
                                set_command_open.set(false);
                                set_command_query.set(String::new());
                            }
                        ><IconRefresh size=15 /><span><b>"Arrange: Home"</b><i>"Canonical workspace overview"</i></span></button>
                        <button
                            class:hidden=move || !command_matches(&command_query.get(), "arrange grid structured columns")
                            on:click=move |_| {
                                history.update(|h| h.push(layout.get_untracked()));
                                layout.update(|l| l.apply_arrangement(ArrangementMode::Grid, None));
                                layout.get_untracked().save();
                                set_command_open.set(false);
                                set_command_query.set(String::new());
                            }
                        ><IconGrid size=15 /><span><b>"Arrange: Grid"</b><i>"Structured multi-track lanes"</i></span></button>
                        <button
                            class:hidden=move || !command_matches(&command_query.get(), "arrange compact packing fit")
                            on:click=move |_| {
                                history.update(|h| h.push(layout.get_untracked()));
                                layout.update(|l| l.apply_arrangement(ArrangementMode::Compact, None));
                                layout.get_untracked().save();
                                set_command_open.set(false);
                                set_command_query.set(String::new());
                            }
                        ><IconMinimize size=15 /><span><b>"Arrange: Compact"</b><i>"Dense packing"</i></span></button>
                        <button
                            class:hidden=move || !command_matches(&command_query.get(), "arrange relations causal")
                            on:click=move |_| {
                                history.update(|h| h.push(layout.get_untracked()));
                                layout.update(|l| l.apply_arrangement(ArrangementMode::Relations, None));
                                layout.get_untracked().save();
                                set_command_open.set(false);
                                set_command_query.set(String::new());
                            }
                        ><Link size=15 /><span><b>"Arrange: Relations"</b><i>"Mind organ graph"</i></span></button>
                        <button
                            class:hidden=move || !command_matches(&command_query.get(), "fit all zoom viewport center")
                            on:click=move |_| {
                                if let Some(bbox) = layout.get_untracked().bounding_rect() {
                                    let (w, h) = (
                                        web_sys::window().and_then(|w| w.inner_width().ok()).and_then(|v| v.as_f64()).unwrap_or(1440.0),
                                        web_sys::window().and_then(|w| w.inner_height().ok()).and_then(|v| v.as_f64()).unwrap_or(900.0),
                                    );
                                    let (z, (px, py)) = DesktopLayout::fit_to_viewport(bbox, w, h, 60.0);
                                    set_zoom.set(z);
                                    set_pan.set((px, py));
                                } else {
                                    set_zoom.set(1.0);
                                    set_pan.set((0.0, 0.0));
                                }
                                set_command_open.set(false);
                                set_command_query.set(String::new());
                            }
                        ><IconMaximize size=15 /><span><b>"Fit All to Viewport"</b><i>"Ctrl+0 · Center and scale canvas"</i></span></button>
                        <button
                            class:hidden=move || !command_matches(&command_query.get(), "reset layout")
                            on:click=move |_| {
                                history.update(|h| h.push(layout.get_untracked()));
                                layout.set(DesktopLayout::default());
                                layout.get_untracked().save();
                                set_command_open.set(false);
                                set_command_query.set(String::new());
                            }
                        ><IconRefresh size=15 /><span><b>"Reset Desktop Layout"</b><i>"Default coordinates"</i></span></button>
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
                            if event.key() == "Enter" {
                                let q = command_query.get();
                                if command_matches(&q, "undo") {
                                    event.prevent_default();
                                    apply_undo(history, layout);
                                    set_command_open.set(false);
                                    set_command_query.set(String::new());
                                } else if command_matches(&q, "redo") {
                                    event.prevent_default();
                                    apply_redo(history, layout);
                                    set_command_open.set(false);
                                    set_command_query.set(String::new());
                                } else if command_matches(&q, "arrange home") {
                                    event.prevent_default();
                                    history.update(|h| h.push(layout.get_untracked()));
                                    layout.update(|l| l.apply_arrangement(ArrangementMode::Home, None));
                                    layout.get_untracked().save();
                                    set_command_open.set(false);
                                    set_command_query.set(String::new());
                                } else if command_matches(&q, "arrange grid") {
                                    event.prevent_default();
                                    history.update(|h| h.push(layout.get_untracked()));
                                    layout.update(|l| l.apply_arrangement(ArrangementMode::Grid, None));
                                    layout.get_untracked().save();
                                    set_command_open.set(false);
                                    set_command_query.set(String::new());
                                } else if command_matches(&q, "arrange compact") {
                                    event.prevent_default();
                                    history.update(|h| h.push(layout.get_untracked()));
                                    layout.update(|l| l.apply_arrangement(ArrangementMode::Compact, None));
                                    layout.get_untracked().save();
                                    set_command_open.set(false);
                                    set_command_query.set(String::new());
                                } else if command_matches(&q, "arrange relations") {
                                    event.prevent_default();
                                    history.update(|h| h.push(layout.get_untracked()));
                                    layout.update(|l| l.apply_arrangement(ArrangementMode::Relations, None));
                                    layout.get_untracked().save();
                                    set_command_open.set(false);
                                    set_command_query.set(String::new());
                                } else if command_matches(&q, "reset layout") {
                                    event.prevent_default();
                                    history.update(|h| h.push(layout.get_untracked()));
                                    layout.set(DesktopLayout::default());
                                    layout.get_untracked().save();
                                    set_command_open.set(false);
                                    set_command_query.set(String::new());
                                } else if command_matches(&q, "fit") || command_matches(&q, "fit all") {
                                    event.prevent_default();
                                    if let Some(bbox) = layout.get_untracked().bounding_rect() {
                                        let (w, h) = (
                                            web_sys::window().and_then(|w| w.inner_width().ok()).and_then(|v| v.as_f64()).unwrap_or(1440.0),
                                            web_sys::window().and_then(|w| w.inner_height().ok()).and_then(|v| v.as_f64()).unwrap_or(900.0),
                                        );
                                        let (z, (px, py)) = DesktopLayout::fit_to_viewport(bbox, w, h, 60.0);
                                        set_zoom.set(z);
                                        set_pan.set((px, py));
                                    } else {
                                        set_zoom.set(1.0);
                                        set_pan.set((0.0, 0.0));
                                    }
                                    set_command_open.set(false);
                                    set_command_query.set(String::new());
                                } else if command_matches(&q, "files") || command_matches(&q, "file manager") {
                                    event.prevent_default();
                                    layout.update(|l| l.open_card(CardId::FileManager(0), 380.0, 120.0));
                                    layout.get_untracked().save();
                                    set_selected.set("files");
                                    set_command_open.set(false);
                                    set_command_query.set(String::new());
                                } else if command_matches(&q, "feed") || command_matches(&q, "event stream") {
                                    event.prevent_default();
                                    layout.update(|l| l.open_card(CardId::JournalFeed(0), 420.0, 150.0));
                                    layout.get_untracked().save();
                                    set_selected.set("journal-feed");
                                    set_command_open.set(false);
                                    set_command_query.set(String::new());
                                } else if command_matches(&q, "auth") || command_matches(&q, "login") || command_matches(&q, "sign in") {
                                    event.prevent_default();
                                    auth_modal_open.set(true);
                                    set_command_open.set(false);
                                    set_command_query.set(String::new());
                                } else if command_matches(&q, "shell") {
                                    event.prevent_default();
                                    layout.update(|l| l.open_card(CardId::Shell(0), 400.0, 160.0));
                                    layout.get_untracked().save();
                                    set_selected.set("shell");
                                    set_command_open.set(false);
                                    set_command_query.set(String::new());
                                } else if let Some(panel) = first_command_match(&q) {
                                    event.prevent_default();
                                    select_from_command(
                                        panel,
                                        set_selected,
                                        set_command_open,
                                        set_command_query,
                                    );
                                }
                            }
                        }
                    />
                    <kbd>"Ctrl K"</kbd>
                </label>

                <Show when=move || minimap_visible.get()>
                    <Minimap layout=layout zoom=zoom set_pan=set_pan />
                </Show>
            </section>

            <DesktopDock
                layout=layout
                selected=selected
                set_selected=set_selected
                auth_modal_open=auth_modal_open
                runtime=runtime
            />

            <AuthModal open=auth_modal_open />
        </main>
    }
}
