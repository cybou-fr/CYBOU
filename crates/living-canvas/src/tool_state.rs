// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Where a tool card's interactive state lives, so that showing it is not what keeps it alive.
//!
//! A Card's identity survives composition — that is Invariant L8's whole point, and the layout
//! model has always held it. What did not survive was everything the card had *done*. Shell
//! history, the command a person was halfway through recalling, the directory the File Manager was
//! looking at: all of it was created with `signal(...)` inside the component, owned by that
//! component's reactive owner, and destroyed the moment the component unmounted.
//!
//! Three ordinary actions unmounted it. Collapsing the card, because `CardFrame` wraps its body in
//! a `Show`. Switching decks tabs, because the deck body renders the active card's content and
//! nothing else. Docking a card into a deck or pulling it back out, because standalone and docked
//! are different subtrees entirely. Collapsing is the one that matters most: a person tidying their
//! desktop was silently erasing a terminal session.
//!
//! The state is therefore created under the root owner and looked up by `CardId`. A component that
//! mounts finds what its card already had; a component that unmounts takes nothing with it. Node
//! references stay component-local on purpose — those point at DOM nodes, which really do belong to
//! one mount.

use std::collections::HashMap;

use leptos::prelude::*;
use leptos::reactive::owner::{Owner, StoredValue};

use crate::CardId;

/// The greeting a Shell card shows before anyone has typed anything.
const SHELL_BANNER: &str = "Bounded, read-only. Type 'help' to see what this shell can do.\n";

/// One Shell card's interactive state.
///
/// Every field is an `RwSignal` rather than a read/write pair, so a caller can bind the same handle
/// under both names and read and write it exactly as it would a local signal.
#[derive(Clone, Copy)]
pub struct ShellSignals {
    /// Command, output and exit code of everything run in this shell.
    pub history: RwSignal<Vec<(String, String, i32)>>,
    /// The commands themselves, for recall with the arrow keys.
    pub cmd_history: RwSignal<Vec<String>>,
    /// How far back through `cmd_history` the person currently is.
    pub history_idx: RwSignal<Option<usize>>,
    /// What they had typed before they started recalling, so it can be given back.
    pub temp_draft: RwSignal<String>,
    /// The line being typed.
    pub input: RwSignal<String>,
    /// Where this shell is standing, as the gateway last reported it.
    pub cwd: RwSignal<String>,
    /// Whether a command is in flight.
    pub running: RwSignal<bool>,
}

impl ShellSignals {
    /// A shell that has run nothing.
    fn new() -> Self {
        Self {
            history: RwSignal::new(vec![(String::new(), SHELL_BANNER.to_owned(), 0)]),
            cmd_history: RwSignal::new(Vec::new()),
            history_idx: RwSignal::new(None),
            temp_draft: RwSignal::new(String::new()),
            input: RwSignal::new(String::new()),
            cwd: RwSignal::new("/".to_owned()),
            running: RwSignal::new(false),
        }
    }
}

/// One File Manager card's interactive state.
#[derive(Clone, Copy)]
pub struct FileManagerSignals {
    /// The directory being looked at.
    pub current_path: RwSignal<String>,
    /// What that directory held when it was last read.
    pub entries: RwSignal<Vec<(String, bool, u64)>>,
    /// The file whose contents are open, if any.
    pub selected_file: RwSignal<Option<String>>,
    /// Those contents.
    pub file_content: RwSignal<String>,
    /// Whether a read is in flight.
    pub loading: RwSignal<bool>,
    /// What went wrong with the last read, if anything.
    pub error_msg: RwSignal<Option<String>>,
    /// Whether this directory has ever been read.
    pub read: RwSignal<bool>,
}

impl FileManagerSignals {
    /// A File Manager that has read nothing.
    fn new() -> Self {
        Self {
            current_path: RwSignal::new("/".to_owned()),
            entries: RwSignal::new(Vec::new()),
            selected_file: RwSignal::new(None),
            file_content: RwSignal::new(String::new()),
            loading: RwSignal::new(false),
            error_msg: RwSignal::new(None),
            read: RwSignal::new(false),
        }
    }
}

