// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Context card and content component representing Context1 associative concepts graph.

use std::sync::Arc;
use cybou_protocol::KnowledgeState;
use leptos::prelude::*;
use lucide_leptos::Link;

use crate::{
    CardId, DesktopLayout,
    components::card_frame::CardFrame,
    interaction::{DragState, ResizeState},
    state::RuntimeState,
};

/// Context domain content presentation.
#[component]
pub fn ContextContent(runtime: RwSignal<RuntimeState>) -> impl IntoView {
    let mind = move || match runtime.get() {
        RuntimeState::Ready { mind, .. } => mind,
        RuntimeState::Loading | RuntimeState::Error(_) => None,
    };

    let concepts = move || mind().map_or_else(Vec::new, |m| m.context.concepts);
    let context_label = move || match mind() {
        None => "Context1 not read".to_owned(),
        Some(m) if m.context.knowledge != KnowledgeState::Known => "Context1 not read".to_owned(),
        Some(m) => match m.context.concepts.len() {
            0 => "Nothing activated yet".to_owned(),
            1 => "1 active concept".to_owned(),
            count => format!("{count} active concepts"),
        },
    };

    view! {
        <div class="context-card-body">
            <strong>{context_label}</strong>
            <div class="concept-list">
                <For
                    each=concepts
                    key=|concept| concept.label.clone()
                    children=move |concept| {
                        view! {
                            <span class="concept-line">
                                <b>{concept.label}</b>
                                <i>{format!("{:.2}", concept.salience)}</i>
                                <small>{concept.activation_reason}</small>
                            </span>
                        }
                    }
                />
            </div>
            <span class="panel-link">"Association is not truth"</span>
        </div>
    }
}

/// Context cognitive card component.
#[component]
pub fn ContextCard(
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

    let context_label = move || match mind() {
        None => "Context1 not read".to_owned(),
        Some(m) if m.context.knowledge != KnowledgeState::Known => "Context1 not read".to_owned(),
        Some(m) => match m.context.concepts.len() {
            0 => "Nothing activated yet".to_owned(),
            1 => "1 active concept".to_owned(),
            count => format!("{count} active concepts"),
        },
    };

    let collapsed = move || {
        let label = context_label();
        view! {
            <div class="card-collapsed-summary">
                <b>"Context"</b>
                <span>{label}</span>
            </div>
        }
        .into_any()
    };

    view! {
        <CardFrame
            card=CardId::Context
            layout=layout
            selected=selected
            set_selected=set_selected
            dragging=dragging
            resizing=resizing
            kicker_title="Context1"
            kicker_icon=Arc::new(|| view! { <Link size=14 /> }.into_any())
            collapsed_summary=Arc::new(collapsed)
        >
            <ContextContent runtime=runtime />
        </CardFrame>
    }
}
