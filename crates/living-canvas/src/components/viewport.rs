// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Canvas viewport component handling spatial navigation, pan/zoom gestures, and card layers.

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{PointerEvent, WheelEvent};

use crate::{
    CardId, DesktopItemId, DesktopLayout, DesktopViewMode, LayoutHistory, SnapGuide,
    components::{
        cards::{
            AgentsCard, AttentionCard, BeliefsCard, CapabilitiesCard, CommitmentsCard, ContextCard,
            DiffCard, DisclosureCard, EditorCard, FileManagerCard, GenericToolCard, IdentityCard,
            InsightCard, InspectorCard, JournalCard, JournalFeedCard, LifecycleCard, OutlineCard,
            PerceptionCard, SelfModelCard, SessionCard, ShellCard,
        },
        deck::DeckContainerView,
        relations::RelationshipsLayer,
    },
    interaction::{DragState, ResizeState, finish_drag, finish_resize, move_drag, move_resize},
    layout::camera::{MAX_ZOOM, MIN_ZOOM},
    state::RuntimeState,
};

/// Main interactive Canvas Viewport with spatial cards, decks, and relationship connections.
#[component]
pub fn CanvasViewport(
    layout: RwSignal<DesktopLayout>,
    history: RwSignal<LayoutHistory>,
    selected: ReadSignal<Option<DesktopItemId>>,
    set_selected: WriteSignal<Option<DesktopItemId>>,
    dragging: RwSignal<Option<DragState>>,
    resizing: RwSignal<Option<ResizeState>>,
    snap_guides: RwSignal<Vec<SnapGuide>>,
    auth_modal_open: RwSignal<bool>,
    runtime: RwSignal<RuntimeState>,
    zoom: ReadSignal<f64>,
    set_zoom: WriteSignal<f64>,
    pan: ReadSignal<(f64, f64)>,
    set_pan: WriteSignal<(f64, f64)>,
    panning: ReadSignal<Option<(f64, f64, f64, f64)>>,
    set_panning: WriteSignal<Option<(f64, f64, f64, f64)>>,
    #[prop(optional)] camera_history: Option<RwSignal<crate::CameraHistory>>,
) -> impl IntoView {
    // Where the camera is, for cards deciding whether they are worth drawing. Provided here
    // because this is the component that owns the transform; read by `CardFrame`, which is as far
    // from here as it is possible to be while still being on this canvas.
    // Provided by `App` for the whole desktop. Measured here only when nothing above did, which
    // is every component test: a viewport that insisted on its own would give the cards inside it
    // a different window from the one the Dock beside them is reading.
    let camera =
        use_context::<crate::components::camera_context::CanvasCamera>().unwrap_or_else(|| {
            let camera = crate::components::camera_context::CanvasCamera {
                pan,
                zoom,
                viewport: crate::components::camera_context::window_size(),
            };
            provide_context(camera);
            camera
        });
    let measured = camera.viewport;

    // Below a certain width a panel is wider than the screen it is on, and a spatial desktop is
    // asking somebody to pan sideways to read a sentence. The panels become one column instead,
    // which is ADR-0044's cluster stack view at its simplest: not a smaller canvas, because pan and
    // zoom mean nothing when everything is already as wide as the window.
    let is_stacked = move || {
        crate::layout::camera::presentation_for(measured.get().0)
            == crate::layout::camera::Presentation::Stacked
    };

    // The two fingers of a pinch, by pointer id. A browser interleaves their moves, so both
    // positions have to be remembered as they were: the gesture is the change between frames, and
    // a frame that only knew where one finger is would read half of it as a pan.
    let touches: RwSignal<Vec<(i32, (f64, f64))>> = RwSignal::new(Vec::new());

    let view_mode = use_context::<RwSignal<DesktopViewMode>>()
        .unwrap_or_else(|| RwSignal::new(DesktopViewMode::Spatial));

    // Asked of `layout::camera`, which the card frame asks too. These were four inline
    // comparisons here and nowhere else, so the frame had no way to know what the stylesheet had
    // decided about the same zoom.
    let detail = move || crate::layout::camera::detail_at(zoom.get());
    let is_lod_overview = move || detail() == crate::layout::camera::Detail::Overview;
    let is_lod_glance = move || detail() == crate::layout::camera::Detail::Glance;
    let is_lod_standard = move || detail() == crate::layout::camera::Detail::Standard;
    let is_lod_detail = move || detail() == crate::layout::camera::Detail::Detail;

    view! {
        <section
            class="canvas"
            class:stacked=is_stacked
            id="canvas"
            class:lod-overview=is_lod_overview
            class:lod-glance=is_lod_glance
            class:lod-standard=is_lod_standard
            class:lod-detail=is_lod_detail
            // No transform while something is focused. A `position: fixed` element inside a
            // transformed ancestor is positioned against that ancestor and scaled with it, so a
            // focused card meant to fill the window was drawn at the canvas zoom and offset by the
            // pan — a large empty frame with its contents small in one corner. Focus means the same
            // thing at every zoom, which it cannot while the canvas is still scaling it.
            style=move || {
                if is_stacked() || matches!(view_mode.get(), DesktopViewMode::Focus(_)) {
                    "transform: none;".to_owned()
                } else {
                    format!(
                        "transform: translate3d({:.1}px, {:.1}px, 0) scale({:.3}); transform-origin: 0 0;",
                        pan.get().0, pan.get().1, zoom.get()
                    )
                }
            }
            // The wheel zooms, and it zooms at the pointer.
            //
            // It used to pan, with zoom behind Ctrl. That is the right way round for a document
            // and the wrong way round for a canvas: on a mouse, the wheel is the only continuous
            // control there is, and reaching for a modifier to change scale on a spatial desktop
            // makes scale feel like a setting rather than a way of looking.
            //
            // A trackpad still pans. Two fingers send a wheel event with a horizontal component or
            // a fractional vertical one; a mouse notch sends whole steps down one axis. The
            // difference is a heuristic and it is allowed to be, because being wrong costs a pan
            // where a zoom was meant and Shift is there for the other axis either way.
            on:wheel=move |event: WheelEvent| {
                event.prevent_default();
                let (dx, dy) = (event.delta_x(), event.delta_y());
                let from_a_mouse = dx == 0.0 && dy.abs() >= 20.0 && dy.fract() == 0.0;
                let zooming = event.ctrl_key() || event.meta_key() || from_a_mouse;

                if !zooming {
                    let (dx, dy) = if event.shift_key() { (dy, dx) } else { (dx, dy) };
                    set_pan.update(|(px, py)| {
                        *px -= dx * 0.8;
                        *py -= dy * 0.8;
                    });
                    return;
                }

                let before = zoom.get_untracked();
                let after = (before - dy * 0.0015).clamp(MIN_ZOOM, MAX_ZOOM);
                if (after - before).abs() < f64::EPSILON {
                    return;
                }

                // Keep whatever is under the pointer under the pointer. The stage is translated by
                // the pan and scaled about its own origin, so its box already sits at the pan: the
                // offset from its left edge is the only measurement this needs, and the correction
                // falls out as `pan + offset * (1 - after / before)`.
                let anchored = event
                    .current_target()
                    .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
                    .map(|element| {
                        let rect = element.get_bounding_client_rect();
                        (
                            f64::from(event.client_x()) - rect.left(),
                            f64::from(event.client_y()) - rect.top(),
                        )
                    });

                set_zoom.set(after);
                if let Some((offset_x, offset_y)) = anchored {
                    let scale = 1.0 - after / before;
                    set_pan.update(|(px, py)| {
                        *px += offset_x * scale;
                        *py += offset_y * scale;
                    });
                }
            }
            on:pointerdown=move |event: PointerEvent| {
                if event.pointer_type() == "touch" {
                    touches.update(|held| {
                        held.retain(|(id, _)| *id != event.pointer_id());
                        // Only ever two. A third finger on the canvas is somebody resting a hand,
                        // and letting it join the gesture would make the pinch jump.
                        if held.len() < 2 {
                            held.push((
                                event.pointer_id(),
                                (f64::from(event.client_x()), f64::from(event.client_y())),
                            ));
                        }
                    });
                }
                let is_canvas_bg = event.target()
                    .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                    .is_some_and(|el| el.class_list().contains("canvas") || el.class_list().contains("ambient") || el.tag_name().eq_ignore_ascii_case("svg"));
                if is_canvas_bg || event.button() == 1 {
                    set_panning.set(Some((f64::from(event.client_x()), f64::from(event.client_y()), pan.get().0, pan.get().1)));
                }
            }
            on:pointermove=move |event: PointerEvent| {
                // A pinch takes precedence over everything else on the canvas: while two fingers
                // are down the person is moving the camera, not dragging what is under them.
                if event.pointer_type() == "touch" && touches.get_untracked().len() == 2 {
                    let held = touches.get_untracked();
                    let current = (
                        f64::from(event.client_x()),
                        f64::from(event.client_y()),
                    );
                    let moved: Vec<(i32, (f64, f64))> = held
                        .iter()
                        .map(|(id, point)| {
                            if *id == event.pointer_id() {
                                (*id, current)
                            } else {
                                (*id, *point)
                            }
                        })
                        .collect();

                    let step = crate::layout::camera::pinch_step(
                        zoom.get_untracked(),
                        pan.get_untracked(),
                        (held[0].1, held[1].1),
                        (moved[0].1, moved[1].1),
                    );
                    set_zoom.set(step.zoom);
                    set_pan.set(step.pan);
                    touches.set(moved);
                    // A pinch that also panned would move the canvas twice.
                    set_panning.set(None);
                    return;
                }

                if let Some((start_x, start_y, init_px, init_py)) = panning.get() {
                    let cur_x = f64::from(event.client_x());
                    let cur_y = f64::from(event.client_y());
                    set_pan.set((init_px + (cur_x - start_x), init_py + (cur_y - start_y)));
                }
                move_drag(event.clone(), layout, dragging, snap_guides, zoom.get_untracked());
                move_resize(event, layout, resizing);
            }
            on:pointerup=move |event: PointerEvent| {
                touches.update(|held| held.retain(|(id, _)| *id != event.pointer_id()));
                if let Some((_start_x, _start_y, init_px, init_py)) = panning.get() {
                    let cur_px = pan.get_untracked().0;
                    let cur_py = pan.get_untracked().1;
                    if ((cur_px - init_px).abs() > 15.0 || (cur_py - init_py).abs() > 15.0)
                        && let Some(ch) = camera_history {
                            ch.update(|h| h.record(crate::CameraState::new(init_px, init_py, zoom.get_untracked())));
                        }
                }
                set_panning.set(None);
                finish_drag(layout, history, dragging, snap_guides);
                finish_resize(layout, resizing);
            }
        >
            <div class="ambient ambient-primary"></div>
            <div class="ambient ambient-secondary"></div>

            <RelationshipsLayer layout=layout selected=selected />

            <svg class="snap-guides-layer" aria-hidden="true">
                <For
                    each=move || snap_guides.get()
                    key=|guide| match guide {
                        SnapGuide::Vertical(x) => format!("v-{x}"),
                        SnapGuide::Horizontal(y) => format!("h-{y}"),
                    }
                    children=move |guide| {
                        match guide {
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

            <For
                each=move || layout.get().clusters
                key=|cluster| cluster.id.clone()
                children=move |cluster| {
                    let cluster_id = cluster.id.clone();
                    let color = cluster.color.clone();
                    let label = cluster.label.clone();
                    let get_rect = move || {
                        let current_layout = layout.get();
                        current_layout
                            .clusters
                            .iter()
                            .find(|c| c.id == cluster_id)
                            .and_then(|c| current_layout.cluster_rect(c))
                    };
                    view! {
                        {move || {
                            if let Some(r) = get_rect() {
                                let style = format!(
                                    "left: {}px; top: {}px; width: {}px; height: {}px;",
                                    r.x, r.y, r.width, r.height
                                );
                                view! {
                                    <div class=format!("canvas-cluster theme-{}", color) style=style>
                                        <div class="canvas-cluster-header">
                                            <span class="canvas-cluster-title">{label.clone()}</span>
                                        </div>
                                    </div>
                                }.into_any()
                            } else {
                                view! { <div/> }.into_any()
                            }
                        }}
                    }
                }
            />

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
            <DisclosureCard layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing runtime=runtime />
            <InsightCard layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing runtime=runtime />
            <AgentsCard layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing runtime=runtime />

            // One card per instance the layout holds, not one card per kind. `CardSpec` says these
            // are not singletons; rendering exactly one of each made that a promise the desktop
            // could not keep, so a second Shell could be opened into the model and never appear.
            <For
                each=move || shell_instances(&layout.get())
                key=|instance| *instance
                children=move |instance| view! {
                    <ShellCard layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing auth_modal_open=auth_modal_open runtime=runtime instance=instance />
                }
            />
            <For
                each=move || file_manager_instances(&layout.get())
                key=|instance| *instance
                children=move |instance| view! {
                    <FileManagerCard layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing auth_modal_open=auth_modal_open runtime=runtime instance=instance />
                }
            />
            <For
                each=move || journal_feed_instances(&layout.get())
                key=|instance| *instance
                children=move |instance| view! {
                    <JournalFeedCard layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing instance=instance />
                }
            />
            <For
                each=move || editor_instances(&layout.get())
                key=|instance| *instance
                children=move |instance| view! {
                    <EditorCard layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing auth_modal_open=auth_modal_open runtime=runtime instance=instance />
                }
            />
            <For
                each=move || diff_instances(&layout.get())
                key=|instance| *instance
                children=move |instance| view! {
                    <DiffCard layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing auth_modal_open=auth_modal_open runtime=runtime instance=instance />
                }
            />
            <For
                each=move || inspector_instances(&layout.get())
                key=|instance| *instance
                children=move |instance| view! {
                    <InspectorCard layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing auth_modal_open=auth_modal_open runtime=runtime instance=instance />
                }
            />
            // Everything the layout holds that nothing above has claimed. Without this a card
            // kind reachable from the Dock but missing a wrapper here was opened, selected, saved
            // and never drawn — and the same card tabbed into a Deck drew perfectly, because a
            // Deck has always rendered whatever `CardContent` can dispatch.
            <For
                each=move || unclaimed_cards(&layout.get())
                key=|card| *card
                children=move |card| view! {
                    <GenericToolCard card=card layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing auth_modal_open=auth_modal_open runtime=runtime />
                }
            />

            <Show when=move || outline_open(&layout.get())>
                <OutlineCard layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing _auth_modal_open=auth_modal_open _runtime=runtime />
            </Show>

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
                            auth_modal_open=auth_modal_open
                        />
                    }
                }
            />
        </section>
    }
}

