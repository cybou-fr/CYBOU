// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Beliefs card and content component representing Epistemic1 derived beliefs and validity.

use cybou_protocol::KnowledgeState;
use leptos::prelude::*;
use lucide_leptos::Sparkles;
use std::sync::Arc;

use crate::{
    CardId, DesktopItemId, DesktopLayout,
    components::card_frame::CardFrame,
    interaction::{DragState, ResizeState},
    state::RuntimeState,
};

/// Beliefs domain content presentation.
#[component]
pub fn BeliefsContent(runtime: RwSignal<RuntimeState>) -> impl IntoView {
    let mind = move || match runtime.get() {
        RuntimeState::Ready { mind, .. } => mind,
        RuntimeState::Loading | RuntimeState::Error(_) | RuntimeState::SignInRequired => None,
    };

    let beliefs = move || mind().map_or_else(Vec::new, |m| m.beliefs.beliefs);
    let beliefs_label = move || match mind() {
        None => "Epistemic1 not read".to_owned(),
        Some(m) if m.beliefs.knowledge != KnowledgeState::Known => "Epistemic1 not read".to_owned(),
        Some(m) => match m.beliefs.beliefs.len() {
            0 => "Believes nothing yet".to_owned(),
            1 => "1 belief".to_owned(),
            count => format!("{count} beliefs"),
        },
    };

    view! {
        <div class="beliefs-card-body">
            <strong>{beliefs_label}</strong>
            <div class="belief-list">
                <For
                    each=beliefs
                    key=|belief| belief.subject.clone()
                    children=move |belief| {
                        let observed = belief.status == "observed";
                        view! {
                            <span class:observed=observed class="belief-line">
                                <b>{belief.subject}</b>
                                <span class="belief-value">{belief.value}</span>
                                <i>{belief.status}</i>
                            </span>
                        }
                    }
                />
            </div>
            <span class="panel-link">"A belief and its validity are separate facts"</span>
        </div>
    }
}

/// Beliefs cognitive card component.
#[component]
pub fn BeliefsCard(
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<Option<DesktopItemId>>,
    set_selected: WriteSignal<Option<DesktopItemId>>,
    dragging: RwSignal<Option<DragState>>,
    resizing: RwSignal<Option<ResizeState>>,
    runtime: RwSignal<RuntimeState>,
) -> impl IntoView {
    let mind = move || match runtime.get() {
        RuntimeState::Ready { mind, .. } => mind,
        RuntimeState::Loading | RuntimeState::Error(_) | RuntimeState::SignInRequired => None,
    };

    let beliefs_label = move || match mind() {
        None => "Epistemic1 not read".to_owned(),
        Some(m) if m.beliefs.knowledge != KnowledgeState::Known => "Epistemic1 not read".to_owned(),
        Some(m) => match m.beliefs.beliefs.len() {
            0 => "Believes nothing yet".to_owned(),
            1 => "1 belief".to_owned(),
            count => format!("{count} beliefs"),
        },
    };

    let collapsed = move || {
        let label = beliefs_label();
        view! {
            <div class="card-collapsed-summary">
                <b>"Beliefs"</b>
                <span>{label}</span>
            </div>
        }
        .into_any()
    };

    view! {
        <CardFrame
            card=CardId::Beliefs
            layout=layout
            selected=selected
            set_selected=set_selected
            dragging=dragging
            resizing=resizing
            kicker_title="Epistemic1"
            kicker_icon=Arc::new(|| view! { <Sparkles size=14 /> }.into_any())
            collapsed_summary=Arc::new(collapsed)
        >
            <BeliefsContent runtime=runtime />
        </CardFrame>
    }
}
