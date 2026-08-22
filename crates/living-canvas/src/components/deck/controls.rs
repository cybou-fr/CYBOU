// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Deck header controls: pin, collapse, focus, and dissolve.

use leptos::prelude::*;
use web_sys::PointerEvent;

use crate::{
    DesktopItemId, DesktopLayout, DesktopViewMode, LayoutHistory,
    components::icons::{IconLayers, IconMaximize, IconMinimize, IconPin},
};

/// Controls toolbar for a deck container.
#[component]
pub fn DeckControls(
    deck_id: String,
    layout: RwSignal<DesktopLayout>,
    history: RwSignal<LayoutHistory>,
) -> impl IntoView {
    let d_id = StoredValue::new(deck_id);
    let view_mode = use_context::<RwSignal<DesktopViewMode>>();

    let is_collapsed = Signal::derive(move || {
        layout
            .get()
            .deck(&d_id.get_value())
            .is_some_and(|d| d.presentation.collapsed)
    });
    let is_pinned = Signal::derive(move || {
        layout
            .get()
            .deck(&d_id.get_value())
            .is_some_and(|d| d.presentation.pinned)
    });
    let is_focused = Signal::derive(move || {
        let current_vm = view_mode.map_or(DesktopViewMode::Spatial, |v| v.get());
        current_vm == DesktopViewMode::Focus(DesktopItemId::Deck(d_id.get_value()))
    });

    view! {
        <div class="card-controls deck-controls" on:pointerdown=move |e: PointerEvent| e.stop_propagation()>
            <button
                class:active=is_pinned
                class="control-btn"
                title="Pin deck"
                aria-label="Pin deck"
                on:click=move |_| {
                    history.update(|h| h.push(layout.get_untracked()));
                    layout.update(|l| l.toggle_deck_pinned(&d_id.get_value()));
                    layout.get_untracked().save();
                }
            >
                <IconPin size=12 />
            </button>
            <button
                class:active=is_collapsed
                class="control-btn"
                title="Collapse deck"
                aria-label="Collapse deck"
                on:click=move |_| {
                    history.update(|h| h.push(layout.get_untracked()));
                    layout.update(|l| l.toggle_deck_collapse(&d_id.get_value()));
                    layout.get_untracked().save();
                }
            >
                <IconMinimize size=12 />
            </button>
            <button
                class:active=is_focused
                class="control-btn"
                title="Focus mode"
                aria-label="Toggle Deck Focus mode"
                on:click=move |_| {
                    if let Some(vm) = view_mode {
                        vm.update(|mode| {
                            *mode = if *mode == DesktopViewMode::Focus(DesktopItemId::Deck(d_id.get_value())) {
                                DesktopViewMode::Spatial
                            } else {
                                DesktopViewMode::Focus(DesktopItemId::Deck(d_id.get_value()))
                            };
                        });
                    }
                }
            >
                <IconMaximize size=12 />
            </button>
            <button
                class="control-btn close-btn"
                title="Dissolve deck into individual cards"
                aria-label="Dissolve deck"
                on:click=move |_| {
                    history.update(|h| h.push(layout.get_untracked()));
                    layout.update(|l| l.dissolve_deck(&d_id.get_value()));
                    layout.get_untracked().save();
                }
            >
                <IconLayers size=12 />
            </button>
        </div>
    }
}
