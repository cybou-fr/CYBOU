// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Canvas Outline non-spatial accessibility tree view and navigator (ADR-0046 §22, §29).

use leptos::prelude::*;
use std::sync::Arc;
use web_sys::PointerEvent;

use crate::{
    CardId, DesktopItemId, DesktopLayout,
    components::{card_frame::CardFrame, icons::IconLayers},
    interaction::{DragState, ResizeState},
    state::RuntimeState,
};

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

    let pan = use_context::<ReadSignal<(f64, f64)>>();
    let set_pan = use_context::<WriteSignal<(f64, f64)>>();
    let zoom = use_context::<ReadSignal<f64>>();
    let set_zoom = use_context::<WriteSignal<f64>>();
    let camera_history = use_context::<RwSignal<crate::CameraHistory>>();

    let fly_to = move |cx: f64, cy: f64, target_zoom: f64| {
        if let (Some(p), Some(sp), Some(z), Some(sz)) = (pan, set_pan, zoom, set_zoom) {
            crate::apply_camera_fly_to(camera_history, p, sp, z, sz, cx, cy, target_zoom);
        }
    };

    let focus_card = move |card_id: CardId| {
        let current_layout = layout_sig.get_untracked();
        let geom = current_layout.geometry(card_id);
        let cx = geom.x + geom.width / 2.0;
        let cy = geom.y + geom.height / 2.0;
        fly_to(cx, cy, 1.0);
        if let Some(sel) = select_fn {
            sel.set(Some(DesktopItemId::Card(card_id)));
        }
    };

    let focus_deck = move |deck_id: String| {
        let current_layout = layout_sig.get_untracked();
        if let Some(deck) = current_layout.decks.iter().find(|d| d.id == deck_id) {
            let cx = deck.geometry.x + deck.geometry.width / 2.0;
            let cy = deck.geometry.y + deck.geometry.height / 2.0;
            fly_to(cx, cy, 1.0);
        }
        if let Some(sel) = select_fn {
            sel.set(Some(DesktopItemId::Deck(deck_id)));
        }
    };

    let focus_cluster = move |cluster_id: String| {
        let current_layout = layout_sig.get_untracked();
        if let Some(cluster) = current_layout.clusters.iter().find(|c| c.id == cluster_id) {
            if let Some(rect) = current_layout.cluster_rect(cluster) {
                let cx = rect.x + rect.width / 2.0;
                let cy = rect.y + rect.height / 2.0;
                fly_to(cx, cy, 0.85);
            }
        }
    };

    let focus_anchor = move |anchor_id: String| {
        let current_layout = layout_sig.get_untracked();
        if let Some(anchor) = current_layout.anchors.iter().find(|a| a.id == anchor_id) {
            fly_to(anchor.center_x, anchor.center_y, anchor.preferred_zoom);
        }
    };

    view! {
        <div class="outline-body" on:pointerdown=move |e: PointerEvent| e.stop_propagation()>
            <div class="outline-header">
                <IconLayers size=14 />
                <span class="outline-title">"Workspace Hierarchy"</span>
            </div>

            // Anchors Tree
            <div class="outline-section">
                <div class="outline-section-label">"Spatial Anchors"</div>
                <div class="outline-tree">
                    <For
                        each=move || layout_sig.get().anchors
                        key=|anchor| anchor.id.clone()
                        children=move |anchor| {
                            let aclick = anchor.id.clone();
                            let title = anchor.name.clone();
                            let (cx, cy) = (anchor.center_x, anchor.center_y);
                            view! {
                                <div class="outline-item anchor" on:click=move |_| focus_anchor(aclick.clone())>
                                    <span class="outline-icon">"⚓"</span>
                                    <span class="outline-name">{title}</span>
                                    <span class="outline-pos">{format!("({:.0}, {:.0})", cx, cy)}</span>
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
                        each=move || layout_sig.get().clusters
                        key=|cluster| cluster.id.clone()
                        children=move |cluster| {
                            let cclick = cluster.id.clone();
                            let member_count = cluster.card_keys.len();
                            let title = cluster.label.clone();
                            view! {
                                <div class="outline-item cluster" on:click=move |_| focus_cluster(cclick.clone())>
                                    <span class="outline-icon">"⊞"</span>
                                    <span class="outline-name">{title}</span>
                                    <span class="outline-badge">{format!("{member_count} cards")}</span>
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
                        each=move || layout_sig.get().decks
                        key=|deck| deck.id.clone()
                        children=move |deck| {
                            let did = deck.id.clone();
                            let dclick = deck.id.clone();
                            let card_count = deck.card_ids.len();
                            view! {
                                <div class="outline-item deck" on:click=move |_| focus_deck(dclick.clone())>
                                    <span class="outline-icon">"⎘"</span>
                                    <span class="outline-name">{format!("Deck [{}]", &did[..did.len().min(8)])}</span>
                                    <span class="outline-badge">{format!("{card_count} tabs")}</span>
                                </div>
                            }
                        }
                    />
                </div>
            </div>

            // Standalone Cards Tree
            <div class="outline-section">
                <div class="outline-section-label">"Standalone Cards"</div>
                <div class="outline-tree">
                    <For
                        each=move || layout_sig.get().cards
                        key=|card| card.id.instance_key()
                        children=move |card| {
                            let cid = card.id;
                            let title = card.id.title();
                            view! {
                                <div class="outline-item card" on:click=move |_| focus_card(cid)>
                                    <span class="outline-icon">"▪"</span>
                                    <span class="outline-name">{title}</span>
                                    <span class="outline-pos">
                                        {format!("({:.0}, {:.0})", card.geometry.x, card.geometry.y)}
                                    </span>
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
            kicker_icon=Arc::new(|| view! { <IconLayers size=14 /> }.into_any())
            collapsed_summary=Arc::new(collapsed)
        >
            <OutlineContent layout=layout set_selected=set_selected />
        </CardFrame>
    }
}
