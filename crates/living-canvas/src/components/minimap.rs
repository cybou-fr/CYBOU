// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Minimap navigation preview component for spatial canvas overview.

use leptos::prelude::*;

use crate::{CardId, DesktopLayout, interaction::minimap_style};

/// Desktop spatial minimap overview component.
#[component]
pub fn Minimap(
    layout: RwSignal<DesktopLayout>,
    zoom: ReadSignal<f64>,
    set_pan: WriteSignal<(f64, f64)>,
) -> impl IntoView {
    let pan_to_card = move |card_id: CardId| {
        let geom = layout.get_untracked().geometry(card_id);
        let (vw, vh) = (
            web_sys::window()
                .and_then(|w| w.inner_width().ok())
                .and_then(|v| v.as_f64())
                .unwrap_or(1440.0),
            web_sys::window()
                .and_then(|w| w.inner_height().ok())
                .and_then(|v| v.as_f64())
                .unwrap_or(900.0),
        );
        let z = zoom.get();
        let target_x = (vw / 2.0) - (geom.x + geom.width / 2.0) * z;
        let target_y = (vh / 2.0) - (geom.y + geom.height / 2.0) * z;
        set_pan.set((target_x, target_y));
    };

    view! {
        <nav class="desktop-minimap" aria-label="Desktop spatial overview">
            <div class="minimap-surface">
                {move || {
                    let current = layout.get();
                    current.cards.into_iter().map(|c| {
                        let id = c.id;
                        let is_collapsed = c.presentation.collapsed;
                        let geom = c.geometry;
                        let label = id.title();
                        view! {
                            <div
                                class="minimap-card"
                                class:collapsed=is_collapsed
                                style=minimap_style(geom)
                                title=label
                                on:click=move |_| pan_to_card(id)
                            />
                        }
                    }).collect::<Vec<_>>()
                }}
            </div>
        </nav>
    }
}
