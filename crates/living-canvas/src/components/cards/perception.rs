// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Perception card and content component representing Perception1 host observations.

use leptos::prelude::*;
use lucide_leptos::Files;
use std::sync::Arc;

use crate::{
    CardId, DesktopLayout,
    components::card_frame::CardFrame,
    interaction::{DragState, ResizeState},
    state::{RuntimeState, unread},
};

/// Perception domain content presentation.
#[component]
pub fn PerceptionContent(runtime: RwSignal<RuntimeState>) -> impl IntoView {
    let mind = move || match runtime.get() {
        RuntimeState::Ready { mind, .. } => mind,
        RuntimeState::Loading | RuntimeState::Error(_) => None,
    };

    let perception_status = move || {
        mind()
            .and_then(|m| m.perception.status)
            .unwrap_or_else(unread)
    };
    let perception_source = move || {
        mind()
            .and_then(|m| m.perception.source_id)
            .unwrap_or_else(unread)
    };
    let perception_at = move || {
        mind()
            .and_then(|m| m.perception.acquired_at)
            .unwrap_or_else(unread)
    };

    view! {
        <div class="perception-card-body">
            <strong>"Host observation"</strong>
            <span class="row"><b>"Status"</b><i>{perception_status}</i></span>
            <span class="row"><b>"Source"</b><i>{perception_source}</i></span>
            <span class="row"><b>"Acquired"</b><i>{perception_at}</i></span>
        </div>
    }
}

/// Perception cognitive card component.
#[component]
pub fn PerceptionCard(
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

    let perception_status = move || {
        mind()
            .and_then(|m| m.perception.status)
            .unwrap_or_else(unread)
    };

    let collapsed = move || {
        let status = perception_status();
        view! {
            <div class="card-collapsed-summary">
                <b>"Perception"</b>
                <span>{status}</span>
            </div>
        }
        .into_any()
    };

    view! {
        <CardFrame
            card=CardId::Perception
            layout=layout
            selected=selected
            set_selected=set_selected
            dragging=dragging
            resizing=resizing
            kicker_title="Perception1"
            kicker_icon=Arc::new(|| view! { <Files size=14 /> }.into_any())
            collapsed_summary=Arc::new(collapsed)
        >
            <PerceptionContent runtime=runtime />
        </CardFrame>
    }
}
