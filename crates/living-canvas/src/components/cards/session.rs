// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Session card and content component representing established trust and gateway session mode.

use cybou_web_contracts::SessionMode;
use leptos::prelude::*;
use lucide_leptos::UsersRound;
use std::sync::Arc;

use crate::{
    CardId, DesktopLayout,
    components::card_frame::CardFrame,
    interaction::{DragState, ResizeState},
    state::{RuntimeState, unread},
};

/// Session domain content presentation.
#[component]
pub fn SessionContent(runtime: RwSignal<RuntimeState>) -> impl IntoView {
    let runtime_label = move || match runtime.get() {
        RuntimeState::Loading => "Connecting".to_owned(),
        RuntimeState::SignInRequired => "Not signed in".to_owned(),
        RuntimeState::Ready { mode, .. } => match mode {
            SessionMode::LocalDesktop => "This machine".to_owned(),
            SessionMode::PublicPreview => "Open to anyone".to_owned(),
            SessionMode::RemoteBrowser => "Signed in, over the network".to_owned(),
            SessionMode::SignInRequired => "Not signed in".to_owned(),
        },
        RuntimeState::Error(_) => "Unavailable".to_owned(),
    };

    let session_consumer = move || match runtime.get() {
        RuntimeState::Ready { session, .. } => session.consumer_id,
        RuntimeState::Loading | RuntimeState::Error(_) | RuntimeState::SignInRequired => unread(),
    };

    let session_auth = move || match runtime.get() {
        RuntimeState::Ready { mode, .. } => match mode {
            SessionMode::RemoteBrowser => "Yes, with an account on this machine".to_owned(),
            SessionMode::LocalDesktop => "No, but the surface is on this machine".to_owned(),
            SessionMode::PublicPreview => "No".to_owned(),
            SessionMode::SignInRequired => "No".to_owned(),
        },
        RuntimeState::Loading | RuntimeState::Error(_) | RuntimeState::SignInRequired => unread(),
    };

    let session_device = move || match runtime.get() {
        RuntimeState::Ready { mode, .. } => match mode {
            SessionMode::LocalDesktop => "Yes, this machine".to_owned(),
            SessionMode::RemoteBrowser => "No, over the network".to_owned(),
            SessionMode::PublicPreview => "No".to_owned(),
            SessionMode::SignInRequired => "No".to_owned(),
        },
        RuntimeState::Loading | RuntimeState::Error(_) | RuntimeState::SignInRequired => unread(),
    };

    let session_id_short = move || match runtime.get() {
        RuntimeState::Ready { session, .. } => session
            .session_id
            .to_string()
            .chars()
            .take(8)
            .collect::<String>(),
        RuntimeState::Loading | RuntimeState::Error(_) | RuntimeState::SignInRequired => unread(),
    };

    let session_expires = move || match runtime.get() {
        RuntimeState::Ready { session, .. } => {
            if session.expires_at.is_empty() {
                "Never (Local)".to_owned()
            } else {
                session.expires_at
            }
        }
        RuntimeState::Loading | RuntimeState::Error(_) | RuntimeState::SignInRequired => unread(),
    };

    view! {
        <div class="session-card-body">
            <strong>{runtime_label}</strong>
            <span class="session-consumer">{session_consumer}</span>
            <span class="session-badges"><i>"Auth "{session_auth}</i><i>"Device "{session_device}</i></span>
            <span class="session-meta">"Session "{session_id_short}" · Expires "{session_expires}</span>
        </div>
    }
}

/// Session established trust card component.
#[component]
pub fn SessionCard(
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<&'static str>,
    set_selected: WriteSignal<&'static str>,
    dragging: RwSignal<Option<DragState>>,
    resizing: RwSignal<Option<ResizeState>>,
    runtime: RwSignal<RuntimeState>,
) -> impl IntoView {
    let runtime_label = move || match runtime.get() {
        RuntimeState::Loading => "Connecting".to_owned(),
        RuntimeState::SignInRequired => "Not signed in".to_owned(),
        RuntimeState::Ready { mode, .. } => match mode {
            SessionMode::LocalDesktop => "This machine".to_owned(),
            SessionMode::PublicPreview => "Open to anyone".to_owned(),
            SessionMode::RemoteBrowser => "Signed in, over the network".to_owned(),
            SessionMode::SignInRequired => "Not signed in".to_owned(),
        },
        RuntimeState::Error(_) => "Unavailable".to_owned(),
    };

    let collapsed = move || {
        let label = runtime_label();
        view! {
            <div class="card-collapsed-summary">
                <b>"Session"</b>
                <span>{label}</span>
            </div>
        }
        .into_any()
    };

    view! {
        <CardFrame
            card=CardId::Session
            layout=layout
            selected=selected
            set_selected=set_selected
            dragging=dragging
            resizing=resizing
            kicker_title="Session"
            kicker_icon=Arc::new(|| view! { <UsersRound size=14 /> }.into_any())
            collapsed_summary=Arc::new(collapsed)
        >
            <SessionContent runtime=runtime />
        </CardFrame>
    }
}
