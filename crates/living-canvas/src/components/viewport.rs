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
    provide_context(crate::components::camera_context::CanvasCamera {
        pan,
        zoom,
        viewport: crate::components::camera_context::window_size(),
    });

    let view_mode = use_context::<RwSignal<DesktopViewMode>>()
        .unwrap_or_else(|| RwSignal::new(DesktopViewMode::Spatial));

    let is_lod_overview = move || zoom.get() <= 0.35;
    let is_lod_glance = move || zoom.get() > 0.35 && zoom.get() <= 0.75;
    let is_lod_standard = move || zoom.get() > 0.75 && zoom.get() <= 1.25;
    let is_lod_detail = move || zoom.get() > 1.25;

    view! {
        <section
            class="canvas"
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
                if matches!(view_mode.get(), DesktopViewMode::Focus(_)) {
                    "transform: none;".to_owned()
                } else {
                    format!(
                        "transform: translate3d({:.1}px, {:.1}px, 0) scale({:.3}); transform-origin: 0 0;",
                        pan.get().0, pan.get().1, zoom.get()
                    )
                }
            }
            on:wheel=move |event: WheelEvent| {
                if event.ctrl_key() || event.meta_key() {
                    event.prevent_default();
                    let delta = -event.delta_y() * 0.0015;
                    set_zoom.update(|z| *z = (*z + delta).clamp(0.4, 2.0));
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
                if let Some((_start_x, _start_y, init_px, init_py)) = panning.get() {
                    let cur_px = pan.get_untracked().0;
                    let cur_py = pan.get_untracked().1;
                    if (cur_px - init_px).abs() > 15.0 || (cur_py - init_py).abs() > 15.0 {
                        if let Some(ch) = camera_history {
                            ch.update(|h| h.record(crate::CameraState::new(init_px, init_py, zoom.get_untracked())));
                        }
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
                        SnapGuide::Vertical(x) => format!("v-{}", x),
                        SnapGuide::Horizontal(y) => format!("h-{}", y),
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
