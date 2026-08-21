// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Attention card component representing Workspace1 Global Workspace Theory attention focus.

use std::sync::Arc;
use cybou_protocol::KnowledgeState;
use leptos::prelude::*;
use lucide_leptos::Map;

use crate::{
    CardId, DesktopLayout,
    components::card_frame::CardFrame,
    interaction::{DragState, ResizeState},
    state::{RuntimeState, unread},
};

/// Attention cognitive card component.
#[component]
pub fn AttentionCard(
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<&'static str>,
    set_selected: WriteSignal<&'static str>,
    dragging: RwSignal<Option<DragState>>,
    resizing: RwSignal<Option<ResizeState>>,
    runtime: RwSignal<RuntimeState>,
) -> impl IntoView {
    let mind = move || match runtime.get() {
        RuntimeState::Ready { mind, .. } => mind,
        RuntimeState::Loading | RuntimeState::Error(_) => None,
    };

    let attention_focus = move || match mind() {
        None => "Workspace1 not read".to_owned(),
        Some(m) if m.attention.knowledge != KnowledgeState::Known => {
            "Workspace1 not read".to_owned()
        }
        Some(m) => m
            .attention
            .focus
            .unwrap_or_else(|| "Nothing holds focus".to_owned()),
    };

    let attention_salience = move || {
        mind()
            .and_then(|m| m.attention.salience)
            .map_or_else(unread, |value| format!("{value:.2}"))
    };

    let attention_organs = move || {
        let organs = mind().map_or_else(Vec::new, |m| m.attention.organs);
        if organs.is_empty() {
            unread()
        } else {
            organs.join(", ")
        }
    };

    let collapsed = move || {
        let focus = attention_focus();
        view! {
            <div class="card-collapsed-summary">
                <b>"Attention"</b>
                <span>{focus}</span>
            </div>
        }
        .into_any()
    };

    view! {
        <CardFrame
            card=CardId::Attention
            layout=layout
            selected=selected
            set_selected=set_selected
            dragging=dragging
            resizing=resizing
            kicker_title="Workspace1"
            kicker_icon=Arc::new(|| view! { <Map size=14 /> }.into_any())
            collapsed_summary=Arc::new(collapsed)
        >
            <strong>"Attention"</strong>
            <span class="attention-focus">{attention_focus}</span>
            <span class="row"><b>"Salience"</b><i>{attention_salience}</i></span>
            <span class="row"><b>"Organs"</b><i>{attention_organs}</i></span>
        </CardFrame>
    }
}
