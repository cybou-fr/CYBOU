// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Minimap navigation preview component for spatial canvas overview.
//!
//! What it draws is `desktop_items()`, not `cards`. Those are different sets: a card docked into a
//! deck is still in `cards`, so the old map drew it standing on its own at coordinates it had left,
//! and never drew the deck it was actually inside. Invariant L8 says a docked card is not a
//! top-level item; an overview that disagreed with the desktop it was overviewing is worse than no
//! overview.

use leptos::prelude::*;

use crate::{
    DesktopItem, DesktopItemId, DesktopLayout, MINIMAP_HEIGHT, MINIMAP_PADDING, MINIMAP_WIDTH,
    MinimapProjection, Rect, pan_centring, visible_desktop_rect,
};

/// The browser window, in CSS pixels.
///
/// A window that reports no size is treated as unknown rather than as a window of no size. The
/// difference shows: a zero-sized viewport rectangle collapses to the minimum the projection will
/// draw, and the map then states that the person is looking at a single point of their desktop.
fn viewport_size() -> (f64, f64) {
    let window = web_sys::window();
    let measure = |value: Option<f64>| match value {
        Some(size) if size.is_finite() && size > 0.0 => Some(size),
        _ => None,
    };
    let width = measure(
        window
            .as_ref()
            .and_then(|w| w.inner_width().ok())
            .and_then(|value| value.as_f64()),
    )
    .unwrap_or(1440.0);
    let height = measure(
        window
            .as_ref()
            .and_then(|w| w.inner_height().ok())
            .and_then(|value| value.as_f64()),
    )
    .unwrap_or(900.0);
    (width, height)
}

/// Absolute placement inside the minimap surface.
fn placement(rect: Rect) -> String {
    format!(
        "left:{:.1}px;top:{:.1}px;width:{:.1}px;height:{:.1}px",
        rect.x, rect.y, rect.width, rect.height
    )
}

/// Desktop spatial minimap overview component.
#[component]
pub fn Minimap(
    layout: RwSignal<DesktopLayout>,
    zoom: ReadSignal<f64>,
    pan: ReadSignal<(f64, f64)>,
    set_pan: WriteSignal<(f64, f64)>,
) -> impl IntoView {
    // Recomputed from the desktop's own bounds, so the map fits whatever the layout is rather than
    // whatever it was assumed to be.
    let projection = move || {
        let bounds = layout
            .get()
            .bounding_rect()
            .unwrap_or_else(|| Rect::new(0.0, 0.0, 1280.0, 650.0));
        MinimapProjection::fit(bounds, MINIMAP_WIDTH, MINIMAP_HEIGHT, MINIMAP_PADDING)
    };

    let items = move || layout.get().desktop_items();

    let pan_to = move |rect: Rect| {
        set_pan.set(pan_centring(rect, zoom.get_untracked(), viewport_size()));
    };

    // Where the person is. The map showed where the cards were and never this, which is the one
    // thing an overview is for.
    let viewport_box = move || {
        placement(projection().project(visible_desktop_rect(
            pan.get(),
            zoom.get(),
            viewport_size(),
        )))
    };

    view! {
        <nav class="desktop-minimap" aria-label="Desktop spatial overview">
            <div
                class="minimap-surface"
                style=format!("width:{MINIMAP_WIDTH:.0}px;height:{MINIMAP_HEIGHT:.0}px")
            >
                <For
                    each=items
                    key=|item: &DesktopItem| match &item.id {
                        DesktopItemId::Card(card) => format!("card:{}", card.key()),
                        DesktopItemId::Deck(deck) => format!("deck:{deck}"),
                    }
                    children=move |item: DesktopItem| {
                        let rect = item.effective_rect();
                        let drawn = projection().project(rect);
                        let is_deck = matches!(item.id, DesktopItemId::Deck(_));
                        let label = match &item.id {
                            DesktopItemId::Card(card) => card.title().to_owned(),
                            DesktopItemId::Deck(deck) => format!("Deck {deck}"),
                        };
                        let collapsed = item.presentation.collapsed;
                        let pinned = item.presentation.pinned;
                        view! {
                            <button
                                type="button"
                                class="minimap-item"
                                class:deck=is_deck
                                class:collapsed=collapsed
                                class:pinned=pinned
                                style=placement(drawn)
                                title=label.clone()
                                aria-label=label
                                on:click=move |_| pan_to(rect)
                            />
                        }
                    }
                />
                <div class="minimap-viewport" style=viewport_box />
            </div>
        </nav>
    }
}
