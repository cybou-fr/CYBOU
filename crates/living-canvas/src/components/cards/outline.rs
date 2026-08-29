// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Canvas Outline non-spatial accessibility tree view and navigator (ADR-0046 §22, §29).

use leptos::prelude::*;
use lucide_leptos::{Anchor, Box, Layers, Maximize2, Minimize2, Search, Split, Trash2, X};
use std::sync::Arc;
use web_sys::{MouseEvent, PointerEvent};

use crate::{
    CardId, DesktopItemId, DesktopLayout, Rect,
    components::{card_controls::request_close_card, card_frame::CardFrame},
    interaction::{DragState, ResizeState},
    state::RuntimeState,
    tool_state::ToolCardStates,
};

fn viewport_dimensions() -> (f64, f64) {
    web_sys::window()
        .map(|window| {
            (
                window
                    .inner_width()
                    .ok()
                    .and_then(|value| value.as_f64())
                    .unwrap_or(1440.0),
                window
                    .inner_height()
                    .ok()
                    .and_then(|value| value.as_f64())
                    .unwrap_or(900.0),
            )
        })
        .unwrap_or((1440.0, 900.0))
}

/// Canvas Outline content component listing clusters, cards, and anchors hierarchically.
#[component]
pub fn OutlineContent(
    #[prop(optional)] layout: Option<RwSignal<DesktopLayout>>,
    #[prop(optional)] set_selected: Option<WriteSignal<Option<DesktopItemId>>>,
) -> impl IntoView {
    let layout_sig = layout
        .or_else(use_context::<RwSignal<DesktopLayout>>)
        .unwrap_or_else(|| RwSignal::new(DesktopLayout::default()));
    let select_fn = set_selected.or_else(use_context::<WriteSignal<Option<DesktopItemId>>>);
    let tool_states = expect_context::<ToolCardStates>();

    let pan = use_context::<ReadSignal<(f64, f64)>>();
    let set_pan = use_context::<WriteSignal<(f64, f64)>>();
    let zoom = use_context::<ReadSignal<f64>>();
    let set_zoom = use_context::<WriteSignal<f64>>();
    let camera_history = use_context::<RwSignal<crate::CameraHistory>>();

    let filter_query = RwSignal::new(String::new());
    let anchor_name = RwSignal::new(String::new());
    let anchor_error = RwSignal::new(None::<String>);

    let fly_to = move |cx: f64, cy: f64, target_zoom: f64| {
        if let (Some(p), Some(sp), Some(z), Some(sz)) = (pan, set_pan, zoom, set_zoom) {
            crate::apply_camera_fly_to(camera_history, p, sp, z, sz, cx, cy, target_zoom);
        }
    };

    let fit_and_fly = move |rect: Rect, maximum_zoom: f64| {
        let viewport = viewport_dimensions();
        let (target_zoom, _) = DesktopLayout::fit_to_viewport(rect, viewport.0, viewport.1, 60.0);
        fly_to(
            rect.x + rect.width / 2.0,
            rect.y + rect.height / 2.0,
            target_zoom.min(maximum_zoom),
        );
    };

    let focus_card = move |card_id: CardId| {
        let current_layout = layout_sig.get_untracked();
        if current_layout.presentation(card_id).collapsed {
            layout_sig.update(|l| l.set_collapsed(card_id, false));
        }
        layout_sig.update(|l| l.bring_forward(card_id));
        layout_sig.get_untracked().save();

        let geom = current_layout.geometry(card_id);
        fit_and_fly(Rect::new(geom.x, geom.y, geom.width, geom.height), 1.0);
        if let Some(sel) = select_fn {
            sel.set(Some(DesktopItemId::Card(card_id)));
        }
    };

    let toggle_card_collapsed = move |card_id: CardId, e: MouseEvent| {
        e.stop_propagation();
        layout_sig.update(|l| {
            let is_collapsed = l.presentation(card_id).collapsed;
            l.set_collapsed(card_id, !is_collapsed);
        });
        layout_sig.get_untracked().save();
    };

    let close_card = move |card_id: CardId, e: MouseEvent| {
        e.stop_propagation();
        request_close_card(card_id, layout_sig, tool_states);
    };

    let focus_deck = move |deck_id: String| {
        let current_layout = layout_sig.get_untracked();
        if let Some(deck) = current_layout.decks.iter().find(|d| d.id == deck_id) {
            fit_and_fly(
                Rect::new(
                    deck.geometry.x,
                    deck.geometry.y,
                    deck.geometry.width,
                    deck.geometry.height,
                ),
                1.0,
            );
        }
        if let Some(sel) = select_fn {
            sel.set(Some(DesktopItemId::Deck(deck_id)));
        }
    };

    let split_deck = move |deck_id: String, e: MouseEvent| {
        e.stop_propagation();
        layout_sig.update(|l| {
            l.dissolve_deck(&deck_id);
        });
        layout_sig.get_untracked().save();
    };

    let focus_cluster = move |cluster_id: String| {
        let current_layout = layout_sig.get_untracked();
        if let Some(cluster) = current_layout.clusters.iter().find(|c| c.id == cluster_id) {
            if let Some(rect) = current_layout.cluster_rect(cluster) {
                fit_and_fly(rect, 1.2);
            }
        }
    };

    let focus_anchor = move |anchor_id: String| {
        let current_layout = layout_sig.get_untracked();
        if let Some(anchor) = current_layout.anchors.iter().find(|a| a.id == anchor_id) {
            fly_to(anchor.center_x, anchor.center_y, anchor.preferred_zoom);
        }
    };

    let save_current_view = move |_| {
        let Some(pan) = pan else {
            anchor_error.set(Some("Camera is unavailable".to_owned()));
            return;
        };
        let Some(zoom) = zoom else {
            anchor_error.set(Some("Camera is unavailable".to_owned()));
            return;
        };
        let viewport = viewport_dimensions();
        let center = crate::camera_center(pan.get_untracked(), zoom.get_untracked(), viewport);
        let name = anchor_name.get_untracked();
        let mut added = false;
        layout_sig.update(|layout| {
            added = layout.add_anchor(&name, center.0, center.1, zoom.get_untracked());
            if added {
                layout.save();
            }
        });
        if added {
            anchor_name.set(String::new());
            anchor_error.set(None);
        } else {
            anchor_error.set(Some("Use a non-empty, unique name".to_owned()));
        }
    };

    let cards_count = move || layout_sig.get().cards.len();
    let decks_count = move || layout_sig.get().decks.len();
    let anchors_count = move || layout_sig.get().anchors.len();

    let matches_filter = move |text: &str| {
        let q = filter_query.get().to_lowercase();
        q.is_empty() || text.to_lowercase().contains(&q)
    };

    view! {
        <div class="outline-body" on:pointerdown=move |e: PointerEvent| e.stop_propagation()>
            // Workspace Summary Header
            <div class="outline-header">
                <div class="outline-header-left">
                    <Layers size=14 />
                    <span class="outline-title">"Workspace Outline"</span>
                </div>
                <div class="outline-stats-bar">
                    <span class="outline-stat-chip">{move || format!("{} cards", cards_count())}</span>
                    <span class="outline-stat-chip">{move || format!("{} decks", decks_count())}</span>
                    <span class="outline-stat-chip">{move || format!("{} anchors", anchors_count())}</span>
                </div>
            </div>

            // Search & Filter Bar
            <div class="outline-search-row">
                <Search size=13 />
                <input
                    type="text"
                    class="outline-search-input"
                    placeholder="Filter workspace items…"
                    prop:value=move || filter_query.get()
                    on:input=move |e| filter_query.set(event_target_value(&e))
                />
                <Show when=move || !filter_query.get().is_empty()>
                    <button
                        type="button"
                        class="outline-clear-btn"
                        title="Clear filter"
                        on:click=move |_| filter_query.set(String::new())
                    >
                        <X size=12 />
                    </button>
                </Show>
            </div>

            // Standalone Cards Tree
            <div class="outline-section">
                <div class="outline-section-label">"Cards"</div>
                <div class="outline-tree">
                    <For
                        each=move || {
                            let cards = layout_sig.get().cards;
                            cards.into_iter().filter(|c| matches_filter(&c.id.title()) || matches_filter(c.id.key())).collect::<Vec<_>>()
                        }
                        key=|card| card.id.instance_key()
                        children=move |card| {
                            let cid = card.id;
                            let title = card.id.title();
                            let is_collapsed = move || layout_sig.get().presentation(cid).collapsed;
                            let (gx, gy) = (card.geometry.x, card.geometry.y);
                            view! {
                                <div class="outline-item card" on:click=move |_| focus_card(cid)>
                                    <span class="outline-icon"><Box size=13 /></span>
                                    <span class="outline-name">{title}</span>
                                    <span class="outline-pos">{format!("({:.0}, {:.0})", gx, gy)}</span>
                                    <div class="outline-card-actions">
                                        <button
                                            type="button"
                                            class="outline-action-btn"
                                            title=move || if is_collapsed() { "Expand card" } else { "Collapse card" }
                                            on:click=move |e| toggle_card_collapsed(cid, e)
                                        >
                                            {move || if is_collapsed() {
                                                view! { <Maximize2 size=11 /> }.into_any()
                                            } else {
                                                view! { <Minimize2 size=11 /> }.into_any()
                                            }}
                                        </button>
                                        <button
                                            type="button"
                                            class="outline-action-btn danger"
                                            title="Close card"
                                            on:click=move |e| close_card(cid, e)
                                        >
                                            <X size=11 />
                                        </button>
                                    </div>
                                </div>
                            }
                        }
                    />
                </div>
            </div>

            // Decks Tree
            <div class="outline-section">
                <div class="outline-section-label">"Composed Decks"</div>
                <div class="outline-tree">
                    <For
                        each=move || {
                            let decks = layout_sig.get().decks;
                            decks.into_iter().filter(|d| matches_filter(&d.title) || matches_filter(&d.id)).collect::<Vec<_>>()
                        }
                        key=|deck| deck.id.clone()
                        children=move |deck| {
                            let did = deck.id.clone();
                            let dclick = deck.id.clone();
                            let dsplit = deck.id.clone();
                            let card_count = deck.card_ids.len();
                            let title = deck.title.clone();
                            view! {
                                <div class="outline-item deck" on:click=move |_| focus_deck(dclick.clone())>
                                    <span class="outline-icon"><Layers size=13 /></span>
                                    <span class="outline-name">{format!("{title} [{}]", &did[..did.len().min(6)])}</span>
                                    <span class="outline-badge">{format!("{card_count} tabs")}</span>
                                    <button
                                        type="button"
                                        class="outline-action-btn split"
                                        title="Split deck into separate cards"
                                        on:click=move |e| split_deck(dsplit.clone(), e)
                                    >
                                        <Split size=11 />
                                        <span>"Split"</span>
                                    </button>
                                </div>
                            }
                        }
                    />
                </div>
            </div>

            // Clusters Tree
            <div class="outline-section">
                <div class="outline-section-label">"Spatial Clusters"</div>
                <div class="outline-tree">
                    <For
                        each=move || {
                            let clusters = layout_sig.get().clusters;
                            clusters.into_iter().filter(|c| matches_filter(&c.label) || matches_filter(&c.id)).collect::<Vec<_>>()
                        }
                        key=|cluster| cluster.id.clone()
                        children=move |cluster| {
                            let cclick = cluster.id.clone();
                            let member_count = cluster.card_keys.len();
                            let title = cluster.label.clone();
                            view! {
                                <div class="outline-item cluster" on:click=move |_| focus_cluster(cclick.clone())>
                                    <span class="outline-icon"><Layers size=13 /></span>
                                    <span class="outline-name">{title}</span>
                                    <span class="outline-badge">{format!("{member_count} cards")}</span>
                                </div>
                            }
                        }
                    />
                </div>
            </div>

            // Anchors Tree
            <div class="outline-section">
                <div class="outline-section-label">"Spatial Anchors"</div>
                <div class="outline-anchor-create">
                    <input
                        class="outline-anchor-input"
                        aria-label="New anchor name"
                        placeholder="Anchor name"
                        prop:value=move || anchor_name.get()
                        on:input=move |event| {
                            anchor_name.set(event_target_value(&event));
                            anchor_error.set(None);
                        }
                    />
                    <button
                        class="outline-anchor-add"
                        type="button"
                        title="Save current viewport as anchor"
                        on:click=save_current_view
                    >
                        "+"
                    </button>
                </div>
                <Show when=move || anchor_error.get().is_some()>
                    <div class="outline-anchor-error" role="alert">
                        {move || anchor_error.get().unwrap_or_default()}
                    </div>
                </Show>
                <div class="outline-tree">
                    <For
                        each=move || {
                            let anchors = layout_sig.get().anchors;
                            anchors.into_iter().filter(|a| matches_filter(&a.name) || matches_filter(&a.id)).collect::<Vec<_>>()
                        }
                        key=|anchor| anchor.id.clone()
                        children=move |anchor| {
                            let aclick = anchor.id.clone();
                            let title = anchor.name.clone();
                            let rename_id = anchor.id.clone();
                            let rename_title = anchor.name.clone();
                            let delete_id = anchor.id.clone();
                            let (cx, cy) = (anchor.center_x, anchor.center_y);
                            view! {
                                <div class="outline-item anchor" on:click=move |_| focus_anchor(aclick.clone())>
                                    <span class="outline-icon"><Anchor size=13 /></span>
                                    <span class="outline-name">{title}</span>
                                    <span class="outline-pos">{format!("({:.0}, {:.0})", cx, cy)}</span>
                                    <button
                                        class="outline-anchor-action"
                                        type="button"
                                        title="Rename anchor"
                                        aria-label="Rename anchor"
                                        on:click=move |event: MouseEvent| {
                                            event.stop_propagation();
                                            let proposed = web_sys::window()
                                                .and_then(|window| {
                                                    window
                                                        .prompt_with_message_and_default(
                                                            "Rename anchor",
                                                            &rename_title,
                                                        )
                                                        .ok()
                                                        .flatten()
                                                });
                                            if let Some(name) = proposed {
                                                let mut renamed = false;
                                                layout_sig.update(|layout| {
                                                    renamed = layout.rename_anchor(&rename_id, &name);
                                                    if renamed {
                                                        layout.save();
                                                    }
                                                });
                                                anchor_error.set((!renamed).then(|| {
                                                    "Use a non-empty, unique name".to_owned()
                                                }));
                                            }
                                        }
                                    >
                                        "✎"
                                    </button>
                                    <button
                                        class="outline-anchor-action danger"
                                        type="button"
                                        title="Delete anchor"
                                        aria-label="Delete anchor"
                                        on:click=move |event: MouseEvent| {
                                            event.stop_propagation();
                                            layout_sig.update(|layout| {
                                                if layout.remove_anchor(&delete_id) {
                                                    layout.save();
                                                }
                                            });
                                            anchor_error.set(None);
                                        }
                                    >
                                        <Trash2 size=11 />
                                    </button>
                                </div>
                            }
                        }
                    />
                </div>
            </div>
        </div>
    }
}

/// Canvas Outline standalone tool card component.
#[component]
pub fn OutlineCard(
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<Option<DesktopItemId>>,
    set_selected: WriteSignal<Option<DesktopItemId>>,
    dragging: RwSignal<Option<DragState>>,
    resizing: RwSignal<Option<ResizeState>>,
    _auth_modal_open: RwSignal<bool>,
    _runtime: RwSignal<RuntimeState>,
) -> impl IntoView {
    let card_id = CardId::Outline;

    let collapsed = move || {
        view! {
            <div class="card-collapsed-summary">
                <b>"Outline"</b>
                <span>"Hierarchy"</span>
            </div>
        }
        .into_any()
    };

    view! {
        <CardFrame
            card=card_id
            layout=layout
            selected=selected
            set_selected=set_selected
            dragging=dragging
            resizing=resizing
            kicker_title="Canvas Outline"
            kicker_icon=Arc::new(|| view! { <Layers size=14 /> }.into_any())
            collapsed_summary=Arc::new(collapsed)
        >
            <OutlineContent layout=layout set_selected=set_selected />
        </CardFrame>
    }
}
