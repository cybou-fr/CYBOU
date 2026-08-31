// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! High-level Living Canvas desktop workspace application coordinator.

use cybou_web_contracts::SessionMode;
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{BeforeUnloadEvent, KeyboardEvent};

use living_canvas::{
    CameraHistory, ClientError, DesktopLayout, DesktopViewMode, GatewayMindClient, LayoutHistory,
    MindClient, SnapGuide, apply_camera_back, apply_camera_forward,
    components::{
        AuthModal, CanvasViewport, CommandPalette, DesktopDock, IconGrid, IconMaximize, Minimap,
        SignInView, Topbar,
    },
    interaction::{DragState, ResizeState, apply_redo, apply_undo},
    state::{DesktopRuntimeSubscription, RuntimeState},
    tool_state::{EditorTab, ToolCardStates},
};

fn load_layout() -> DesktopLayout {
    DesktopLayout::load()
}

#[cfg(target_arch = "wasm32")]
fn is_editing_target(event: &KeyboardEvent) -> bool {
    if event.is_composing() {
        return true;
    }
    if let Some(target) = event
        .target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok())
    {
        let tag = target.tag_name().to_ascii_uppercase();
        if tag == "TEXTAREA" || tag == "INPUT" || target.is_content_editable() {
            return true;
        }
        if target
            .closest(".editor-surface, .terminal-surface, [data-shortcut-scope='local']")
            .ok()
            .flatten()
            .is_some()
        {
            return true;
        }
    }
    false
}

