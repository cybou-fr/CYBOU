// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Commitments card and content component representing Intention1 open obligations.

use cybou_protocol::KnowledgeState;
use leptos::prelude::*;
use lucide_leptos::ListChecks;
use std::sync::Arc;

use crate::{
    MindClient,
    CardId, DesktopItemId, DesktopLayout,
    components::card_frame::CardFrame,
    interaction::{DragState, ResizeState},
    state::RuntimeState,
};

/// Commitments domain content presentation.
#[component]
pub fn CommitmentsContent(runtime: RwSignal<RuntimeState>) -> impl IntoView {
    let mind = move || match runtime.get() {
        RuntimeState::Ready { mind, .. } => mind,
        RuntimeState::Loading | RuntimeState::Error(_) | RuntimeState::SignInRequired => None,
    };

    let commitments = move || mind().map_or_else(Vec::new, |m| m.commitments.open);
    let commitments_label = move || match mind() {
        None => "Intention1 not read".to_owned(),
        Some(m) if m.commitments.knowledge != KnowledgeState::Known => {
            "Intention1 not read".to_owned()
        }
        Some(m) => match m.commitments.open_count.unwrap_or_default() {
            0 => "No open commitments".to_owned(),
            1 => "1 open commitment".to_owned(),
            count => format!("{count} open commitments"),
        },
    };

    view! {
        <div class="commitments-card-body">
            <div class="commitments-meta-label">
                <strong>{commitments_label}</strong>
            </div>
            <For
                each=commitments
                key=|commitment| commitment.id.clone()
                children=move |commitment| {
                    view! {
                        <span class="check-row">
                            <b>{commitment.description}</b>
                            <i>{commitment.trigger}</i>
                        </span>
                    }
                }
            />
            <span class="panel-link">"Intention1 holds these until they are closed"</span>
        </div>
    }
}

/// Commitments cognitive card component.
#[component]
pub fn CommitmentsCard(
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

    let commitments_label = move || match mind() {
        None => "Intention1 not read".to_owned(),
        Some(m) if m.commitments.knowledge != KnowledgeState::Known => {
            "Intention1 not read".to_owned()
        }
        Some(m) => match m.commitments.open_count.unwrap_or_default() {
            0 => "No open commitments".to_owned(),
            1 => "1 open commitment".to_owned(),
            count => format!("{count} open commitments"),
        },
    };

    let collapsed = move || {
        let label = commitments_label();
        view! {
            <div class="card-collapsed-summary">
                <b>"Commitments"</b>
                <span>{label}</span>
            </div>
        }
        .into_any()
    };

    view! {
        <CardFrame
            card=CardId::Commitments
            layout=layout
            selected=selected
            set_selected=set_selected
            dragging=dragging
            resizing=resizing
            kicker_title="Intention1"
            kicker_icon=Arc::new(|| view! { <ListChecks size=14 /> }.into_any())
            collapsed_summary=Arc::new(collapsed)
        >
            <CommitmentsContent runtime=runtime />
        </CardFrame>
    }
}
