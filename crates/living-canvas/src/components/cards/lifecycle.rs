// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Lifecycle1 card and content component representing sleep/wake states and background consolidation.

use leptos::prelude::*;
use lucide_leptos::Sparkles;
use std::sync::Arc;

use crate::{
    CardId, DesktopLayout,
    components::card_frame::CardFrame,
    interaction::{DragState, ResizeState},
    state::{RuntimeState, unread},
};

/// Lifecycle1 domain content presentation.
#[component]
pub fn LifecycleContent(runtime: RwSignal<RuntimeState>) -> impl IntoView {
    let mind = move || match runtime.get() {
        RuntimeState::Ready { mind, .. } => mind,
        RuntimeState::Loading | RuntimeState::Error(_) => None,
    };

    let lifecycle_activity = move || {
        mind()
            .and_then(|m| m.lifecycle.last_user_activity_at)
            .unwrap_or_else(unread)
    };
    let mind_observed = move || {
        mind().map_or_else(
            || "owners not read".to_owned(),
            |m| format!("Owners read {}", m.observed_at),
        )
    };

    view! {
        <div class="lifecycle-card-body">
            <strong>"Sleep and wake"</strong>
            <p>"The mode is the owner's own spelling, not a summary of it. After fifteen idle minutes the system re-verifies its whole chain, and stops the moment someone arrives."</p>
            <span class="row"><b>"Last user activity"</b><i>{lifecycle_activity}</i></span>
            <span class="lifecycle-source">{mind_observed}</span>
        </div>
    }
}

/// Lifecycle1 cognitive card component.
#[component]
pub fn LifecycleCard(
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

    let lifecycle_mode = move || mind().and_then(|m| m.lifecycle.mode).unwrap_or_else(unread);

    let collapsed = move || {
        let mode = lifecycle_mode();
        view! {
            <div class="card-collapsed-summary">
                <b>"Lifecycle"</b>
                <span>{mode}</span>
            </div>
        }
        .into_any()
    };

    view! {
        <CardFrame
            card=CardId::Lifecycle
            layout=layout
            selected=selected
            set_selected=set_selected
            dragging=dragging
            resizing=resizing
            kicker_title="Lifecycle1"
            kicker_icon=Arc::new(|| view! { <Sparkles size=14 /> }.into_any())
            collapsed_summary=Arc::new(collapsed)
        >
            <LifecycleContent runtime=runtime />
        </CardFrame>
    }
}
