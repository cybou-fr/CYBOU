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
        RuntimeState::Ready { mode, .. } => match mode {
            SessionMode::LocalDesktop => "Local desktop (Zone 2)".to_owned(),
            SessionMode::PublicPreview => "Public surface (Zone 1)".to_owned(),
            SessionMode::RemoteBrowser => "Remote browser (Zone 2)".to_owned(),
        },
        RuntimeState::Error(_) => "Unavailable".to_owned(),
    };

    let session_consumer = move || match runtime.get() {
        RuntimeState::Ready { session, .. } => session.consumer_id,
        RuntimeState::Loading | RuntimeState::Error(_) => unread(),
    };

    let session_auth = move || match runtime.get() {
        RuntimeState::Ready { mode, .. } => match mode {
            SessionMode::RemoteBrowser => "Yes (Host token)".to_owned(),
            SessionMode::LocalDesktop => "Device loopback".to_owned(),
            SessionMode::PublicPreview => "No (Public)".to_owned(),
        },
        RuntimeState::Loading | RuntimeState::Error(_) => unread(),
    };

    let session_device = move || match runtime.get() {
        RuntimeState::Ready { mode, .. } => match mode {
            SessionMode::LocalDesktop => "Yes (Local Unix Socket)".to_owned(),
            SessionMode::RemoteBrowser => "No (Network Session)".to_owned(),
            SessionMode::PublicPreview => "No (Public Surface)".to_owned(),
        },
        RuntimeState::Loading | RuntimeState::Error(_) => unread(),
    };

    let session_id_short = move || match runtime.get() {
        RuntimeState::Ready { session, .. } => session
            .session_id
            .to_string()
            .chars()
            .take(8)
            .collect::<String>(),
        RuntimeState::Loading | RuntimeState::Error(_) => unread(),
    };

    let session_expires = move || match runtime.get() {
        RuntimeState::Ready { session, .. } => {
            if session.expires_at.is_empty() {
                "Never (Local)".to_owned()
            } else {
                session.expires_at
            }
        }
        RuntimeState::Loading | RuntimeState::Error(_) => unread(),
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
        RuntimeState::Ready { mode, .. } => match mode {
            SessionMode::LocalDesktop => "Local desktop (Zone 2)".to_owned(),
            SessionMode::PublicPreview => "Public surface (Zone 1)".to_owned(),
            SessionMode::RemoteBrowser => "Remote browser (Zone 2)".to_owned(),
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
