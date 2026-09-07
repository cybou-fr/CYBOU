// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! How a person moves through a directory, and what they may do with what is under the cursor.
//!
//! Separated from the panel that draws it for the same reason [`crate::spatial`] is: this is the
//! part with rules in it, and rules that can only be exercised by clicking are rules nobody checks.
//! Which entry the arrow keys reach, and which actions a right-click offers on a folder as opposed
//! to a file, are decided here and tested natively.
//!
//! Nothing here reaches a file. It is a function of the names the panel was given.

/// One thing a person can ask for on the entry under the cursor.
///
/// Only actions that exist. A menu is a promise, and an entry that opens a dialog leading nowhere
/// is worse than an entry that is absent: the absent one costs a person a search, and the present
/// one costs them their belief that the menu means anything.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntryAction {
    /// Go into a folder, or preview a file.
    Open,
    /// Open a file in the editor.
    OpenInEditor,
    /// Start a terminal already in this folder.
    TerminalHere,
    /// Save a copy of a file to the machine the browser is on.
    Download,
    /// Give it a different name.
    Rename,
    /// Remove it.
    Delete,
}

impl EntryAction {
    /// The words on the menu row.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Open => "Open",
            Self::OpenInEditor => "Open in Editor",
            Self::TerminalHere => "Terminal here",
            Self::Download => "Download",
            Self::Rename => "Rename",
            Self::Delete => "Delete",
        }
    }

    /// Whether this one cannot be taken back, and is drawn as such.
    #[must_use]
    pub const fn is_destructive(self) -> bool {
        matches!(self, Self::Delete)
    }

    /// Whether a separator is drawn above this one, grouping the menu by what the actions do.
    ///
    /// Opening, transferring and destroying are three different intentions, and a delete sitting
    /// directly under a rename is a delete somebody reaches by overshooting.
    #[must_use]
    pub const fn opens_group(self) -> bool {
        matches!(self, Self::Download | Self::Delete)
    }
}

/// What a right-click offers on one entry.
///
/// `transfers_available` is the panel's own answer about the location domain it is showing: byte
/// transfer exists in some of them and not others, and offering a download that the domain has no
/// route for would be the promise this type exists to avoid making.
#[must_use]
pub fn actions_for(is_dir: bool, transfers_available: bool) -> Vec<EntryAction> {
    let mut actions = vec![EntryAction::Open];
    if is_dir {
        actions.push(EntryAction::TerminalHere);
    } else {
        actions.push(EntryAction::OpenInEditor);
        if transfers_available {
            actions.push(EntryAction::Download);
        }
    }
    actions.push(EntryAction::Rename);
    actions.push(EntryAction::Delete);
    actions
}

/// Where the cursor lands after an arrow key, given the entries as they are currently shown.
///
/// Ordered by what is on screen rather than by what the directory holds, because a filtered list is
/// what a person is looking at and moving through a hidden entry would move the highlight nowhere
/// visible. Stops at the ends rather than wrapping: a list that jumps from the last name to the
/// first loses somebody who was holding the key down.
///
/// An empty listing has nowhere to go, and a cursor on a name that is no longer there — a rename, a
/// delete, a filter typed since — starts again from the end the key was heading towards.
#[must_use]
pub fn move_cursor(shown: &[String], cursor: Option<&str>, delta: i32) -> Option<String> {
    if shown.is_empty() {
        return None;
    }
    let last = shown.len() - 1;
    let current = cursor.and_then(|name| shown.iter().position(|entry| entry == name));
    let step = delta.unsigned_abs() as usize;
    let next = match current {
        Some(index) if delta < 0 => index.saturating_sub(step),
        Some(index) => (index + step).min(last),
        None if delta < 0 => last,
        None => 0,
    };
    shown.get(next).cloned()
}

/// The path a name under `parent` refers to, without inventing a double slash at the root.
#[must_use]
pub fn child_path(parent: &str, name: &str) -> String {
    if parent == "/" {
        format!("/{name}")
    } else {
        format!("{parent}/{name}")
    }
}

/// The directory above this one, or `None` at the top of the domain.
#[must_use]
pub fn parent_path(path: &str) -> Option<String> {
    if path == "/" || path.is_empty() {
        return None;
    }
    let trimmed = path.trim_end_matches('/');
    match trimmed.rfind('/') {
        None | Some(0) => Some("/".to_owned()),
        Some(index) => Some(trimmed[..index].to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn a_folder_and_a_file_are_not_offered_the_same_things() {
        assert_eq!(
            actions_for(true, true),
            vec![
                EntryAction::Open,
                EntryAction::TerminalHere,
                EntryAction::Rename,
                EntryAction::Delete
            ]
        );
        assert_eq!(
            actions_for(false, true),
            vec![
                EntryAction::Open,
                EntryAction::OpenInEditor,
                EntryAction::Download,
                EntryAction::Rename,
                EntryAction::Delete
            ]
        );
    }

    #[test]
    fn a_domain_that_cannot_transfer_bytes_is_not_offered_a_download() {
        // The one promise this menu could make and not keep. Home has no byte route today, and an
        // entry that opened a dialog leading nowhere would cost a person their belief in the menu.
        assert!(!actions_for(false, false).contains(&EntryAction::Download));
        assert!(actions_for(false, false).contains(&EntryAction::OpenInEditor));
    }

    #[test]
    fn the_cursor_moves_through_what_is_on_screen_and_stops_at_the_ends() {
        let shown = names(&["a", "b", "c"]);
        assert_eq!(move_cursor(&shown, None, 1).as_deref(), Some("a"));
        assert_eq!(move_cursor(&shown, None, -1).as_deref(), Some("c"));
        assert_eq!(move_cursor(&shown, Some("a"), 1).as_deref(), Some("b"));
        assert_eq!(move_cursor(&shown, Some("c"), 1).as_deref(), Some("c"));
        assert_eq!(move_cursor(&shown, Some("a"), -1).as_deref(), Some("a"));
        assert_eq!(move_cursor(&[], Some("a"), 1), None);
    }

    #[test]
    fn a_cursor_on_something_that_is_no_longer_shown_starts_again() {
        // A rename, a delete, or a filter typed since. The highlight has to land somewhere, and
        // where it lands follows the direction the key was going.
        let shown = names(&["a", "b", "c"]);
        assert_eq!(move_cursor(&shown, Some("gone"), 1).as_deref(), Some("a"));
        assert_eq!(move_cursor(&shown, Some("gone"), -1).as_deref(), Some("c"));
    }

    #[test]
    fn a_path_under_the_root_has_one_slash_in_it() {
        assert_eq!(child_path("/", "etc"), "/etc");
        assert_eq!(child_path("/home/ada", "notes"), "/home/ada/notes");
    }

    #[test]
    fn going_up_stops_at_the_root_rather_than_above_it() {
        assert_eq!(parent_path("/home/ada/notes").as_deref(), Some("/home/ada"));
        assert_eq!(parent_path("/home").as_deref(), Some("/"));
        assert_eq!(parent_path("/home/").as_deref(), Some("/"));
        assert_eq!(parent_path("/"), None);
        assert_eq!(parent_path(""), None);
    }
}
