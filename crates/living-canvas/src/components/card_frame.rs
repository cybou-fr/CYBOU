// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Unified generic CardFrame container encapsulating window mechanics, dragging, and resizing.

use leptos::prelude::*;
use std::sync::Arc;
use web_sys::{KeyboardEvent, PointerEvent};

use crate::{
    CardId, DesktopLayout,
    components::card_controls::{CardControls, CardResizeHandle},
    interaction::{DragState, ResizeState, card_style, keyboard_move, start_drag},
};

/// Unified desktop card container providing consistent header, controls, state classes, and gestures.
#[component]
pub fn CardFrame(
    card: CardId,
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<&'static str>,
    set_selected: WriteSignal<&'static str>,
    dragging: RwSignal<Option<DragState>>,
    resizing: RwSignal<Option<ResizeState>>,
    kicker_title: &'static str,
    kicker_icon: Arc<dyn Fn() -> AnyView + Send + Sync>,
    collapsed_summary: Arc<dyn Fn() -> AnyView + Send + Sync>,
    children: ChildrenFn,
) -> impl IntoView {
    let key = card.key();
    let is_open = move || layout.get().contains_card(card) && !layout.get().is_in_deck(card);
    let is_selected = move || selected.get() == key;
    let is_pinned = move || layout.get().presentation(card).pinned;
    let is_collapsed = move || layout.get().presentation(card).collapsed;
    let is_magnet_target = move || dragging.get().and_then(|d| d.drop_target) == Some(card);

    let aria_label = format!(
        "{} card. Drag to reposition; use arrow keys for keyboard movement.",
        card.title()
    );

    let kicker = StoredValue::new(kicker_icon);
    let collapsed = StoredValue::new(collapsed_summary);
    let render_children = StoredValue::new(children);

    view! {
        <Show when=is_open>
            <div
                class=format!("object {}", key)
                class:selected=is_selected
                class:pinned=is_pinned
                class:collapsed=is_collapsed
                class:magnet-target=is_magnet_target
                style=move || card_style(layout.get(), card)
                tabindex="0"
                role="region"
                aria-label=aria_label.clone()
                on:pointerdown=move |event: PointerEvent| start_drag(event, card, layout, dragging)
                on:keydown=move |event: KeyboardEvent| keyboard_move(event, card, layout)
                on:click=move |_| set_selected.set(key)
            >
                <header class="card-header">
                    <small class="panel-kicker">
                        {move || kicker.with_value(|k| k())}
                        <span>{kicker_title}</span>
                    </small>
                    <CardControls card=card layout=layout />
                </header>

                <Show
                    when=move || !layout.get().presentation(card).collapsed
                    fallback=move || collapsed.with_value(|c| c())
                >
                    {move || render_children.with_value(|rc| rc())}
                </Show>

                <CardResizeHandle card=card layout=layout resizing=resizing />
            </div>
        </Show>
    }
}
