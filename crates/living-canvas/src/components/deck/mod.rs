// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Deck container component managing tabbed card groupings.

pub mod controls;
pub mod frame;
pub mod tabs;

use leptos::prelude::*;
use web_sys::{KeyboardEvent, PointerEvent};

use crate::{
    CardId, DesktopLayout, DesktopViewMode, LayoutHistory,
    components::{
        card_controls::DeckResizeHandle,
        cards::content::CardContent,
        deck::{controls::DeckControls, frame::compute_deck_style, tabs::DeckTabs},
    },
    interaction::{DragState, ResizeState, keyboard_deck_move, start_deck_drag},
    state::RuntimeState,
};

/// Deck grouping container component.
#[component]
pub fn DeckContainerView(
    deck_id: String,
    layout: RwSignal<DesktopLayout>,
    history: RwSignal<LayoutHistory>,
    dragging: RwSignal<Option<DragState>>,
    resizing: RwSignal<Option<ResizeState>>,
    runtime: RwSignal<RuntimeState>,
    #[prop(optional)] auth_modal_open: Option<RwSignal<bool>>,
) -> impl IntoView {
    let d_id = StoredValue::new(deck_id);

    let deck_opt = Signal::derive(move || layout.get().deck(&d_id.get_value()).cloned());
    let is_collapsed =
        Signal::derive(move || deck_opt.get().is_some_and(|d| d.presentation.collapsed));
    let is_pinned = Signal::derive(move || deck_opt.get().is_some_and(|d| d.presentation.pinned));
    let active_card =
        Signal::derive(move || deck_opt.get().map_or(CardId::Identity, |d| d.active_card));
    let cards = Signal::derive(move || deck_opt.get().map_or_else(Vec::new, |d| d.card_ids));

    let is_magnet = Signal::derive(move || {
        let target_opt = dragging.get().and_then(|drag| drag.drop_target);
        target_opt.is_some_and(|target| cards.get().contains(&target))
    });

    let deck_style = Signal::derive(move || {
        let vm = use_context::<RwSignal<DesktopViewMode>>()
            .map_or(DesktopViewMode::Spatial, |v| v.get());
        compute_deck_style(&d_id.get_value(), layout.get(), vm)
    });

    let auth_signal = auth_modal_open.unwrap_or_else(|| RwSignal::new(false));

    view! {
        <div
            tabindex="0"
            role="region"
            aria-label="Deck container. Drag header to move, use arrow keys for keyboard placement."
            class="object deck-container"
            class:collapsed=is_collapsed
            class:pinned=is_pinned
            class:magnet-target=is_magnet
            style=deck_style
            on:keydown=move |event: KeyboardEvent| keyboard_deck_move(event, &d_id.get_value(), layout)
            on:click=move |_| {
                layout.update(|l| l.bring_deck_forward(&d_id.get_value()));
            }
        >
            <header
                class="object-header deck-header"
                on:pointerdown=move |event: PointerEvent| start_deck_drag(event, d_id.get_value(), layout, dragging)
            >
                <DeckTabs
                    deck_id=d_id.get_value()
                    layout=layout
                    history=history
                    active_card=active_card
                    cards=cards
                />
                <DeckControls
                    deck_id=d_id.get_value()
                    layout=layout
                    history=history
                />
            </header>

            <Show
                when=move || !is_collapsed.get()
                fallback=move || {
                    let cur_active = active_card.get();
                    let title = cur_active.title();
                    view! {
                        <div class="card-collapsed-summary">
                            <b>"Deck"</b>
                            <span>{title}</span>
                        </div>
                    }
                }
            >
                <div class="deck-body">
                    <CardContent
                        card=active_card.get()
                        runtime=runtime
                        auth_modal_open=auth_signal
                    />
                </div>
            </Show>

            <DeckResizeHandle deck_id=d_id.get_value() layout=layout resizing=resizing />
        </div>
    }
}
