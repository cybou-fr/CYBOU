// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Deck tab navigation and card detachment bar.

use leptos::prelude::*;
use web_sys::{KeyboardEvent, PointerEvent};

use crate::{
    CardId, DesktopLayout, LayoutHistory,
    components::icons::{IconExternalLink, IconLayers},
};

/// Tab navigation bar for a deck container.
#[component]
pub fn DeckTabs(
    deck_id: String,
    layout: RwSignal<DesktopLayout>,
    history: RwSignal<LayoutHistory>,
    active_card: Signal<CardId>,
    cards: Signal<Vec<CardId>>,
) -> impl IntoView {
    let d_id = StoredValue::new(deck_id);

    view! {
        <div class="deck-tabs-header">
            <span class="deck-brand">
                <IconLayers size=13 />
                <span class="deck-title">"Deck"</span>
            </span>
            <div
                class="deck-tab-bar"
                role="tablist"
                aria-label="Deck tabs"
                on:pointerdown=move |e: PointerEvent| e.stop_propagation()
            >
                <For
                    each=move || cards.get()
                    key=|c| c.key().to_string()
                    children=move |c| {
                        let is_active = move || active_card.get() == c;
                        let card_label = c.title();
                        view! {
                            <button
                                role="tab"
                                aria-selected=move || is_active().to_string()
                                class="deck-tab"
                                class:active=is_active
                                on:click=move |_| {
                                    layout.update(|l| {
                                        if let Some(d) = l.deck_mut(&d_id.get_value()) {
                                            d.active_card = c;
                                        }
                                    });
                                    layout.get_untracked().save();
                                }
                                on:keydown=move |e: KeyboardEvent| {
                                    let key = e.key();
                                    let current_cards = cards.get();
                                    if let Some(idx) = current_cards.iter().position(|&x| x == c) {
                                        if key == "ArrowRight" {
                                            e.prevent_default();
                                            let next_idx = (idx + 1) % current_cards.len();
                                            if let Some(&next_c) = current_cards.get(next_idx) {
                                                layout.update(|l| {
                                                    if let Some(d) = l.deck_mut(&d_id.get_value()) {
                                                        d.active_card = next_c;
                                                    }
                                                });
                                            }
                                        } else if key == "ArrowLeft" {
                                            e.prevent_default();
                                            let prev_idx = if idx == 0 { current_cards.len() - 1 } else { idx - 1 };
                                            if let Some(&prev_c) = current_cards.get(prev_idx) {
                                                layout.update(|l| {
                                                    if let Some(d) = l.deck_mut(&d_id.get_value()) {
                                                        d.active_card = prev_c;
                                                    }
                                                });
                                            }
                                        }
                                    }
                                }
                            >
                                <span>{card_label}</span>
                            </button>
                        }
                    }
                />
            </div>
            <button
                class="deck-detach-btn"
                title="Detach active card into standalone spatial card"
                aria-label="Detach card"
                on:pointerdown=move |e: PointerEvent| e.stop_propagation()
                on:click=move |_| {
                    history.update(|h| h.push(layout.get_untracked()));
                    let cur_c = active_card.get();
                    layout.update(|l| {
                        l.detach_from_deck(&d_id.get_value(), cur_c, None);
                    });
                    layout.get_untracked().save();
                }
            >
                <IconExternalLink size=12 />
            </button>
        </div>
    }
}
