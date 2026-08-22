// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Self-model card and content component representing Self1 autobiographical assessment and narration.

use std::sync::Arc;
use leptos::prelude::*;
use lucide_leptos::Sparkles;

use crate::{
    CardId, DesktopLayout,
    components::card_frame::CardFrame,
    interaction::{DragState, ResizeState},
    state::{RuntimeState, unread},
};

/// SelfModel domain content presentation.
#[component]
pub fn SelfModelContent(runtime: RwSignal<RuntimeState>) -> impl IntoView {
    let mind = move || match runtime.get() {
        RuntimeState::Ready { mind, .. } => mind,
        RuntimeState::Loading | RuntimeState::Error(_) => None,
    };

    let self_narration = move || {
        mind()
            .and_then(|m| m.self_model.narration)
            .unwrap_or_else(|| "Self1 has not been read.".to_owned())
    };
    let self_open_intentions = move || {
        mind()
            .and_then(|m| m.self_model.open_intentions)
            .map_or_else(unread, |value| value.to_string())
    };
    let self_settled = move || {
        mind()
            .and_then(|m| m.self_model.settled_predictions)
            .map_or_else(unread, |value| value.to_string())
    };

    view! {
        <div class="self-model-card-body">
            <strong>"Self-assessment"</strong>
            <p class="self-narration">{self_narration}</p>
            <span class="row"><b>"Open obligations"</b><i>{self_open_intentions}</i></span>
            <span class="row"><b>"Settled predictions"</b><i>{self_settled}</i></span>
            <span class="panel-link">"Composed by Self1, not by this page"</span>
        </div>
    }
}

/// Self-model cognitive card component.
#[component]
pub fn SelfModelCard(
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

    let self_open_intentions = move || {
        mind()
            .and_then(|m| m.self_model.open_intentions)
            .map_or_else(unread, |value| value.to_string())
    };

    let collapsed = move || {
        let open = self_open_intentions();
        view! {
            <div class="card-collapsed-summary">
                <b>"Self-assessment"</b>
                <span>{open}" open"</span>
            </div>
        }
        .into_any()
    };

    view! {
        <CardFrame
            card=CardId::SelfModel
            layout=layout
            selected=selected
            set_selected=set_selected
            dragging=dragging
            resizing=resizing
            kicker_title="Self1"
            kicker_icon=Arc::new(|| view! { <Sparkles size=14 /> }.into_any())
            collapsed_summary=Arc::new(collapsed)
        >
            <SelfModelContent runtime=runtime />
        </CardFrame>
    }
}
