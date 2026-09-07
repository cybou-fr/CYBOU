// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! What this desktop can open, and which few of those are worth a permanent place.
//!
//! The Dock used to be the catalogue: thirty-one buttons, written out one after another, answering
//! the question "what exists in this system?". That is a real question and it deserves an answer,
//! but it is not the question a dock answers. A dock answers "what do I reach for constantly", and
//! a dock that answers the other one is a launcher with no launcher behind it — every application
//! equally prominent, which is the same as none of them being.
//!
//! So the two are separated here. [`DOCK_APPLICATIONS`] is the short list a person reaches for;
//! [`ALL_APPLICATIONS`] is everything, grouped, behind one button. Both are data rather than markup
//! so that a card cannot be added to one and forgotten in the other — there is a test below that
//! says so, and it runs natively, unlike anything in `components`.

use crate::card::CardId;

/// Where an application belongs in the launcher.
///
/// Ordered as the launcher shows them, and that order is a claim about who is using this: the
/// machine first, the work on it second, CYBOU's own faculties third, and this person's own things
/// last. Somebody restarting a unit should not have to read past the Cognitive Graph to find it.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum LauncherCategory {
    /// Managing the machine itself.
    Server,
    /// Working on what is on it.
    Work,
    /// What CYBOU is and what it is doing on somebody's behalf.
    Cybou,
    /// This person's own mail, time and notes.
    Personal,
    /// What Mind holds, for a reader who wants to look.
    Mind,
}

impl LauncherCategory {
    /// The heading this category is shown under.
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Server => "Server",
            Self::Work => "Work",
            Self::Cybou => "CYBOU",
            Self::Personal => "Personal",
            Self::Mind => "Mind",
        }
    }

    /// Every category, in the order the launcher lists them.
    pub const ALL: [Self; 5] = [
        Self::Server,
        Self::Work,
        Self::Cybou,
        Self::Personal,
        Self::Mind,
    ];
}

/// One thing a person can open, and where it is filed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Application {
    /// The card this opens.
    pub card: CardId,
    /// Where the launcher files it.
    pub category: LauncherCategory,
}

/// The applications that keep a permanent place in the Dock.
///
/// Six, and the argument for each is that a server operator reaches for it without deciding to:
/// where they are (Home), their files, a shell, what is running, and what it is saying. Everything
/// else is one click further away, which is the correct distance for something used once a week.
///
/// Editor and Diff are deliberately not here. They are opened from a file, not from a dock, and a
/// person who wants one without a file has the launcher and the command palette.
pub const DOCK_APPLICATIONS: [CardId; 5] = [
    CardId::FileManager(0),
    CardId::Terminal(0),
    CardId::Services(0),
    CardId::SystemLogs(0),
    CardId::Monitor(0),
];

