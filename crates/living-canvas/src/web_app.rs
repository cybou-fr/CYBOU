// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

use leptos::prelude::*;
use leptos::task::spawn_local;
use living_canvas::{GatewayMindClient, MindClient};
use serde::{Deserialize, Serialize};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{EventSource, HtmlElement, KeyboardEvent, MessageEvent, PointerEvent};

const LAYOUT_KEY: &str = "cybou.living-canvas.layout.v1";

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct Point {
    x: f64,
    y: f64,
    z: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct CanvasLayout {
    artifact: Point,
    release: Point,
    suggestion: Point,
}

impl Default for CanvasLayout {
    fn default() -> Self {
        Self {
            artifact: Point {
                x: 165.0,
                y: 90.0,
                z: 1,
            },
            release: Point {
                x: 480.0,
                y: 140.0,
                z: 3,
            },
            suggestion: Point {
                x: 900.0,
                y: 360.0,
                z: 2,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Panel {
    Artifact,
    Release,
    Suggestion,
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
            Panel::Release => self.release,
            Panel::Suggestion => self.suggestion,
        }
    }

    fn set_point(&mut self, panel: Panel, point: Point) {
        match panel {
            Panel::Artifact => self.artifact = point,
            Panel::Release => self.release = point,
            Panel::Suggestion => self.suggestion = point,
        }
    }

    fn bring_forward(&mut self, panel: Panel) {
        let next = self.artifact.z.max(self.release.z).max(self.suggestion.z) + 1;
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
pub fn App() -> impl IntoView {
    let (selected, set_selected) = signal("release");
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
                    <span class="brand-mark" aria-hidden="true">"◌"</span>
                    <span>"Living Canvas"</span>
                </a>
                <p class="path">"Cybou Workspace / Programs / Cybou 0.8 release"</p>
                <div class="runtime" aria-label="Runtime connection" aria-live="polite">
                    <span class="status-dot" aria-hidden="true"></span>
                    <strong>{runtime_label}</strong>
                    <small>{projection_label}</small>
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
                <button
                    class:selected=move || selected.get() == "artifact"
                    class="object artifact"
                    style=move || panel_style(layout.get(), Panel::Artifact)
                    aria-label="Artifact panel. Drag to reposition; use arrow keys for keyboard movement."
                    on:pointerdown=move |event| start_drag(event, Panel::Artifact, layout, dragging)
                    on:keydown=move |event| keyboard_move(event, Panel::Artifact, layout)
                    on:click=move |_| set_selected.set("artifact")
                >
                    <small>"Artifact"</small>
                    <strong>"Release evidence"</strong>
                    <span>"12 verified sources"</span>
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
                    <small>"Release plan"</small>
                    <h1>"Cybou 0.8 release"</h1>
                    <p>"Stable release with local-first guarantees, improved reliability, and rollback safety."</p>
                    <div class="progress-label"><span>"Progress"</span><strong>"68%"</strong></div>
                    <div class="progress" aria-label="Release progress 68 percent"><span></span></div>
                    <footer><span>"Target · May 30"</span><span class="nominal">"On track"</span></footer>
                </button>

                <button
                    class:selected=move || selected.get() == "suggestion"
                    class="object suggestion"
                    style=move || panel_style(layout.get(), Panel::Suggestion)
                    aria-label="Mind suggestion panel. Drag to reposition; use arrow keys for keyboard movement."
                    on:pointerdown=move |event| start_drag(event, Panel::Suggestion, layout, dragging)
                    on:keydown=move |event| keyboard_move(event, Panel::Suggestion, layout)
                    on:click=move |_| set_selected.set("suggestion")
                >
                    <small>"Mind suggestion"</small>
                    <strong>"Verify rollback path"</strong>
                    <span>"Proposed · not authorized"</span>
                </button>

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