/// The cards this layout holds that nothing else on this canvas draws.
///
/// Cards inside a Deck are excluded: the Deck draws them, and drawing them here as well would put
/// two of the same panel on the canvas, one of them outside the Deck that owns it.
fn unclaimed_cards(layout: &DesktopLayout) -> Vec<CardId> {
    layout
        .cards
        .iter()
        .map(|card| card.id)
        .filter(|card| !card.has_dedicated_view() && !layout.is_in_deck(*card))
        .collect()
}

/// The Shell cards this layout holds, by instance.
fn shell_instances(layout: &DesktopLayout) -> Vec<u32> {
    layout
        .cards
        .iter()
        .filter_map(|card| match card.id {
            CardId::Shell(instance) => Some(instance),
            _ => None,
        })
        .collect()
}

/// The File Manager cards this layout holds, by instance.
fn file_manager_instances(layout: &DesktopLayout) -> Vec<u32> {
    layout
        .cards
        .iter()
        .filter_map(|card| match card.id {
            CardId::FileManager(instance) => Some(instance),
            _ => None,
        })
        .collect()
}

/// The Text Editor cards this layout holds, by instance.
fn editor_instances(layout: &DesktopLayout) -> Vec<u32> {
    layout
        .cards
        .iter()
        .filter_map(|card| match card.id {
            CardId::Editor(instance) => Some(instance),
            _ => None,
        })
        .collect()
}

/// The Diff Viewer cards this layout holds, by instance.
fn diff_instances(layout: &DesktopLayout) -> Vec<u32> {
    layout
        .cards
        .iter()
        .filter_map(|card| match card.id {
            CardId::Diff(instance) => Some(instance),
            _ => None,
        })
        .collect()
}

/// The Universal Inspector cards this layout holds, by instance.
fn inspector_instances(layout: &DesktopLayout) -> Vec<u32> {
    layout
        .cards
        .iter()
        .filter_map(|card| match card.id {
            CardId::Inspector(instance) => Some(instance),
            _ => None,
        })
        .collect()
}

/// Whether the Canvas Outline card is currently in layout.
fn outline_open(layout: &DesktopLayout) -> bool {
    layout.cards.iter().any(|card| card.id == CardId::Outline)
}

/// The event-stream cards this layout holds, by instance.
fn journal_feed_instances(layout: &DesktopLayout) -> Vec<u32> {
    layout
        .cards
        .iter()
        .filter_map(|card| match card.id {
            CardId::JournalFeed(instance) => Some(instance),
            _ => None,
        })
        .collect()
}
