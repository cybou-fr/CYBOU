// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Unified generic `CardFrame` container encapsulating window mechanics, dragging, and resizing.

use leptos::prelude::*;
use std::sync::Arc;
use web_sys::{KeyboardEvent, PointerEvent};

use crate::{
    CardId, DesktopItemId, DesktopLayout,
    components::card_controls::{CardControls, CardResizeHandle},
    interaction::{DragState, ResizeState, card_style, keyboard_move, start_drag},
};

/// Unified desktop card container providing consistent header, controls, state classes, and gestures.
#[component]
pub fn CardFrame(
    card: CardId,
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<Option<DesktopItemId>>,
    set_selected: WriteSignal<Option<DesktopItemId>>,
    dragging: RwSignal<Option<DragState>>,
    resizing: RwSignal<Option<ResizeState>>,
    kicker_title: &'static str,
    kicker_icon: Arc<dyn Fn() -> AnyView + Send + Sync>,
    collapsed_summary: Arc<dyn Fn() -> AnyView + Send + Sync>,
    children: ChildrenFn,
) -> impl IntoView {
    // The kind, for CSS. Not the identity: `Shell(0)` and `Shell(2)` share this string, and
    // comparing it selected every Shell card at once while the action bar acted on the first.
    let key = card.key();
    let item = move || DesktopItemId::Card(card);
    let is_open = move || layout.get().contains_card(card) && !layout.get().is_in_deck(card);
    let is_selected = move || selected.get() == Some(item());
    let is_pinned = move || layout.get().presentation(card).pinned;
    let is_collapsed = move || layout.get().presentation(card).collapsed;
    let representation = move || layout.get().presentation(card).representation;
    let is_glance = move || representation() == crate::PanelRepresentation::Glance;
    let is_expanded = move || representation() == crate::PanelRepresentation::Expanded;
    let is_standard = move || representation() == crate::PanelRepresentation::Standard;
    let is_magnet_target = move || dragging.get().and_then(|d| d.drop_target) == Some(card);

    // Whether this card's contents are worth building.
    //
    // ADR-0044 named this as the cost of an infinite canvas, and it is not idle markup: every card
    // here holds signals that update on a timer, so a panel nobody can see is work nobody asked
    // for. What is dropped is the contents; the frame stays, which keeps the card where the layout
    // says it is, keeps the minimap and hit-testing honest, and keeps a selected card selectable.
    //
    // Absent context means everything draws. A card must never be hidden because a wiring step was
    // forgotten somewhere above it — a panel that vanished for that reason is one a person cannot
    // find and cannot explain.
    let camera = use_context::<crate::components::camera_context::CanvasCamera>();
    let is_drawable = move || {
        // Focus takes the card out of the canvas transform entirely, and a dragged card is under
        // the pointer whatever the arithmetic says about where it was when the drag began.
        if dragging
            .get()
            .is_some_and(|drag| drag.target == crate::interaction::DragTarget::Card(card))
        {
            return true;
        }
        camera.is_none_or(|camera| camera.shows(layout.get().geometry(card)))
    };

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
                class:glance=is_glance
                class:expanded=is_expanded
                class:standard=is_standard
                class:magnet-target=is_magnet_target
                style=move || card_style(layout.get(), card)
                tabindex="0"
                role="region"
                aria-label=aria_label.clone()
                on:pointerdown=move |event: PointerEvent| start_drag(event, card, layout, dragging)
                on:keydown=move |event: KeyboardEvent| keyboard_move(event, card, layout)
                on:click=move |_| set_selected.set(Some(item()))
            >
                <header class="card-header">
                    <small class="panel-kicker">
                        {move || kicker.with_value(|k| k())}
                        // What the card is, in words, and then which organ composed it. Every
                        // System card used to hand over the organ name alone, so the desktop
                        // greeted a first-time reader with IDENTITY1, HEALTH1 and EPISTEMIC1 and
                        // left them to guess. The organ still has to be visible — it is how this
                        // desktop can say who composed an answer rather than implying the page
                        // did — so it is kept, in small type, beside a title anyone can read.
                        <span class="panel-title">{card.title()}</span>
                        <Show when=move || kicker_title != card.title()>
                            <span class="panel-organ" title="The organ that composed this">
                                {kicker_title}
                            </span>
                        </Show>
                    </small>
                    <CardControls card=card layout=layout />
                </header>

                <Show when=is_drawable>
                    <Show
                        when=move || !layout.get().presentation(card).collapsed
                        fallback=move || collapsed.with_value(|c| c())
                    >
                        {move || render_children.with_value(|rc| rc())}
                    </Show>
                </Show>

                <CardResizeHandle card=card layout=layout resizing=resizing />
            </div>
        </Show>
    }
}
