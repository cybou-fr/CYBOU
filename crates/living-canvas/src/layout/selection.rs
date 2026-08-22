// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! What "the selected thing" resolves to, and where it is.
//!
//! Selection used to be a `&'static str` holding a card's key. A key names a *kind*: `Shell(0)`,
//! `Shell(1)` and `Shell(2)` all answer `"shell"`. So clicking one Shell card selected every Shell
//! card at once, and the action attached to the selection resolved that key back through
//! `CardId::from_key`, which answers `Shell(0)` — clicking the third Shell brought the first one
//! forward.
//!
//! Selection is a [`DesktopItemId`] now, which is what the layout has always used to distinguish
//! one thing on the desktop from another, decks included. `key()` stays what it always was: a name
//! for a kind, useful for CSS and for routing, and never an identity.
//!
//! This lives in `layout` rather than beside the component that draws the button because it is
//! ordinary arithmetic over the layout, and it is the arithmetic that was wrong.

use crate::layout::engine::DesktopLayout;
use crate::layout::model::{DesktopItemId, Rect};

/// Where the selected item is, if anything selectable is selected.
///
/// `None` covers three cases that are one answer: nothing is selected, the selected card was
/// closed, or it was docked into a deck and is no longer a top-level item. An action bar has
/// nothing to attach to in any of them.
#[must_use]
pub fn selected_rect(layout: &DesktopLayout, selected: Option<&DesktopItemId>) -> Option<Rect> {
    match selected? {
        DesktopItemId::Card(card) => {
            if !layout.contains_card(*card) || layout.is_in_deck(*card) {
                return None;
            }
            let geometry = layout.geometry(*card);
            let presentation = layout.presentation(*card);
            let height = if presentation.collapsed {
                44.0
            } else {
                geometry.height
            };
            Some(Rect::new(geometry.x, geometry.y, geometry.width, height))
        }
        DesktopItemId::Deck(deck) => layout.deck(deck).map(|deck| {
            let height = if deck.presentation.collapsed {
                44.0
            } else {
                deck.geometry.height
            };
            Rect::new(
                deck.geometry.x,
                deck.geometry.y,
                deck.geometry.width,
                height,
            )
        }),
    }
}

/// The stacking order of the selected item, for placing anything drawn against it.
#[must_use]
pub fn selected_z(layout: &DesktopLayout, selected: Option<&DesktopItemId>) -> u32 {
    match selected {
        Some(DesktopItemId::Card(card)) => layout.geometry(*card).z,
        Some(DesktopItemId::Deck(deck)) => layout.deck(deck).map_or(0, |deck| deck.geometry.z),
        None => 0,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::CardId;

    fn desktop() -> DesktopLayout {
        let mut layout = DesktopLayout::canonical(None);
        layout.open_card(CardId::Shell(0), 100.0, 100.0);
        layout.open_card(CardId::Shell(2), 900.0, 700.0);
        layout
    }

    #[test]
    fn selecting_the_third_shell_does_not_resolve_to_the_first() {
        // The bug this module exists for. Both cards answer `"shell"` to `key()`, so a selection
        // held as a key resolved to `Shell(0)` whichever one a person had clicked.
        let layout = desktop();
        let first = selected_rect(&layout, Some(&DesktopItemId::Card(CardId::Shell(0))))
            .expect("the first shell");
        let third = selected_rect(&layout, Some(&DesktopItemId::Card(CardId::Shell(2))))
            .expect("the third shell");

        assert_ne!(
            (first.x, first.y),
            (third.x, third.y),
            "two shell cards resolved to the same place"
        );
        assert_eq!((third.x, third.y), (900.0, 700.0));
    }

    #[test]
    fn a_card_that_is_not_on_the_desktop_is_not_selectable() {
        let mut layout = DesktopLayout::canonical(None);
        assert!(selected_rect(&layout, Some(&DesktopItemId::Card(CardId::Shell(0)))).is_none());

        layout.open_card(CardId::Shell(0), 10.0, 10.0);
        assert!(selected_rect(&layout, Some(&DesktopItemId::Card(CardId::Shell(0)))).is_some());
    }

    #[test]
    fn a_docked_card_is_not_a_top_level_selection() {
        // Invariant L8: a card inside a deck is not a desktop item. Anything drawn against it
        // would be drawn where the card is not.
        let mut layout = DesktopLayout::canonical(None);
        layout
            .create_deck("Pair", vec![CardId::Identity, CardId::Session], 60.0, 60.0)
            .expect("a deck");
        assert!(selected_rect(&layout, Some(&DesktopItemId::Card(CardId::Identity))).is_none());
    }

    #[test]
    fn a_deck_is_selectable_in_its_own_right() {
        let mut layout = DesktopLayout::canonical(None);
        let deck = layout
            .create_deck("Pair", vec![CardId::Identity, CardId::Session], 60.0, 60.0)
            .expect("a deck");
        let rect =
            selected_rect(&layout, Some(&DesktopItemId::Deck(deck.clone()))).expect("the deck");
        assert_eq!((rect.x, rect.y), (60.0, 60.0));
    }

    #[test]
    fn nothing_selected_resolves_to_nothing() {
        let layout = desktop();
        assert!(selected_rect(&layout, None).is_none());
    }

    #[test]
    fn a_collapsed_selection_is_the_height_it_is_drawn_at() {
        let mut layout = desktop();
        layout.set_collapsed(CardId::Shell(0), true);
        let rect = selected_rect(&layout, Some(&DesktopItemId::Card(CardId::Shell(0))))
            .expect("the collapsed shell");
        assert_eq!(rect.height, 44.0);
    }
}
