// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Everything this desktop can open, behind one button.
//!
//! The catalogue itself is [`crate::applications`], deliberately outside `components` so that what
//! is reachable can be checked without a browser. This draws it: headings in the order the
//! categories are declared, and one row per application.
//!
//! It is a catalogue and not a search. Ctrl+K already searches everything — anchors, clusters,
//! notes, cards, relations — and answers a question a person can phrase. This answers the one they
//! cannot: what is there, when they do not yet know what it is called.

use leptos::prelude::*;

use crate::applications::{LauncherCategory, applications_in};
use crate::card::CardId;
use crate::{DesktopItemId, DesktopLayout};

/// The application launcher panel.
#[component]
pub fn Launcher(
    layout: RwSignal<DesktopLayout>,
    set_selected: WriteSignal<Option<DesktopItemId>>,
    open: RwSignal<bool>,
) -> impl IntoView {
    let pan = use_context::<ReadSignal<(f64, f64)>>();
    let zoom = use_context::<ReadSignal<f64>>();

    let launch = move |card: CardId| {
        open_or_focus(layout, set_selected, pan, zoom, card);
        open.set(false);
    };

    view! {
        <Show when=move || open.get()>
            // The backdrop is what closes this. A launcher that could only be dismissed by the
            // button that opened it is a launcher people learn to avoid opening.
            <div
                class="launcher-backdrop"
                role="presentation"
                on:click=move |_| open.set(false)
            ></div>
            <section
                class="launcher"
                role="dialog"
                aria-label="Applications"
                aria-modal="false"
                on:keydown=move |ev| {
                    if ev.key() == "Escape" {
                        open.set(false);
                    }
                }
            >
                <header class="launcher-header">
                    <span class="launcher-title">"Applications"</span>
                    <span class="launcher-hint">"Ctrl+K to search"</span>
                </header>
                <div class="launcher-body">
                    {LauncherCategory::ALL
                        .into_iter()
                        .map(|category| {
                            let entries = applications_in(category);
                            view! {
                                <div class="launcher-category">
                                    <h3 class="launcher-category-title">{category.title()}</h3>
                                    <div class="launcher-grid">
                                        {entries
                                            .into_iter()
                                            .map(|application| {
                                                let card = application.card;
                                                view! {
                                                    <button
                                                        class="launcher-entry"
                                                        type="button"
                                                        title=card.title()
                                                        on:click=move |_| launch(card)
                                                    >
                                                        {card.title()}
                                                    </button>
                                                }
                                            })
                                            .collect_view()}
                                    </div>
                                </div>
                            }
                        })
                        .collect_view()}
                </div>
            </section>
        </Show>
    }
}

/// Open a card, or bring the one that is already there forward.
///
/// The same rule the Dock and the command palette use, and it lives here rather than in each of
/// them: where a card opens is decided when it opens, from what is on the canvas and what the
/// window is showing, so that two callers cannot disagree about where Operations lands.
pub fn open_or_focus(
    layout: RwSignal<DesktopLayout>,
    set_selected: WriteSignal<Option<DesktopItemId>>,
    pan: Option<ReadSignal<(f64, f64)>>,
    zoom: Option<ReadSignal<f64>>,
    card: CardId,
) {
    if layout.get_untracked().contains_card(card) {
        if layout.get_untracked().presentation(card).collapsed {
            layout.update(|layout| layout.set_collapsed(card, false));
        }
    } else {
        let view = crate::interaction::visible_canvas_rect(
            pan.map_or((0.0, 0.0), |signal| signal.get_untracked()),
            zoom.map_or(1.0, |signal| signal.get_untracked()),
        );
        let spot = layout
            .get_untracked()
            .free_spot_in(card.spec().default_size, view);
        layout.update(|layout| layout.open_card(card, spot.0, spot.1));
    }
    layout.update(|layout| layout.bring_forward(card));
    set_selected.set(Some(DesktopItemId::Card(card)));
    layout.get_untracked().save();
}
