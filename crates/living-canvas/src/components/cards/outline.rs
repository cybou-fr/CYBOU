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

    let focus_card = move |card_id: CardId| {
        if let Some(sel) = select_fn {
            sel.set(Some(DesktopItemId::Card(card_id)));
        }
    };

    let focus_deck = move |deck_id: String| {
        if let Some(sel) = select_fn {
            sel.set(Some(DesktopItemId::Deck(deck_id)));
        }
    };

    view! {
        <div class="outline-body" on:pointerdown=move |e: PointerEvent| e.stop_propagation()>
            <div class="outline-header">
                <IconLayers size=14 />
                <span class="outline-title">"Workspace Hierarchy"</span>
            </div>

            // Clusters Tree
            <div class="outline-section">
                <div class="outline-section-label">"Spatial Clusters"</div>
                <div class="outline-tree">
                    <For
                        each=move || layout_sig.get().clusters
                        key=|cluster| cluster.id.clone()
                        children=move |cluster| {
                            let member_count = cluster.card_keys.len();
                            let title = cluster.label.clone();
                            view! {
                                <div class="outline-item cluster">
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
                        key=|card| card.id.key().to_string()
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