/// Everything this desktop can open, grouped for the launcher.
pub const ALL_APPLICATIONS: &[Application] = &[
    // The machine.
    app(CardId::Services(0), LauncherCategory::Server),
    app(CardId::Processes(0), LauncherCategory::Server),
    app(CardId::Monitor(0), LauncherCategory::Server),
    app(CardId::SystemLogs(0), LauncherCategory::Server),
    app(CardId::Storage(0), LauncherCategory::Server),
    app(CardId::Network(0), LauncherCategory::Server),
    app(CardId::Packages(0), LauncherCategory::Server),
    app(CardId::Updates(0), LauncherCategory::Server),
    app(CardId::UserSettings(0), LauncherCategory::Server),
    app(CardId::Security(0), LauncherCategory::Server),
    app(CardId::Backup(0), LauncherCategory::Server),
    // The work done on it.
    app(CardId::FileManager(0), LauncherCategory::Work),
    app(CardId::Editor(0), LauncherCategory::Work),
    app(CardId::Diff(0), LauncherCategory::Work),
    app(CardId::Terminal(0), LauncherCategory::Work),
    app(CardId::Inspector(0), LauncherCategory::Work),
    // What CYBOU is doing, and what it makes of the host.
    app(CardId::Insight, LauncherCategory::Cybou),
    app(CardId::Operations(0), LauncherCategory::Cybou),
    app(CardId::Notifications(0), LauncherCategory::Cybou),
    app(CardId::Agents, LauncherCategory::Cybou),
    app(CardId::Meaning(0), LauncherCategory::Cybou),
    app(CardId::Outline, LauncherCategory::Cybou),
    // This person's own things.
    app(CardId::Mail(0), LauncherCategory::Personal),
    app(CardId::Calendar(0), LauncherCategory::Personal),
    app(CardId::Notes(0), LauncherCategory::Personal),
    app(CardId::Contacts(0), LauncherCategory::Personal),
    // What Mind holds. Reachable, and behind the work rather than in front of it: restarting a unit
    // must not require reading CYBOU's neuroanatomy first.
    app(CardId::Identity, LauncherCategory::Mind),
    app(CardId::Session, LauncherCategory::Mind),
    app(CardId::Capabilities, LauncherCategory::Mind),
    app(CardId::Journal, LauncherCategory::Mind),
    app(CardId::Lifecycle, LauncherCategory::Mind),
    app(CardId::Commitments, LauncherCategory::Mind),
    app(CardId::SelfModel, LauncherCategory::Mind),
    app(CardId::Attention, LauncherCategory::Mind),
    app(CardId::Beliefs, LauncherCategory::Mind),
    app(CardId::Perception, LauncherCategory::Mind),
    app(CardId::Context, LauncherCategory::Mind),
    app(CardId::Disclosure, LauncherCategory::Mind),
    app(CardId::CognitiveGraph(0), LauncherCategory::Mind),
    app(CardId::EventJournal(0), LauncherCategory::Mind),
    app(CardId::JournalFeed(0), LauncherCategory::Mind),
    app(CardId::Learning(0), LauncherCategory::Mind),
];

/// One catalogue entry.
const fn app(card: CardId, category: LauncherCategory) -> Application {
    Application { card, category }
}

/// What the launcher shows under one heading, in catalogue order.
#[must_use]
pub fn applications_in(category: LauncherCategory) -> Vec<Application> {
    ALL_APPLICATIONS
        .iter()
        .filter(|application| application.category == category)
        .copied()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_application_names_a_card_that_can_actually_be_drawn_and_restored() {
        for application in ALL_APPLICATIONS {
            let card = application.card;
            assert_eq!(
                CardId::from_key(card.key()),
                Some(card),
                "{card:?} is filed in the launcher under a key that does not resolve back to it"
            );
            assert!(
                !card.title().is_empty(),
                "{card:?} would appear in the launcher as a blank row"
            );
        }
    }

    #[test]
    fn nothing_is_filed_twice() {
        let mut seen: Vec<CardId> = Vec::new();
        for application in ALL_APPLICATIONS {
            assert!(
                !seen.contains(&application.card),
                "{:?} appears in the launcher more than once",
                application.card
            );
            seen.push(application.card);
        }
    }

    #[test]
    fn everything_in_the_dock_is_also_in_the_launcher() {
        // The dock is a shortcut, never the only way to something. A person who has never noticed
        // the dock, or who is on a window too narrow to show it, still reaches all of it.
        for card in DOCK_APPLICATIONS {
            assert!(
                ALL_APPLICATIONS
                    .iter()
                    .any(|application| application.card == card),
                "{card:?} is in the dock and nowhere else"
            );
        }
    }

    #[test]
    fn every_mind_card_is_reachable_from_the_launcher() {
        // The first visit stopped opening most of these. That is only acceptable while every one of
        // them is still one click away, and this is the check that keeps it true: a System card
        // that opens on no first visit and appears in no launcher is a card nobody can reach.
        for card in CardId::ALL_SYSTEM_CARDS {
            assert!(
                ALL_APPLICATIONS
                    .iter()
                    .any(|application| application.card == card),
                "{card:?} exists, opens on no first visit, and is in no launcher"
            );
        }
    }

    #[test]
    fn every_category_has_something_in_it() {
        for category in LauncherCategory::ALL {
            assert!(
                !applications_in(category).is_empty(),
                "{category:?} would render as an empty heading"
            );
        }
    }
}
