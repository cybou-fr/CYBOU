// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Deck container component managing tabbed card groupings.

use cybou_protocol::{CapabilityState, KnowledgeState};
use cybou_web_contracts::SessionMode;
use leptos::prelude::*;
use web_sys::KeyboardEvent;
use web_sys::PointerEvent;

use crate::{
    CardId, DesktopItemId, DesktopLayout, DesktopViewMode, LayoutHistory,
    components::{
        card_controls::DeckResizeHandle,
        icons::{IconExternalLink, IconLayers, IconMaximize, IconMinimize, IconPin},
    },
    interaction::{DragState, ResizeState, keyboard_deck_move, start_deck_drag},
    state::{RuntimeState, unread},
};

/// Deck grouping container component.
#[component]
pub fn DeckContainerView(
    deck_id: String,
    layout: RwSignal<DesktopLayout>,
    history: RwSignal<LayoutHistory>,
    dragging: RwSignal<Option<DragState>>,
    resizing: RwSignal<Option<ResizeState>>,
    runtime: RwSignal<RuntimeState>,
) -> impl IntoView {
    let d_id = StoredValue::new(deck_id);

    let deck_opt = Signal::derive(move || layout.get().deck(&d_id.get_value()).cloned());
    let is_collapsed =
        Signal::derive(move || deck_opt.get().is_some_and(|d| d.presentation.collapsed));
    let is_pinned = Signal::derive(move || deck_opt.get().is_some_and(|d| d.presentation.pinned));
    let active_card =
        Signal::derive(move || deck_opt.get().map_or(CardId::Identity, |d| d.active_card));
    let cards = Signal::derive(move || deck_opt.get().map_or_else(Vec::new, |d| d.card_ids));

    let is_magnet = Signal::derive(move || {
        let target_opt = dragging.get().and_then(|drag| drag.drop_target);
        target_opt.is_some_and(|target| cards.get().contains(&target))
    });

    let deck_style = Signal::derive(move || {
        let vm = use_context::<RwSignal<DesktopViewMode>>()
            .map_or(DesktopViewMode::Spatial, |v| v.get());
        if vm == DesktopViewMode::Focus(DesktopItemId::Deck(d_id.get_value())) {
            "position: fixed; left: 20px; top: 20px; width: calc(100vw - 40px); height: calc(100vh - 100px); z-index: 9999; box-shadow: 0 0 0 9999px rgba(0,0,0,0.65);".to_string()
        } else if let Some(deck) = deck_opt.get() {
            let geom = deck.geometry;
            let h = if deck.presentation.collapsed {
                44.0
            } else {
                geom.height
            };
            format!(
                "transform: translate3d({:.1}px, {:.1}px, 0); width: {:.1}px; height: {:.1}px; z-index: {};",
                geom.x, geom.y, geom.width, h, geom.z
            )
        } else {
            String::new()
        }
    });

    let runtime_label = move || match runtime.get() {
        RuntimeState::Loading => "Connecting".to_owned(),
        RuntimeState::Ready { mode, .. } => match mode {
            SessionMode::LocalDesktop => "Local".to_owned(),
            SessionMode::PublicPreview => "Preview".to_owned(),
            SessionMode::RemoteBrowser => "Remote".to_owned(),
        },
        RuntimeState::Error(_) => "Unavailable".to_owned(),
    };
    let system_label = move || match runtime.get() {
        RuntimeState::Loading => "Connecting…".into(),
        RuntimeState::Ready { snapshot, .. } => {
            let available = snapshot
                .capabilities
                .iter()
                .filter(|capability| capability.state == CapabilityState::Available)
                .count();
            format!("{available}/{} capabilities", snapshot.capabilities.len())
        }
        RuntimeState::Error(_) => "Gateway unavailable".into(),
    };
    let observed_label = move || match runtime.get() {
        RuntimeState::Ready { snapshot, .. } => format!("Observed {}", snapshot.observed_at),
        RuntimeState::Loading => "Waiting for snapshot".into(),
        RuntimeState::Error(_) => "No snapshot".into(),
    };
    let mind = move || match runtime.get() {
        RuntimeState::Ready { mind, .. } => mind,
        RuntimeState::Loading | RuntimeState::Error(_) => None,
    };
    let identity_id = move || {
        mind()
            .and_then(|m| m.identity.identity_id)
            .unwrap_or_else(unread)
    };
    let identity_origin = move || {
        mind()
            .and_then(|m| m.identity.origin)
            .unwrap_or_else(unread)
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
    let session_consumer = move || match runtime.get() {
        RuntimeState::Ready { session, .. } => session.consumer_id,
        RuntimeState::Loading | RuntimeState::Error(_) => unread(),
    };
    let session_auth = move || match runtime.get() {
        RuntimeState::Ready { mode, .. } => match mode {
            SessionMode::RemoteBrowser => "Yes (Host token)".to_owned(),
            SessionMode::LocalDesktop => "Device loopback".to_owned(),
            SessionMode::PublicPreview => "No (Preview)".to_owned(),
        },
        RuntimeState::Loading | RuntimeState::Error(_) => unread(),
    };
    let session_device = move || match runtime.get() {
        RuntimeState::Ready { mode, .. } => match mode {
            SessionMode::LocalDesktop => "Yes (Local)".to_owned(),
            SessionMode::RemoteBrowser => "No (Network)".to_owned(),
            SessionMode::PublicPreview => "No (Public)".to_owned(),
        },
        RuntimeState::Loading | RuntimeState::Error(_) => unread(),
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
    let journal_integrity = move || {
        mind()
            .and_then(|m| m.journal.integrity)
            .unwrap_or_else(|| "not verified yet".to_owned())
    };
    let lifecycle_mode = move || mind().and_then(|m| m.lifecycle.mode).unwrap_or_else(unread);
    let lifecycle_activity = move || {
        mind()
            .and_then(|m| m.lifecycle.last_user_activity_at)
            .unwrap_or_else(unread)
    };
    let commitments_label = move || {
        mind()
            .and_then(|m| m.commitments.open_count)
            .map_or_else(unread, |value| format!("{value} active commitments"))
    };
    let self_narration = move || {
        mind()
            .and_then(|m| m.self_model.narration)
            .unwrap_or_else(|| "Self1 has not been read.".to_owned())
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
    let beliefs_label = move || match mind() {
        None => "Epistemic1 not read".to_owned(),
        Some(m) if m.beliefs.knowledge != KnowledgeState::Known => "Epistemic1 not read".to_owned(),
        Some(m) => match m.beliefs.beliefs.len() {
            0 => "Believes nothing yet".to_owned(),
            1 => "1 belief".to_owned(),
            count => format!("{count} beliefs"),
        },
    };
    let perception_status = move || {
        mind()
            .and_then(|m| m.perception.status)
            .unwrap_or_else(unread)
    };
    let context_label = move || match mind() {
        None => "Context1 not read".to_owned(),
        Some(m) if m.context.knowledge != KnowledgeState::Known => "Context1 not read".to_owned(),
        Some(m) => match m.context.concepts.len() {
            0 => "No concepts indexed".to_owned(),
            1 => "1 concept indexed".to_owned(),
            count => format!("{count} concepts indexed"),
        },
    };

    view! {
        <Show when=move || deck_opt.get().is_some()>
            <div
                class="object deck-container"
                class:magnet-target=move || is_magnet.get()
                class:pinned=move || is_pinned.get()
                class:collapsed=move || is_collapsed.get()
                style=move || deck_style.get()
                tabindex="0"
                role="region"
                aria-label=move || format!("Deck container: {}", deck_opt.get().map_or_else(String::new, |d| d.title))
                on:click=move |_| {
                    layout.update(|l| l.bring_deck_forward(&d_id.get_value()));
                }
                on:keydown=move |event: KeyboardEvent| keyboard_deck_move(event, &d_id.get_value(), layout)
            >
                <header
                    class="deck-header"
                    on:pointerdown=move |event: PointerEvent| {
                        start_deck_drag(event, d_id.get_value(), layout, dragging);
                    }
                >
                    <div
                        class="deck-tabs"
                        role="tablist"
                        aria-label="Deck tabs"
                        on:pointerdown=move |e: PointerEvent| e.stop_propagation()
                    >
                        <For
                            each=move || cards.get()
                            key=|card| *card
                            children=move |card| {
                                let is_active = move || active_card.get() == card;
                                view! {
                                    <div
                                        class="deck-tab"
                                        class:active=is_active
                                        role="tab"
                                        tabindex="0"
                                        aria-selected=move || is_active().to_string()
                                        on:pointerdown=move |e: PointerEvent| {
                                            e.stop_propagation();
                                        }
                                        on:click=move |e: web_sys::MouseEvent| {
                                            e.stop_propagation();
                                            layout.update(|l| {
                                                if let Some(d) = l.deck_mut(&d_id.get_value()) {
                                                    d.set_active(card);
                                                }
                                            });
                                            layout.get_untracked().save();
                                        }
                                        on:keydown=move |e: web_sys::KeyboardEvent| {
                                            let current_cards = cards.get_untracked();
                                            if let Some(pos) = current_cards.iter().position(|&c| c == card) {
                                                let target_idx = match e.key().as_str() {
                                                    "ArrowLeft" | "ArrowUp" => {
                                                        if pos == 0 { current_cards.len() - 1 } else { pos - 1 }
                                                    }
                                                    "ArrowRight" | "ArrowDown" => {
                                                        (pos + 1) % current_cards.len()
                                                    }
                                                    "Home" => 0,
                                                    "End" => current_cards.len() - 1,
                                                    _ => return,
                                                };
                                                e.prevent_default();
                                                let next_card = current_cards[target_idx];
                                                layout.update(|l| {
                                                    if let Some(d) = l.deck_mut(&d_id.get_value()) {
                                                        d.set_active(next_card);
                                                    }
                                                });
                                                layout.get_untracked().save();
                                            }
                                        }
                                    >
                                        <span>{card.title()}</span>
                                        <button
                                            class="deck-tab-detach"
                                            title="Detach tab to canvas"
                                            aria-label="Detach tab"
                                            on:pointerdown=move |e: PointerEvent| {
                                                e.stop_propagation();
                                            }
                                            on:click=move |e: web_sys::MouseEvent| {
                                                e.stop_propagation();
                                                history.update(|h| h.push(layout.get_untracked()));
                                                layout.update(|l| l.detach_from_deck(&d_id.get_value(), card, None));
                                                layout.get_untracked().save();
                                            }
                                        >
                                            <IconExternalLink size=10 />
                                        </button>
                                    </div>
                                }
                            }
                        />
                    </div>
                    <div
                        class="deck-controls"
                        on:pointerdown=move |e: PointerEvent| e.stop_propagation()
                    >
                        <button
                            class="card-control-btn"
                            title="Ungroup deck into separate cards"
                            on:pointerdown=move |e: PointerEvent| e.stop_propagation()
                            on:click=move |e: web_sys::MouseEvent| {
                                e.stop_propagation();
                                history.update(|h| h.push(layout.get_untracked()));
                                layout.update(|l| l.dissolve_deck(&d_id.get_value()));
                                layout.get_untracked().save();
                            }
                        >
                            <IconLayers size=13 />
                        </button>
                        <button
                            class="card-control-btn"
                            class:active=move || {
                                let vm = use_context::<RwSignal<DesktopViewMode>>()
                                    .unwrap_or_else(|| RwSignal::new(DesktopViewMode::Spatial));
                                vm.get() == DesktopViewMode::Focus(DesktopItemId::Deck(d_id.get_value()))
                            }
                            title="Focus deck"
                            aria-label="Focus deck"
                            on:pointerdown=move |e: PointerEvent| e.stop_propagation()
                            on:click=move |e: web_sys::MouseEvent| {
                                e.stop_propagation();
                                let vm = use_context::<RwSignal<DesktopViewMode>>()
                                    .unwrap_or_else(|| RwSignal::new(DesktopViewMode::Spatial));
                                if vm.get() == DesktopViewMode::Focus(DesktopItemId::Deck(d_id.get_value())) {
                                    vm.set(DesktopViewMode::Spatial);
                                } else {
                                    vm.set(DesktopViewMode::Focus(DesktopItemId::Deck(d_id.get_value())));
                                }
                            }
                        >
                            <IconMaximize size=13 />
                        </button>
                        <button
                            class="card-control-btn"
                            title=move || if is_pinned.get() { "Unpin deck" } else { "Pin deck" }
                            on:pointerdown=move |e: PointerEvent| e.stop_propagation()
                            on:click=move |e: web_sys::MouseEvent| {
                                e.stop_propagation();
                                layout.update(|l| l.toggle_deck_pinned(&d_id.get_value()));
                                layout.get_untracked().save();
                            }
                        >
                            <IconPin size=13 />
                        </button>
                        <button
                            class="card-control-btn"
                            title=move || if is_collapsed.get() { "Expand deck" } else { "Collapse deck" }
                            on:pointerdown=move |e: PointerEvent| e.stop_propagation()
                            on:click=move |e: web_sys::MouseEvent| {
                                e.stop_propagation();
                                layout.update(|l| l.toggle_deck_collapse(&d_id.get_value()));
                                layout.get_untracked().save();
                            }
                        >
                            <Show when=move || is_collapsed.get() fallback=|| view! { <IconMinimize size=13 /> }>
                                <IconMaximize size=13 />
                            </Show>
                        </button>
                    </div>
                </header>
                <Show when=move || !is_collapsed.get()>
                    <div class="deck-body">
                        {move || match active_card.get() {
                            CardId::Identity => view! {
                                <strong>"Subject continuity"</strong>
                                <span class="identity-digest">{identity_id()}</span>
                                <span class="identity-badges"><i>{identity_sessions()}" sessions"</i><i>{identity_age()}</i></span>
                                <span class="identity-meta">"Origin "{identity_origin()}" · "{identity_architecture()}</span>
                            }.into_any(),
                            CardId::Session => view! {
                                <strong>"Established trust"</strong>
                                <span class="row"><b>"Mode"</b><i>{runtime_label()}</i></span>
                                <span class="row"><b>"Consumer"</b><i>{session_consumer()}</i></span>
                                <span class="row"><b>"Authenticated"</b><i>{session_auth()}</i></span>
                                <span class="row"><b>"Device bound"</b><i>{session_device()}</i></span>
                                <span class="panel-link">"Established by the gateway"</span>
                            }.into_any(),
                            CardId::Capabilities => view! {
                                <h1>{system_label()}</h1>
                                <span class="capabilities-kind">"Capability health"</span>
                                <footer class="capabilities-meta"><span><small>"Observed"</small><b>{observed_label()}</b></span></footer>
                            }.into_any(),
                            CardId::Journal => view! {
                                <strong>"Canonical Journal"</strong>
                                <span class="row"><b>"Contributions"</b><i>{journal_count()}</i></span>
                                <span class="row"><b>"Erasure epoch"</b><i>{journal_epoch()}</i></span>
                                <span class="row"><b>"Integrity"</b><i>{journal_integrity()}</i></span>
                            }.into_any(),
                            CardId::Lifecycle => view! {
                                <strong>"Lifecycle state"</strong>
                                <span class="row"><b>"Mode"</b><i>{lifecycle_mode()}</i></span>
                                <span class="row"><b>"User activity"</b><i>{lifecycle_activity()}</i></span>
                            }.into_any(),
                            CardId::Commitments => view! {
                                <strong>"Active commitments"</strong>
                                <span class="commitments-meta">{commitments_label()}</span>
                            }.into_any(),
                            CardId::SelfModel => view! {
                                <strong>"Self-model narrative"</strong>
                                <p class="self-narration">{self_narration()}</p>
                            }.into_any(),
                            CardId::Attention => view! {
                                <strong>"Attention focus"</strong>
                                <span class="attention-focus">{attention_focus()}</span>
                            }.into_any(),
                            CardId::Beliefs => view! {
                                <strong>"Beliefs & propositions"</strong>
                                <span class="beliefs-meta">{beliefs_label()}</span>
                            }.into_any(),
                            CardId::Perception => view! {
                                <strong>"Perception facts"</strong>
                                <span class="row"><b>"Status"</b><i>{perception_status()}</i></span>
                            }.into_any(),
                            CardId::Context => view! {
                                <strong>"Associative context"</strong>
                                <span class="context-meta">{context_label()}</span>
                            }.into_any(),
                            CardId::Shell(_) => view! {
                                <strong>"CYBOU Shell"</strong>
                                <span>"Zone 3 Body capability"</span>
                            }.into_any(),
                            CardId::FileManager(_) => view! {
                                <strong>"File Manager"</strong>
                                <span>"Zone 3 Read-Only Storage"</span>
                            }.into_any(),
                            CardId::JournalFeed(_) => view! {
                                <strong>"Event Stream"</strong>
                                <span>"Real-time Journal SSE stream"</span>
                            }.into_any(),
                        }}
                    </div>
                </Show>
                <DeckResizeHandle deck_id=d_id.get_value() layout=layout resizing=resizing />
            </div>
        </Show>
    }
}
