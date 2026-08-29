// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Capabilities card and content component representing Health1 capability graph and organ availability.

use cybou_protocol::CapabilityState;
use leptos::prelude::*;
use lucide_leptos::Sparkles;
use std::sync::Arc;

use crate::instant_label;
use crate::{
    CardId, DesktopItemId, DesktopLayout,
    components::card_frame::CardFrame,
    interaction::{DragState, ResizeState},
    state::{RuntimeState, capability_state_label},
};

/// Capabilities domain content presentation.
#[component]
pub fn CapabilitiesContent(runtime: RwSignal<RuntimeState>) -> impl IntoView {
    let capabilities = move || match runtime.get() {
        RuntimeState::Ready { snapshot, .. } => snapshot.capabilities,
        RuntimeState::Loading | RuntimeState::Error(_) | RuntimeState::SignInRequired => Vec::new(),
    };

    let system_label = move || match runtime.get() {
        RuntimeState::Loading => "Connecting…".into(),
        RuntimeState::SignInRequired => "Not signed in".into(),
        RuntimeState::Ready { snapshot, .. } => {
            let available = snapshot
                .capabilities
                .iter()
                .filter(|c| c.state == CapabilityState::Available)
                .count();
            format!("{available}/{} capabilities", snapshot.capabilities.len())
        }
        RuntimeState::Error(_) => "Gateway unavailable".into(),
    };

    let observed_label = move || match runtime.get() {
        RuntimeState::Ready { snapshot, .. } => instant_label(&snapshot.observed_at),
        RuntimeState::Loading => "Waiting for snapshot".into(),
        RuntimeState::SignInRequired => "Not signed in".into(),
        RuntimeState::Error(_) => "No snapshot".into(),
    };

    view! {
        <div class="capabilities-card-body">
            <h1>{system_label}</h1>
            <span class="capabilities-kind">"Capability health"</span>
            <p>"A capability is available only while every organ it depends on answers Health1. Nothing here is composed by this page."</p>
            <div class="capability-list">
                <For
                    each=capabilities
                    key=|capability| capability.id.clone()
                    children=move |capability| {
                        let available = capability.state == CapabilityState::Available;
                        let status = capability_state_label(capability.state);
                        let reason = capability.reason.unwrap_or_default();
                        view! {
                            <span class:available=available class="capability-line">
                                <span class="status-dot" aria-hidden="true"></span>
                                <b>{capability.id}</b>
                                <i>{status}</i>
                                <small>{reason}</small>
                            </span>
                        }
                    }
                />
            </div>
            <footer class="capabilities-meta">
                <span><small>"Observed"</small><b>{observed_label}</b></span>
            </footer>
        </div>
    }
}

/// Capabilities cognitive card component.
#[component]
pub fn CapabilitiesCard(
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<Option<DesktopItemId>>,
    set_selected: WriteSignal<Option<DesktopItemId>>,
    dragging: RwSignal<Option<DragState>>,
    resizing: RwSignal<Option<ResizeState>>,
    runtime: RwSignal<RuntimeState>,
) -> impl IntoView {
    let system_label = move || match runtime.get() {
        RuntimeState::Loading => "Connecting…".into(),
        RuntimeState::SignInRequired => "Not signed in".into(),
        RuntimeState::Ready { snapshot, .. } => {
            let available = snapshot
                .capabilities
                .iter()
                .filter(|c| c.state == CapabilityState::Available)
                .count();
            format!("{available}/{} capabilities", snapshot.capabilities.len())
        }
        RuntimeState::Error(_) => "Gateway unavailable".into(),
    };

    let collapsed = move || {
        let label = system_label();
        view! {
            <div class="card-collapsed-summary">
                <b>"Capabilities"</b>
                <span>{label}</span>
            </div>
        }
        .into_any()
    };

    view! {
        <CardFrame
            card=CardId::Capabilities
            layout=layout
            selected=selected
            set_selected=set_selected
            dragging=dragging
            resizing=resizing
            kicker_title="Health1"
            kicker_icon=Arc::new(|| view! { <Sparkles size=14 /> }.into_any())
            collapsed_summary=Arc::new(collapsed)
        >
            <CapabilitiesContent runtime=runtime />
        </CardFrame>
    }
}
