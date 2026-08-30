// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The card that draws whatever the layout is holding and nothing else has claimed.
//!
//! The viewport used to name every card it could draw, one component at a time. Six dynamic kinds
//! had a wrapper; the rest — Services, Processes, System Logs, Storage, Network, Packages, Updates,
//! Users, Security, Backup, Mail, Calendar, Notes, Contacts, the Cognitive Graph, the Event
//! Journal, Meaning, Learning, Operations, Notifications — did not, and were reachable from the
//! Dock and the command palette anyway.
//!
//! So opening one added it to the layout, moved the selection onto it, saved, and drew nothing.
//! Not an error, not an empty panel: nothing at all, on a desktop that had just been told to open
//! it. The same card tabbed into a Deck drew perfectly, because a Deck renders whatever
//! [`CardContent`](super::content::CardContent) can dispatch, which was every kind all along.
//!
//! This renders the same way for a card standing on its own. A new card kind is now a `CardId`
//! variant and a `CardContent` arm; it does not also need somebody to remember the viewport.

use std::sync::Arc;

use leptos::prelude::*;

use crate::{
    CardId, DesktopItemId, DesktopLayout,
    components::{card_frame::CardFrame, cards::content::CardContent, icons::IconLayers},
    interaction::{DragState, ResizeState},
    state::RuntimeState,
};

/// One card of any kind, in the frame every card gets.
#[component]
pub fn GenericToolCard(
    card: CardId,
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<Option<DesktopItemId>>,
    set_selected: WriteSignal<Option<DesktopItemId>>,
    dragging: RwSignal<Option<DragState>>,
    resizing: RwSignal<Option<ResizeState>>,
    auth_modal_open: RwSignal<bool>,
    runtime: RwSignal<RuntimeState>,
) -> impl IntoView {
    // The card's own title, so a panel opened from the Dock is labelled the way the Dock labelled
    // it. A generic renderer that said "Card" would make every one of these look like the same
    // thing collapsed.
    let title = card.title();

    let collapsed = move || {
        view! {
            <div class="card-collapsed-summary">
                <b>{title}</b>
            </div>
        }
        .into_any()
    };

    view! {
        <CardFrame
            card=card
            layout=layout
            selected=selected
            set_selected=set_selected
            dragging=dragging
            resizing=resizing
            kicker_title=title
            kicker_icon=Arc::new(|| view! { <IconLayers size=14 /> }.into_any())
            collapsed_summary=Arc::new(collapsed)
        >
            <CardContent card=card runtime=runtime auth_modal_open=auth_modal_open />
        </CardFrame>
    }
}
