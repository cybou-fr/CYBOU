// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Attention card and content component representing Workspace1 Global Workspace Theory attention focus.

use cybou_protocol::KnowledgeState;
use cybou_protocol::attention::SubjectReading;
use cybou_web_contracts::AttendedSubjectProjection;
use leptos::prelude::*;
use lucide_leptos::Map;
use std::sync::Arc;

use crate::{
    CardId, DesktopItemId, DesktopLayout,
    components::card_frame::CardFrame,
    interaction::{DragState, ResizeState},
    state::{RuntimeState, unread},
};

/// What one attended contribution is about, as a line a person can read.
///
/// The three readings stay three lines. A payload this build could not parse is shown as exactly
/// that rather than folded into "no subject", because the two look identical on screen and only one
/// of them is a claim about the machine.
fn subject_label(subject: &AttendedSubjectProjection) -> (String, String) {
    match &subject.reading {
        SubjectReading::Classified(query) => {
            (query.kind_name().to_owned(), query.identifier().to_owned())
        }
        SubjectReading::Uninterpreted(key) => ("Subject".to_owned(), key.clone()),
        SubjectReading::Unread => (subject.kind.clone(), "payload not read".to_owned()),
    }
}

/// The subjects of the focused coalition, or an explanation of why there are none.
fn attended_subjects(
    mind: Option<cybou_web_contracts::MindProjection>,
) -> Vec<AttendedSubjectProjection> {
    mind.filter(|m| m.attention.knowledge == KnowledgeState::Known)
        .map_or_else(Vec::new, |m| m.attention.subjects)
}

/// Attention domain content presentation.
#[component]
pub fn AttentionContent(runtime: RwSignal<RuntimeState>) -> impl IntoView {
    let mind = move || match runtime.get() {
        RuntimeState::Ready { mind, .. } => mind,
        RuntimeState::Loading | RuntimeState::Error(_) | RuntimeState::SignInRequired => None,
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

    let subjects = move || attended_subjects(mind());

    view! {
        <div class="attention-card-body">
            <strong>"Attention"</strong>
            <span class="attention-focus">{attention_focus}</span>
            <span class="row"><b>"Salience"</b><i>{attention_salience}</i></span>
            <span class="row"><b>"Organs"</b><i>{attention_organs}</i></span>
            <Show when=move || !subjects().is_empty()>
                <span class="heading-label">"About"</span>
                <div class="attention-subjects">
                    {move || subjects()
                        .into_iter()
                        .map(|subject| {
                            let (kind, label) = subject_label(&subject);
                            let organ = subject.organ.clone();
                            view! {
                                <span class="row attention-subject">
                                    <b>{label}</b>
                                    <i>{kind}" · "{organ}</i>
                                </span>
                            }
                        })
                        .collect_view()}
                </div>
            </Show>
        </div>
    }
}

/// Attention cognitive card component.
#[component]
pub fn AttentionCard(
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

    let collapsed = move || {
        // A person scanning a collapsed card wants what their machine is attending to, and a
        // correlation UUID is not that. The identity stays available in the open card.
        let focus = attended_subjects(mind())
            .first()
            .map_or_else(attention_focus, |subject| subject_label(subject).1);
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
            <AttentionContent runtime=runtime />
        </CardFrame>
    }
}
