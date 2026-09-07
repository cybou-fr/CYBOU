// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Window management control buttons and resize handles for Cards and Decks.

use leptos::prelude::*;
use web_sys::PointerEvent;

use crate::card::PanelRepresentation;
use crate::{
    CardId, DesktopItemId, DesktopLayout, DesktopViewMode,
    components::icons::{IconClose, IconMaximize, IconMinimize, IconResizeGrip},
    interaction::{ResizeState, start_deck_resize, start_resize},
    tool_state::ToolCardStates,
};

/// Route every user-initiated card close through the same editor and shell lifecycle policy.
pub fn request_close_card(
    card: CardId,
    layout: RwSignal<DesktopLayout>,
    tool_states: ToolCardStates,
) {
    if matches!(card, CardId::Editor(_)) {
        let editor = tool_states.editor(card);
        let has_unsaved = editor
            .tabs
            .get_untracked()
            .iter()
            .any(|tab| tab.dirty || tab.conflict.is_some());
        if has_unsaved {
            editor.card_close_open.set(true);
            return;
        }
    }

    layout.update(|current| {
        current.close_card(card);
    });
    // Closing is the one action that really is a person finished with the card. Everything else
    // that unmounts it deliberately preserves its tool state.
    tool_states.forget(card);
    layout.get_untracked().save();
}

/// Card header window management controls: focus, everything else, and close.
///
/// Three buttons, where there were six. Representation, raise, pin, focus, collapse and close were
/// all equally prominent on every card, which made the header of a panel about a systemd unit as
/// busy as the panel — and put `PanelRepresentation`, a word from this codebase rather than from
/// anybody's job, permanently in front of a person restarting nginx.
///
/// What stayed visible is what is used constantly and what is irreversible enough to want in plain
/// sight. The rest is a click away under `More`, in words that say what they do rather than what
/// they are called internally.
#[expect(
    clippy::too_many_lines,
    reason = "three header buttons and the menu behind one of them; splitting the menu out would put half a control in another file"
)]
#[component]
pub fn CardControls(card: CardId, layout: RwSignal<DesktopLayout>) -> impl IntoView {
    let tool_states = expect_context::<ToolCardStates>();
    let is_pinned = move || layout.get().presentation(card).pinned;
    let is_collapsed = move || layout.get().presentation(card).collapsed;
    let representation = move || layout.get().presentation(card).representation;
    let view_mode = use_context::<RwSignal<DesktopViewMode>>()
        .unwrap_or_else(|| RwSignal::new(DesktopViewMode::Spatial));
    let is_focused = move || view_mode.get() == DesktopViewMode::Focus(DesktopItemId::Card(card));
    let menu_open = RwSignal::new(false);
    let in_deck = move || layout.get().is_in_deck(card);

    let set_representation = move |next: PanelRepresentation| {
        layout.update(|current| current.set_representation(card, next));
        layout.get_untracked().save();
        menu_open.set(false);
    };

    view! {
        <div class="card-controls" on:pointerdown=move |e: PointerEvent| e.stop_propagation() on:click=move |e: web_sys::MouseEvent| e.stop_propagation()>
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
                class:active=move || menu_open.get()
                class="card-control-btn more-btn"
                title="More"
                aria-label="More card actions"
                aria-haspopup="menu"
                aria-expanded=move || menu_open.get().to_string()
                on:click=move |_| menu_open.update(|open| *open = !*open)
            >
                <lucide_leptos::Ellipsis size=12 />
            </button>
            {if card.spec().closable {
                view! {
                    <button
                        class="card-control-btn close-btn"
                        title="Close card"
                        aria-label="Close card"
                        on:click=move |_| {
                            request_close_card(card, layout, tool_states);
                        }
                    >
                        <IconClose size=12 />
                    </button>
                }.into_any()
            } else {
                // A card that cannot be closed shows no close button. Detaching from a deck, which
                // used to sit here for exactly those cards, is in the menu with the rest.
                ().into_any()
            }}

            <Show when=move || menu_open.get()>
                // Anything outside the menu dismisses it, including the rest of the card. A menu
                // that could only be closed by the button that opened it leaves a person who
                // clicked it by accident with something to work out.
                <div
                    class="card-menu-backdrop"
                    role="presentation"
                    on:click=move |_| menu_open.set(false)
                ></div>
                <div class="card-menu" role="menu" aria-label="Card actions">
                    <button
                        class="card-menu-item"
                        role="menuitem"
                        on:click=move |_| {
                            layout.update(|current| {
                                let pinned = current.presentation(card).pinned;
                                current.set_pinned(card, !pinned);
                            });
                            layout.get_untracked().save();
                            menu_open.set(false);
                        }
                    >
                        {move || if is_pinned() { "Unpin position" } else { "Pin position" }}
                    </button>
                    <button
                        class="card-menu-item"
                        role="menuitem"
                        on:click=move |_| {
                            layout.update(|current| {
                                let collapsed = current.presentation(card).collapsed;
                                current.set_collapsed(card, !collapsed);
                            });
                            layout.get_untracked().save();
                            menu_open.set(false);
                        }
                    >
                        {move || if is_collapsed() { "Expand" } else { "Collapse" }}
                    </button>
                    <button
                        class="card-menu-item"
                        role="menuitem"
                        on:click=move |_| {
                            layout.update(|current| current.bring_forward(card));
                            layout.get_untracked().save();
                            menu_open.set(false);
                        }
                    >
                        "Bring to front"
                    </button>
                    <div class="card-menu-separator"></div>
                    // Named "Panel size" rather than "Representation". The three sizes are a thing
                    // a person can see; the word for them is this codebase's, not theirs.
                    <span class="card-menu-label">"Panel size"</span>
                    {[
                        PanelRepresentation::Glance,
                        PanelRepresentation::Standard,
                        PanelRepresentation::Expanded,
                    ]
                        .into_iter()
                        .map(|size| {
                            view! {
                                <button
                                    class="card-menu-item"
                                    class:selected=move || representation() == size
                                    role="menuitemradio"
                                    aria-checked=move || (representation() == size).to_string()
                                    on:click=move |_| set_representation(size)
                                >
                                    {size.label()}
                                </button>
                            }
                        })
                        .collect_view()}
                    <Show when=in_deck>
                        <div class="card-menu-separator"></div>
                        <button
                            class="card-menu-item"
                            role="menuitem"
                            on:click=move |_| {
                                layout.update(|l| {
                                    if let Some(d) = l.deck_for_card(card) {
                                        let d_id = d.id.clone();
                                        l.detach_from_deck(&d_id, card, None);
                                    }
                                });
                                layout.get_untracked().save();
                                menu_open.set(false);
                            }
                        >
                            "Detach from deck"
                        </button>
                    </Show>
                </div>
            </Show>
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