/// One open file buffer / tab in the Text Editor.
#[derive(Clone, Debug, PartialEq)]
pub struct EditorTab {
    /// File name / display label.
    pub name: String,
    /// Typed location reference and authority domain.
    pub location: cybou_protocol::LocationRef,
    /// Editable text buffer.
    pub content: String,
    /// Original unmodified disk content for diffing.
    pub original_content: String,
    /// Whether the buffer contains unsaved changes.
    pub dirty: bool,
    /// Line number (1-indexed).
    pub line: usize,
    /// Column number (1-indexed).
    pub col: usize,
    /// Detected language / syntax format.
    pub language: String,
    /// Read-only protection mode.
    pub read_only: bool,
}

impl EditorTab {
    /// Create a new empty untitled buffer.
    #[must_use]
    pub fn untitled() -> Self {
        Self {
            name: "untitled.txt".to_string(),
            location: cybou_protocol::LocationRef::HostUserPath("/home/cybou/untitled.txt".to_string()),
            content: String::new(),
            original_content: String::new(),
            dirty: false,
            line: 1,
            col: 1,
            language: "text".to_string(),
            read_only: false,
        }
    }

    /// Create a buffer from an existing file path and content.
    #[must_use]
    pub fn from_file(path: &str, content: String) -> Self {
        let name = path.rsplit('/').next().unwrap_or(path).to_string();
        let ext = name.rsplit('.').next().unwrap_or("");
        let language = match ext {
            "rs" => "rust",
            "md" | "markdown" => "markdown",
            "json" => "json",
            "toml" => "toml",
            "yaml" | "yml" => "yaml",
            "sh" | "bash" => "shell",
            "conf" if path.contains("nginx") => "nginx",
            "service" | "target" | "timer" => "systemd",
            "py" => "python",
            "js" | "ts" => "javascript",
            "html" | "htm" => "html",
            "css" => "css",
            _ => "text",
        }.to_string();

        let location = cybou_protocol::LocationRef::from_path(path);
        let read_only = location.is_read_only();

        Self {
            name,
            location,
            content: content.clone(),
            original_content: content,
            dirty: false,
            line: 1,
            col: 1,
            language,
            read_only,
        }
    }
}

/// One Text Editor card's interactive state.
#[derive(Clone, Copy)]
pub struct EditorSignals {
    /// Open file buffer tabs.
    pub tabs: RwSignal<Vec<EditorTab>>,
    /// Index of currently active tab.
    pub active_tab_index: RwSignal<usize>,
    /// Whether a file load/save is in progress.
    pub loading: RwSignal<bool>,
    /// Status or error message.
    pub status_msg: RwSignal<Option<String>>,
    /// Whether Markdown split preview is enabled.
    pub markdown_preview: RwSignal<bool>,
    /// Whether the Action1 Diff/Save confirmation modal is open.
    pub save_proposal_open: RwSignal<bool>,
}

impl EditorSignals {
    fn new() -> Self {
        Self {
            tabs: RwSignal::new(vec![EditorTab::untitled()]),
            active_tab_index: RwSignal::new(0),
            loading: RwSignal::new(false),
            status_msg: RwSignal::new(None),
            markdown_preview: RwSignal::new(false),
            save_proposal_open: RwSignal::new(false),
        }
    }
}

/// One Diff Viewer card's interactive state.
#[derive(Clone, Copy)]
pub struct DiffSignals {
    /// Target file or entity title.
    pub title: RwSignal<String>,
    /// Source 1 label (e.g. "On-disk / live").
    pub original_label: RwSignal<String>,
    /// Source 2 label (e.g. "Editor buffer / Proposed patch").
    pub proposed_label: RwSignal<String>,
    /// Original text content.
    pub original_content: RwSignal<String>,
    /// Proposed text content.
    pub proposed_content: RwSignal<String>,
    /// Whether an action is in flight.
    pub loading: RwSignal<bool>,
    /// Status message.
    pub status_msg: RwSignal<Option<String>>,
}

