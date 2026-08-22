// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Canvas viewport component handling spatial navigation, pan/zoom gestures, and card layers.

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{PointerEvent, WheelEvent};

use crate::{
    CardId, DesktopItemId, DesktopLayout, LayoutHistory, SnapGuide,
    components::{
        cards::{
            AttentionCard, BeliefsCard, CapabilitiesCard, CommitmentsCard, ContextCard,
            DisclosureCard, FileManagerCard, IdentityCard, JournalCard, JournalFeedCard,
            LifecycleCard, PerceptionCard, SelfModelCard, SessionCard, ShellCard,
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
) -> impl IntoView {
    view! {
        <section
            class="canvas"
            id="canvas"
            style=move || format!(
                "transform: translate3d({:.1}px, {:.1}px, 0) scale({:.3}); transform-origin: 0 0;",
                pan.get().0, pan.get().1, zoom.get()
            )
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
            <DisclosureCard layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing runtime=runtime />

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
