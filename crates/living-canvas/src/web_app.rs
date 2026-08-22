// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! High-level Living Canvas desktop workspace application coordinator.

use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::KeyboardEvent;

use living_canvas::{
    ClientError, DesktopLayout, DesktopViewMode, GatewayMindClient, LayoutHistory, MindClient,
    SnapGuide,
    components::{
        AuthModal, CanvasViewport, CommandPalette, DesktopDock, IconGrid, IconMaximize, Minimap,
        Topbar,
    },
    interaction::{DragState, ResizeState, apply_redo, apply_undo, selection_actions_style},
    state::{DesktopRuntimeSubscription, RuntimeState},
};

fn load_layout() -> DesktopLayout {
    DesktopLayout::load()
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

    // Initial gateway bootstrap
    spawn_local(async move {
        let client = GatewayMindClient;
        let result = async {
            let session = client.session().await?;
            let snapshot = client.snapshot().await?;
            Ok::<_, ClientError>((session, snapshot))
        }
        .await;
        let mind = client.mind().await.ok();
        // Asked after the projection it describes. A delivery is recorded when it happens, so
        // reading this first would report the previous delivery as though it were this one.
        let disclosure = client.disclosure().await.ok();
        runtime.set(match result {
            Ok((session, snapshot)) => RuntimeState::Ready {
                mode: session.mode,
                session,
                snapshot,
                mind,
                disclosure,
            },
            Err(error) => RuntimeState::Error(error.to_string()),
        });
    });

    // Managed SSE live stream subscription
    let subscription = StoredValue::new(DesktopRuntimeSubscription::subscribe(runtime));
    on_cleanup(move || {
        drop(subscription);
    });

    // Global workspace keyboard shortcuts
    #[cfg(target_arch = "wasm32")]
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
            }
        });
        let _ = window
            .add_event_listener_with_callback("keydown", on_shortcut.as_ref().unchecked_ref());
        on_shortcut.forget();
    }

    view! {
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
            />

            <DesktopDock
                selected=selected
                set_selected=set_selected
                layout=layout
                auth_modal_open=auth_modal_open
                runtime=runtime
            />

            <div class="selection-actions" style=move || selection_actions_style(layout.get())>
                <button
                    class="action-btn"
                    title="Bring forward in Z-order"
                    aria-label="Bring forward"
                    on:click=move |_| {
                        if let Some(card_id) = living_canvas::CardId::from_key(selected.get()) {
                            layout.update(|l| l.bring_forward(card_id));
                            layout.get_untracked().save();
                        }
                    }
                >
                    <IconMaximize size=12 />
                </button>
            </div>

            <section class="canvas-controls" aria-label="Canvas viewport navigation">
                <Show when=move || minimap_visible.get()>
                    <Minimap layout=layout zoom=zoom set_pan=set_pan />
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
            />

            <AuthModal open=auth_modal_open />
        </main>
    }
}