impl DiffSignals {
    fn new() -> Self {
        Self {
            title: RwSignal::new("Diff Viewer".to_string()),
            original_label: RwSignal::new("Current (Disk)".to_string()),
            proposed_label: RwSignal::new("Proposed".to_string()),
            original_content: RwSignal::new(String::new()),
            proposed_content: RwSignal::new(String::new()),
            loading: RwSignal::new(false),
            status_msg: RwSignal::new(None),
        }
    }
}

/// The interactive state of every tool card on this desktop.
///
/// Provided once at the root and read from context by the cards. It is `Copy` so a card can hold it
/// in a closure without ceremony.
#[derive(Clone, Copy)]
pub struct ToolCardStates {
    /// The owner every piece of state is created under.
    owner: StoredValue<Owner>,
    shells: StoredValue<HashMap<CardId, ShellSignals>>,
    file_managers: StoredValue<HashMap<CardId, FileManagerSignals>>,
    editors: StoredValue<HashMap<CardId, EditorSignals>>,
    diffs: StoredValue<HashMap<CardId, DiffSignals>>,
}

impl ToolCardStates {
    /// Build the store under the current reactive owner.
    #[must_use]
    pub fn new() -> Self {
        let owner = Owner::current().expect("a reactive owner to anchor tool card state to");
        Self {
            owner: StoredValue::new(owner),
            shells: StoredValue::new(HashMap::new()),
            file_managers: StoredValue::new(HashMap::new()),
            editors: StoredValue::new(HashMap::new()),
            diffs: StoredValue::new(HashMap::new()),
        }
    }

    /// This Shell card's state, creating it the first time the card is shown.
    #[must_use]
    pub fn shell(&self, card: CardId) -> ShellSignals {
        if let Some(existing) = self.shells.with_value(|held| held.get(&card).copied()) {
            return existing;
        }
        let created = self.owner.with_value(|owner| owner.with(ShellSignals::new));
        self.shells.update_value(|held| {
            held.insert(card, created);
        });
        created
    }

    /// This File Manager card's state, creating it the first time the card is shown.
    #[must_use]
    pub fn file_manager(&self, card: CardId) -> FileManagerSignals {
        if let Some(existing) = self
            .file_managers
            .with_value(|held| held.get(&card).copied())
        {
            return existing;
        }
        let created = self
            .owner
            .with_value(|owner| owner.with(FileManagerSignals::new));
        self.file_managers.update_value(|held| {
            held.insert(card, created);
        });
        created
    }

    /// This Text Editor card's state, creating it the first time the card is shown.
    #[must_use]
    pub fn editor(&self, card: CardId) -> EditorSignals {
        if let Some(existing) = self
            .editors
            .with_value(|held| held.get(&card).copied())
        {
            return existing;
        }
        let created = self
            .owner
            .with_value(|owner| owner.with(EditorSignals::new));
        self.editors.update_value(|held| {
            held.insert(card, created);
        });
        created
    }

    /// This Diff Viewer card's state, creating it the first time the card is shown.
    #[must_use]
    pub fn diff(&self, card: CardId) -> DiffSignals {
        if let Some(existing) = self
            .diffs
            .with_value(|held| held.get(&card).copied())
        {
            return existing;
        }
        let created = self
            .owner
            .with_value(|owner| owner.with(DiffSignals::new));
        self.diffs.update_value(|held| {
            held.insert(card, created);
        });
        created
    }

    /// Forget everything a card had done, because the card itself is gone.
    pub fn forget(&self, card: CardId) {
        self.shells.update_value(|held| {
            held.remove(&card);
        });
        self.file_managers.update_value(|held| {
            held.remove(&card);
        });
        self.editors.update_value(|held| {
            held.remove(&card);
        });
        self.diffs.update_value(|held| {
            held.remove(&card);
        });
    }
}

impl Default for ToolCardStates {
    fn default() -> Self {
        Self::new()
    }
}
