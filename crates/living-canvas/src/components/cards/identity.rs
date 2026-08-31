// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Identity1 card and content component representing subject continuity and provenance.

use leptos::prelude::*;
use lucide_leptos::FileCheck;
use std::sync::Arc;

use crate::instant_label;
use crate::{
    CardId, DesktopItemId, DesktopLayout,
    components::card_frame::CardFrame,
    interaction::{DragState, ResizeState},
    state::{RuntimeState, unread},
};

/// Identity1 domain content presentation.
#[component]
pub fn IdentityContent(runtime: RwSignal<RuntimeState>) -> impl IntoView {
    let mind = move || match runtime.get() {
        RuntimeState::Ready { mind, .. } => mind,
        RuntimeState::Loading | RuntimeState::Error(_) | RuntimeState::SignInRequired => None,
    };
    let identity_id = move || {
        mind()
            .and_then(|m| m.identity.identity_id)
            .unwrap_or_else(unread)
    };
    let identity_origin = move || {
        mind()
            .and_then(|m| m.identity.origin)
            .map_or_else(unread, |origin| instant_label(&origin))
    };
    let identity_sessions = move || {
        mind()
            .and_then(|m| m.identity.session_count)
            .map_or_else(unread, |value| value.to_string())
    };
    let identity_age = move || {
        mind()
            .and_then(|m| m.identity.age_in_days)
            .map_or_else(unread, |value| format!("{value} d"))
    };
    let identity_architecture = move || {
        mind()
            .and_then(|m| m.identity.architecture_version)
            .unwrap_or_else(unread)
    };

    view! {
        <div class="identity-card-body">
            <strong>"Subject continuity"</strong>
            <span class="identity-digest">{identity_id}</span>
            <span class="identity-badges"><i>{identity_sessions}" sessions"</i><i>{identity_age}</i></span>
            <span class="identity-meta">"Origin "{identity_origin}" · "{identity_architecture}</span>
        </div>
    }
}

/// Identity1 cognitive card component.
#[component]
pub fn IdentityCard(
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
    // The frame's summary needs two of the same readings the content shows. Two closures over the
    // same signal rather than one shared: the card and its content are separate components, and
    // threading a value between them would put the frame's business inside the content.
    let sessions_seen = move || {
        mind()
            .and_then(|m| m.identity.session_count)
            .map_or_else(unread, |value| value.to_string())
    };
    let days_continuous = move || {
        mind()
            .and_then(|m| m.identity.age_in_days)
            .map_or_else(unread, |value| format!("{value} d"))
    };

    // What this card says from across the room. It used to be the subject's uuid, which is the one
    // fact on the card that nobody can read at a glance and nobody would recognise if they could —
    // at overview zoom it was thirty-six characters of noise where a headline goes. How long this
    // subject has been continuous, and across how many sessions, is what the card is about.
    let collapsed = move || {
        let sessions = sessions_seen();
        let age = days_continuous();
        view! {
            <div class="card-collapsed-summary">
                <b>"Identity"</b>
                <span>{sessions}" sessions · "{age}</span>
            </div>
        }
        .into_any()
    };

    view! {
        <CardFrame
            card=CardId::Identity
            layout=layout
            selected=selected
            set_selected=set_selected
            dragging=dragging
            resizing=resizing
            kicker_title="Identity1"
            kicker_icon=Arc::new(|| view! { <FileCheck size=14 /> }.into_any())
            collapsed_summary=Arc::new(collapsed)
        >
            <IdentityContent runtime=runtime />
        </CardFrame>
    }
}
