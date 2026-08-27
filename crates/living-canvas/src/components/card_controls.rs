// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Window management control buttons and resize handles for Cards and Decks.

use leptos::prelude::*;
use web_sys::PointerEvent;

use leptos::task::spawn_local;

use crate::{
    CardId, DesktopItemId, DesktopLayout, DesktopViewMode, GatewayMindClient, MindClient,
    components::icons::{
        IconClose, IconExternalLink, IconLayers, IconMaximize, IconMinimize, IconPin,
        IconResizeGrip,
    },
    interaction::{ResizeState, start_deck_resize, start_resize},
    tool_state::ToolCardStates,
};

/// Card header window management controls (Pin, Representation, Focus, Collapse/Expand, Close/Detach).
#[component]
pub fn CardControls(card: CardId, layout: RwSignal<DesktopLayout>) -> impl IntoView {
    let tool_states = expect_context::<ToolCardStates>();
    let is_pinned = move || layout.get().presentation(card).pinned;
    let is_collapsed = move || layout.get().presentation(card).collapsed;
    let representation = move || layout.get().presentation(card).representation;
    let view_mode = use_context::<RwSignal<DesktopViewMode>>()
        .unwrap_or_else(|| RwSignal::new(DesktopViewMode::Spatial));
    let is_focused = move || view_mode.get() == DesktopViewMode::Focus(DesktopItemId::Card(card));

    view! {
        <div class="card-controls" on:pointerdown=move |e: PointerEvent| e.stop_propagation() on:click=move |e: web_sys::MouseEvent| e.stop_propagation()>
            <button
                class="card-control-btn representation-btn"
                title=move || format!("Panel mode: {} (Click to cycle)", representation().label())
                aria-label=move || format!("Panel mode: {}", representation().label())
                on:click=move |_| {
                    layout.update(|current| {
                        let p = current.presentation(card);
                        current.set_representation(card, p.representation.cycle());
                    });
                    layout.get_untracked().save();
                }
            >
                <IconLayers size=12 />
            </button>
            <button
                class:active=is_pinned
                class="card-control-btn pin-btn"
                title=move || if is_pinned() { "Unpin card" } else { "Pin card (lock position)" }
                aria-label=move || if is_pinned() { "Unpin card" } else { "Pin card" }
                on:click=move |_| {
                    layout.update(|current| {
                        let p = current.presentation(card);
                        current.set_pinned(card, !p.pinned);
                    });
                    layout.get_untracked().save();
                }
            >
                <IconPin size=12 />
            </button>
            <button
                class:active=is_focused
                class="card-control-btn focus-btn"
                title=move || if is_focused() { "Leave focus" } else { "Focus card" }
                aria-label=move || if is_focused() { "Leave focus" } else { "Focus card" }
                on:click=move |_| {
                    if is_focused() {
                        view_mode.set(DesktopViewMode::Spatial);
                    } else {
                        view_mode.set(DesktopViewMode::Focus(DesktopItemId::Card(card)));
                    }
                }
            >
                {move || if is_focused() {
                    view! { <IconMinimize size=12 /> }.into_any()
                } else {
                    view! { <IconMaximize size=12 /> }.into_any()
                }}
            </button>
            <button
                class:active=is_collapsed
                class="card-control-btn collapse-btn"
                title=move || if is_collapsed() { "Expand card" } else { "Collapse card" }
                aria-label=move || if is_collapsed() { "Expand card" } else { "Collapse card" }
                on:click=move |_| {
                    layout.update(|current| {
                        let p = current.presentation(card);
                        current.set_collapsed(card, !p.collapsed);
                    });
                    layout.get_untracked().save();
                }
            >
                <IconMinimize size=12 />
            </button>
            {if card.spec().closable {
                view! {
                    <button
                        class="card-control-btn close-btn"
                        title="Close card"
                        aria-label="Close card"
                        on:click=move |_| {
                            if matches!(card, CardId::Editor(_)) {
                                let editor = tool_states.editor(card);
                                let has_unsaved = editor.tabs.get_untracked().iter().any(|tab| {
                                    tab.dirty || tab.conflict.is_some()
                                });
                                if has_unsaved {
                                    editor.card_close_open.set(true);
                                    return;
                                }
                            }
                            layout.update(|current| {
                                current.close_card(card);
                            });
                            // Closing is the one action that really is a person finished with the
                            // card. Everything else that unmounts it — collapsing, switching a deck
                            // tab, docking — deliberately does not reach here.
                            tool_states.forget(card);
                            // The shell behind it goes too. Left standing, it would still be in the
                            // directory this card was in, and the next card opened at the same
                            // number would show `/` and then jump there on the first command.
                            if let CardId::Shell(instance) = card {
                                spawn_local(async move {
                                    let _ = GatewayMindClient.close_shell(instance).await;
                                });
                            }
                            layout.get_untracked().save();
                        }
                    >
                        <IconClose size=12 />
                    </button>
                }.into_any()
            } else {
                let in_deck = move || layout.get().is_in_deck(card);
                view! {
                    <Show when=in_deck>
                        <button
                            class="card-control-btn detach-btn"
                            title="Detach from Deck"
                            aria-label="Detach from Deck"
                            on:click=move |_| {
                                layout.update(|l| {
                                    if let Some(d) = l.deck_for_card(card) {
                                        let d_id = d.id.clone();
                                        l.detach_from_deck(&d_id, card, None);
                                    }
                                });
                                layout.get_untracked().save();
                            }
                        >
                            <IconExternalLink size=12 />
                        </button>
                    </Show>
                }.into_any()
            }}
        </div>
    }
}

/// Interactive resize grip for standalone Cards.
#[component]
pub fn CardResizeHandle(
    card: CardId,
    layout: RwSignal<DesktopLayout>,
    resizing: RwSignal<Option<ResizeState>>,
) -> impl IntoView {
    let is_collapsed = move || layout.get().presentation(card).collapsed;
    view! {
        <Show when=move || !is_collapsed()>
            <div
                class="card-resize-handle"
                title="Resize"
                aria-label="Resize card"
                on:pointerdown=move |event| start_resize(event, card, layout, resizing)
                on:click=move |e: web_sys::MouseEvent| e.stop_propagation()
            >
                <IconResizeGrip />
            </div>
        </Show>
    }
}

/// Interactive resize grip for Deck grouping containers.
#[component]
pub fn DeckResizeHandle(
    deck_id: String,
    layout: RwSignal<DesktopLayout>,
    resizing: RwSignal<Option<ResizeState>>,
) -> impl IntoView {
    let d_id = deck_id.clone();
    let d_id_res = deck_id;
    let is_collapsed = move || {
        layout
            .get()
            .deck(&d_id)
            .is_some_and(|d| d.presentation.collapsed)
    };
    view! {
        <Show when=move || !is_collapsed()>
            {
                let d_id_click = d_id_res.clone();
                view! {
                    <div
                        class="card-resize-handle"
                        title="Resize deck"
                        aria-label="Resize deck"
                        on:pointerdown=move |event| start_deck_resize(event, d_id_click.clone(), layout, resizing)
                        on:click=move |e: web_sys::MouseEvent| e.stop_propagation()
                    >
                        <IconResizeGrip />
                    </div>
                }
            }
        </Show>
    }
}
