// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Journal card and content component representing Event1 canonical event store and integrity.

use leptos::prelude::*;
use lucide_leptos::Files;
use std::sync::Arc;

use crate::{
    CardId, DesktopLayout,
    components::card_frame::CardFrame,
    interaction::{DragState, ResizeState},
    state::{RuntimeState, knowledge_label, unread},
};

/// Journal domain content presentation.
#[component]
pub fn JournalContent(runtime: RwSignal<RuntimeState>) -> impl IntoView {
    let mind = move || match runtime.get() {
        RuntimeState::Ready { mind, .. } => mind,
        RuntimeState::Loading | RuntimeState::Error(_) => None,
    };

    let journal_count = move || {
        mind()
            .and_then(|m| m.journal.contribution_count)
            .map_or_else(unread, |value| value.to_string())
    };

    let journal_epoch = move || {
        mind()
            .and_then(|m| m.journal.erasure_epoch)
            .map_or_else(unread, |value| value.to_string())
    };

    let journal_recent = move || mind().map_or_else(Vec::new, |m| m.journal.recent);

    let journal_integrity = move || {
        mind()
            .and_then(|m| m.journal.integrity)
            .unwrap_or_else(|| "not verified yet".to_owned())
    };

    let journal_state = move || {
        mind().map_or_else(
            || "Event1 not read".to_owned(),
            |m| knowledge_label(m.journal.knowledge).to_owned(),
        )
    };

    view! {
        <div class="journal-card-body">
            <strong>"Canonical Journal"</strong>
            <span class="row"><b>"Contributions"</b><i>{journal_count}</i></span>
            <span class="row"><b>"Erasure epoch"</b><i>{journal_epoch}</i></span>
            <span class="row"><b>"Integrity"</b><i>{journal_integrity}</i></span>
            <div class="journal-feed">
                <For
                    each=journal_recent
                    key=|contribution| contribution.message_id.clone()
                    children=move |contribution| {
                        view! {
                            <span class="journal-line">
                                <b>{contribution.kind}</b>
                                <i>{contribution.origin_organ}</i>
                                <small>{contribution.recorded_at}</small>
                            </span>
                        }
                    }
                />
            </div>
            <span class="muted">{journal_state}</span>
        </div>
    }
}

/// Journal cognitive card component.
#[component]
pub fn JournalCard(
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

    let journal_count = move || {
        mind()
            .and_then(|m| m.journal.contribution_count)
            .map_or_else(unread, |value| value.to_string())
    };

    let collapsed = move || {
        let count = journal_count();
        view! {
            <div class="card-collapsed-summary">
                <b>"Journal"</b>
                <span>{count}" entries"</span>
            </div>
        }
        .into_any()
    };

    view! {
        <CardFrame
            card=CardId::Journal
            layout=layout
            selected=selected
            set_selected=set_selected
            dragging=dragging
            resizing=resizing
            kicker_title="Event1"
            kicker_icon=Arc::new(|| view! { <Files size=14 /> }.into_any())
            collapsed_summary=Arc::new(collapsed)
        >
            <JournalContent runtime=runtime />
        </CardFrame>
    }
}
