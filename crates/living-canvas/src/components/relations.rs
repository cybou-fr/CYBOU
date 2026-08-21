// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! SVG relationship connection line between dependent cognitive cards.

use leptos::prelude::*;

use crate::{CardId, DesktopLayout, interaction::relationship_points};

/// SVG relationship edge component.
#[component]
pub fn RelationshipEdge(
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<&'static str>,
    from: CardId,
    to: CardId,
    label: &'static str,
    amber: bool,
) -> impl IntoView {
    let points = move || relationship_points(layout.get(), from, to);
    view! {
        <g
            class:amber=amber
            class:active=move || selected.get() == from.key() || selected.get() == to.key()
            class="relationship-edge"
        >
            <line
                x1=move || points().0.to_string()
                y1=move || points().1.to_string()
                x2=move || points().2.to_string()
                y2=move || points().3.to_string()
            />
            <text
                x=move || points().4.to_string()
                y=move || points().5.to_string()
                text-anchor="middle"
            >{label}</text>
        </g>
    }
}
