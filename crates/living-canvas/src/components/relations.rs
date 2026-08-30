// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! SVG relationship connection line and graph layer between dependent cognitive cards.

use leptos::prelude::*;

use crate::{
    CardId, DesktopItemId, DesktopLayout,
    interaction::relationship_points,
    layout::relations::{DesktopRelationshipGraph, Relationship},
};

/// SVG relationship edge component connecting two cards.
#[component]
pub fn RelationshipEdge(
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<Option<DesktopItemId>>,
    from: CardId,
    to: CardId,
    label: &'static str,
    amber: bool,
) -> impl IntoView {
    let points = move || relationship_points(layout.get(), from, to);
    view! {
        <g
            class:amber=amber
            class:active=move || {
                let selected = selected.get();
                selected == Some(DesktopItemId::Card(from)) || selected == Some(DesktopItemId::Card(to))
            }
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

/// Full SVG layer rendering all canonical semantic relationship connections.
#[component]
pub fn RelationshipsLayer(
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<Option<DesktopItemId>>,
    #[prop(optional)] visibility: Option<ReadSignal<crate::layout::relations::RelationVisibility>>,
) -> impl IntoView {
    let relationships = DesktopRelationshipGraph::canonical();
    let vis = move || {
        visibility.map_or(
            crate::layout::relations::RelationVisibility::Selected,
            |v| v.get(),
        )
    };

    view! {
        <Show when=move || vis() != crate::layout::relations::RelationVisibility::Off>
            <svg class="relationships" aria-hidden="true">
                {relationships.iter().map(|rel: &Relationship| {
                    view! {
                        <RelationshipEdge
                            layout=layout
                            selected=selected
                            from=rel.from
                            to=rel.to
                            label=rel.label
                            amber=rel.amber
                        />
                    }
                }).collect_view()}
            </svg>
        </Show>
    }
}