/// Root Living Canvas Desktop Application component.
#[allow(non_snake_case)]
#[must_use]
pub fn App() -> impl IntoView {
    // The item, not its kind. A key identifies a kind: `Shell(0)` and `Shell(2)` share one, so
    // comparing keys selected every Shell card at once and the action bar acted on the first.
    let (selected, set_selected) = signal(Some(living_canvas::DesktopItemId::Card(
        living_canvas::CardId::Capabilities,
    )));
    let (runtime_menu_open, set_runtime_menu_open) = signal(false);
    let (minimap_visible, set_minimap_visible) = signal(true);
    // Sticking is asked for rather than assumed. A card dragged across a small window spends most
    // of the journey inside its bounds plus twenty-four pixels, so the magnet used to catch cards
    // that were only travelling through — and where they landed was not where the hand let go.
    let magnet = RwSignal::new(false);
    provide_context(magnet);
    let (command_open, set_command_open) = signal(false);
    let (command_query, set_command_query) = signal(String::new());
    let auth_modal_open = RwSignal::new(false);
    let (zoom, set_zoom) = signal(1.0f64);
    let (pan, set_pan) = signal((0.0f64, 0.0f64));
    let (panning, set_panning) = signal(Option::<(f64, f64, f64, f64)>::None);

    // Where the camera is, provided once for the whole desktop. It used to be provided by the
    // viewport, which meant only what the viewport draws could read it — and the Dock is its
    // sibling, so the Dock could not ask whether the desktop is a plane or a column. The
    // alternative was a media query answering the same question one pixel differently.
    provide_context(living_canvas::components::camera_context::CanvasCamera {
        pan,
        zoom,
        viewport: living_canvas::components::camera_context::window_size(),
    });
    let command_input = NodeRef::<leptos::html::Input>::new();
    let view_mode = RwSignal::new(DesktopViewMode::Spatial);
    provide_context(view_mode);
    // Built here so every tool card's state is owned by the root rather than by whichever mount
    // happens to be showing it. Collapsing a card, switching a deck tab, or docking a card all
    // unmount its content; none of them are a person discarding what they had done.
    let tool_states = ToolCardStates::new();
    provide_context(tool_states);
    // One clock for every age label in the desktop. Started here rather than per panel so a
    // hundred open cards do not run a hundred timers to say the same thing.
    living_canvas::refresh::provide_desktop_clock();
    let layout = RwSignal::new(load_layout());
    provide_context(layout);
    // The local copy is what there is until the account answers. From here on it is a cache
    // in front of the arrangement the seat carries, rather than the only place it exists.
    #[cfg(target_arch = "wasm32")]
    living_canvas::workspace_sync::provide_workspace_sync(layout);
    provide_context(set_selected);
    provide_context(pan);
    provide_context(set_pan);
    provide_context(zoom);
    provide_context(set_zoom);
    let history = RwSignal::new(LayoutHistory::new());
    let camera_history = RwSignal::new(CameraHistory::new());
    provide_context(camera_history);
    let dragging = RwSignal::new(None::<DragState>);
    let resizing = RwSignal::new(None::<ResizeState>);
    let snap_guides = RwSignal::new(Vec::<SnapGuide>::new());
    let runtime = RwSignal::new(RuntimeState::Loading);

    let open_deep_link = move |hash: &str| {
        let Ok(subject) = cybou_protocol::SubjectRef::from_deep_link_hash(hash) else {
            return;
        };
        let inspector = living_canvas::CardId::Inspector(0);
        let inspector_state = tool_states.inspector(inspector);
        inspector_state.subject_query.set(None);
        inspector_state.target_subject.set(Some(subject));
        layout.update(|desktop| desktop.open_card(inspector, 380.0, 150.0));
        layout.get_untracked().save();
        set_selected.set(Some(living_canvas::DesktopItemId::Card(inspector)));
    };

    #[cfg(target_arch = "wasm32")]
    if let Some(window) = web_sys::window() {
        if let Ok(hash) = window.location().hash() {
            open_deep_link(&hash);
        }
        let listener_window = window.clone();
        let on_hash_change = Closure::<dyn FnMut()>::new(move || {
            if let Ok(hash) = listener_window.location().hash() {
                open_deep_link(&hash);
            }
        });
        let _ = window.add_event_listener_with_callback(
            "hashchange",
            on_hash_change.as_ref().unchecked_ref(),
        );
        let installed = StoredValue::new_local(Some((window, on_hash_change)));
        on_cleanup(move || {
            installed.update_value(|held| {
                if let Some((window, handler)) = held.take() {
                    let _ = window.remove_event_listener_with_callback(
                        "hashchange",
                        handler.as_ref().unchecked_ref(),
                    );
                }
            });
        });
    }

    // Editor contents are deliberately not copied into localStorage: that would turn private file
    // content into an ungoverned durable browser cache. Until user-scoped server draft persistence
    // exists, make navigation honest and interrupt accidental loss of browser-only buffers.
    #[cfg(target_arch = "wasm32")]
    if let Some(window) = web_sys::window() {
        let on_before_unload =
            Closure::<dyn FnMut(BeforeUnloadEvent)>::new(move |event: BeforeUnloadEvent| {
                if tool_states.has_unsaved_editor_buffers() {
                    event.prevent_default();
                    event.set_return_value("Unsaved CYBOU editor buffers will be lost.");
                }
            });
        let _ = window.add_event_listener_with_callback(
            "beforeunload",
            on_before_unload.as_ref().unchecked_ref(),
        );
        let installed = StoredValue::new_local(Some((window, on_before_unload)));
        on_cleanup(move || {
            installed.update_value(|held| {
                if let Some((window, handler)) = held.take() {
                    let _ = window.remove_event_listener_with_callback(
                        "beforeunload",
                        handler.as_ref().unchecked_ref(),
                    );
                }
            });
        });
    }

    // Initial gateway bootstrap
    spawn_local(async move {
        let client = GatewayMindClient;
        // The session is read first and on its own. On a surface that serves nothing until somebody
        // signs in, every other route refuses, and asking anyway turned a closed door into a
        // connection error.
        let session = match client.session().await {
            Ok(session) => session,
            Err(error) => {
                runtime.set(RuntimeState::Error(error.to_string()));
                return;
            }
        };
        if session.mode == SessionMode::SignInRequired {
            runtime.set(RuntimeState::SignInRequired);
            return;
        }
        let result = async {
            let snapshot = client.snapshot().await?;
            Ok::<_, ClientError>((session, snapshot))
        }
        .await;
        let mind = client.mind().await.ok();
        // Asked after the projection it describes. A delivery is recorded when it happens, so
        // reading this first would report the previous delivery as though it were this one.
        let disclosure = client.disclosure().await.ok();
        let insight = client.insight().await.ok();
        let agents = client.agents().await.ok();
        let actions = client.actions(None).await.ok();
        let agent_offers = client.agent_offers().await.ok();
        runtime.set(match result {
            Ok((session, snapshot)) => {
                let ready_state = RuntimeState::Ready {
                    mode: session.mode,
                    session,
                    snapshot,
                    mind,
                    disclosure,
                    insight,
                    agents,
                    actions,
                    agent_offers,
                };
                spawn_local(async move {
                    if let Ok(recovered) = client.drafts().await
                        && !recovered.drafts.is_empty()
                    {
                        let mut restored = Vec::with_capacity(recovered.drafts.len());
                        for draft in recovered.drafts {
                            let current_path = match &draft.base_location {
                                Some(cybou_protocol::LocationRef::SafeShellJail {
                                    path, ..
                                }) => Some(path.clone()),
                                _ => None,
                            };
                            let tab = if let Some(path) = current_path {
                                match client.read_text_file(&path).await {
                                    Ok(current) => {
                                        EditorTab::from_recovery_against_file(draft, current)
                                    }
                                    Err(_) => EditorTab::from_recovery(draft),
                                }
                            } else {
                                EditorTab::from_recovery(draft)
                            };
                            restored.push(tab);
                        }
                        tool_states.restore_drafts(restored);
                    }
                });
                ready_state
            }
            Err(error) => RuntimeState::Error(error.to_string()),
        });
    });

    // Managed SSE live stream subscription
    let subscription = StoredValue::new(DesktopRuntimeSubscription::subscribe(runtime));
    on_cleanup(move || {
        subscription.dispose();
    });

    // Global workspace keyboard shortcuts
    #[cfg(target_arch = "wasm32")]
    if let Some(window) = web_sys::window() {
        let listener_window = window.clone();
        let on_shortcut = Closure::<dyn FnMut(KeyboardEvent)>::new(move |event: KeyboardEvent| {
            if (event.ctrl_key() || event.meta_key()) && event.key().eq_ignore_ascii_case("k") {
                event.prevent_default();
                set_command_open.set(true);
                if let Some(input) = command_input.get() {
                    let _ = input.focus();
                }
                return;
            }

            // Keyboard hierarchy: do not intercept local editor, terminal, or text-input shortcuts
            if is_editing_target(&event) {
                return;
            }

            if (event.ctrl_key() || event.meta_key()) && event.key().eq_ignore_ascii_case("z") {
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
                        web_sys::window()
                            .and_then(|w| w.inner_width().ok())
                            .and_then(|v| v.as_f64())
                            .unwrap_or(1440.0),
                        web_sys::window()
                            .and_then(|w| w.inner_height().ok())
                            .and_then(|v| v.as_f64())
                            .unwrap_or(900.0),
                    );
                    let (z, (px, py)) = DesktopLayout::fit_to_viewport(bbox, w, h, 60.0);
                    set_zoom.set(z);
                    set_pan.set((px, py));
                } else {
                    set_zoom.set(1.0);
                    set_pan.set((0.0, 0.0));
                }
            } else if (event.ctrl_key() || event.meta_key())
                && (event.key() == "=" || event.key() == "+")
            {
                event.prevent_default();
                set_zoom.update(|z| *z = (*z + 0.1).min(2.0));
            } else if (event.ctrl_key() || event.meta_key())
                && (event.key() == "-" || event.key() == "_")
            {
                event.prevent_default();
                set_zoom.update(|z| *z = (*z - 0.1).max(0.4));
            } else if event.key() == "Escape" {
                // Focus mode takes a card out of the canvas and fills the window with it, and the
                // only way back out was the button that put you there — which is off screen the
                // moment the card scrolls. Escape leaves it, and failing that clears the
                // selection, which is what Escape means everywhere else.
                if matches!(view_mode.get_untracked(), DesktopViewMode::Focus(_)) {
                    event.prevent_default();
                    view_mode.set(DesktopViewMode::Spatial);
                } else if selected.get_untracked().is_some() {
                    event.prevent_default();
                    set_selected.set(None);
                }
            } else if event.alt_key() && event.key() == "ArrowLeft" {
                event.prevent_default();
                apply_camera_back(camera_history, pan, set_pan, zoom, set_zoom);
            } else if event.alt_key() && event.key() == "ArrowRight" {
                event.prevent_default();
                apply_camera_forward(camera_history, pan, set_pan, zoom, set_zoom);
            }
        });
        let _ = listener_window
            .add_event_listener_with_callback("keydown", on_shortcut.as_ref().unchecked_ref());
        // Removed rather than leaked. `forget()` is survivable for something that lives as long as
        // the page, and it is the wrong habit for a surface meant to be a system's own: a listener
        // nobody can take off keeps answering after whatever installed it is gone. The closure is
        // moved into the cleanup, which is what keeps it alive until there is nothing left to
        // detach it from.
        // `new_local` because a JS closure is neither `Send` nor `Sync` and never leaves this
        // thread; the handle itself is what the cleanup captures.
        let installed = StoredValue::new_local(Some((window, on_shortcut)));
        on_cleanup(move || {
            installed.update_value(|held| {
                if let Some((window, handler)) = held.take() {
                    let _ = window.remove_event_listener_with_callback(
                        "keydown",
                        handler.as_ref().unchecked_ref(),
                    );
                }
            });
        });
    }

    // Nothing but the way in, until there is a way in. The gateway refuses every projection in this
    // mode, so a desktop drawn here would be a frame around empty cards claiming a system is
    // unavailable when it is simply not being shown to a stranger.
    let must_sign_in = move || matches!(runtime.get(), RuntimeState::SignInRequired);

    view! {
        <Show when=must_sign_in>
            <SignInView />
        </Show>

        <Show when=move || !must_sign_in()>
        <main class="app-shell">
            <Topbar
                runtime=runtime
                auth_modal_open=auth_modal_open
                selected=selected
                set_selected=set_selected
                runtime_menu_open=runtime_menu_open
                set_runtime_menu_open=set_runtime_menu_open
                layout=layout
                history=history
                camera_history=camera_history
                pan=pan
                set_pan=set_pan
                zoom=zoom
                set_zoom=set_zoom
            />

            <CanvasViewport
                layout=layout
                history=history
                selected=selected
                set_selected=set_selected
                dragging=dragging
                resizing=resizing
                snap_guides=snap_guides
                auth_modal_open=auth_modal_open
                runtime=runtime
                zoom=zoom
                set_zoom=set_zoom
                pan=pan
                set_pan=set_pan
                panning=panning
                set_panning=set_panning
                camera_history=camera_history
            />

            <DesktopDock
                selected=selected
                set_selected=set_selected
                layout=layout
                auth_modal_open=auth_modal_open
                runtime=runtime
            />

            <section class="canvas-controls" aria-label="Canvas viewport navigation">
                <Show when=move || minimap_visible.get()>
                    <Minimap layout=layout zoom=zoom pan=pan set_pan=set_pan />
                </Show>

                <div class="zoom-controls">
                    <button
                        class="canvas-btn"
                        title="Zoom In (Ctrl +)"
                        aria-label="Zoom In"
                        on:click=move |_| set_zoom.update(|z| *z = (*z + 0.1).min(2.0))
                    >
                        "+"
                    </button>
                    <span class="zoom-label">{move || format!("{:.0}%", zoom.get() * 100.0)}</span>
                    <button
                        class="canvas-btn"
                        title="Zoom Out (Ctrl -)"
                        aria-label="Zoom Out"
                        on:click=move |_| set_zoom.update(|z| *z = (*z - 0.1).max(0.4))
                    >
                        "-"
                    </button>
                    <button
                        class="canvas-btn"
                        title="Fit All Cards (Ctrl 0)"
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
                        title="Remember this place"
                        aria-label="Remember this place"
                        on:click=move |_| {
                            // The centre of what is on screen, in canvas coordinates, and a name
                            // taken from what is standing there. Making an anchor used to mean
                            // leaving the place to go and describe it in another panel.
                            let view = living_canvas::interaction::visible_canvas_rect(
                                pan.get_untracked(),
                                zoom.get_untracked(),
                            );
                            let name = layout.get_untracked().name_for_view(view);
                            history.update(|h| h.push(layout.get_untracked()));
                            layout.update(|l| {
                                l.add_anchor(
                                    &name,
                                    view.x + view.width / 2.0,
                                    view.y + view.height / 2.0,
                                    zoom.get_untracked(),
                                );
                            });
                            layout.get_untracked().save();
                        }
                    >
                        <lucide_leptos::MapPin size=12 />
                    </button>
                    <button
                        class="canvas-btn"
                        class:active=move || magnet.get()
                        title=move || if magnet.get() {
                            "Sticking is on: cards align and merge into decks"
                        } else {
                            "Sticking is off: cards go where you put them"
                        }
                        aria-label="Toggle sticking"
                        aria-pressed=move || if magnet.get() { "true" } else { "false" }
                        on:click=move |_| magnet.update(|on| *on = !*on)
                    >
                        <lucide_leptos::Anchor size=12 />
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

            <CommandPalette
                layout=layout
                history=history
                set_selected=set_selected
                auth_modal_open=auth_modal_open
                command_open=command_open
                set_command_open=set_command_open
                command_query=command_query
                set_command_query=set_command_query
                command_input=command_input
                set_zoom=set_zoom
                set_pan=set_pan
                runtime=runtime
            />

            <AuthModal open=auth_modal_open />
        </main>
        </Show>
    }
}
