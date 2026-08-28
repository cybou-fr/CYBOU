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

/// Server version observed after a conditional editor save was refused as stale.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileConflict {
    /// Current verified server text.
    pub server_content: String,
    /// Digest of that server text, usable only after an explicit resolution choice.
    pub server_sha256: String,
}

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

/// Sorting mode for directory listings.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FileSortMode {
    /// Sort alphabetically by name.
    #[default]
    Name,
    /// Sort by size in bytes.
    Size,
    /// Sort by entry type (folders vs files).
    Kind,
}

/// One File Manager card's reactive state signals.
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
    /// Owner-issued reference for the selected file, if its read succeeded.
    pub selected_location: RwSignal<Option<cybou_protocol::LocationRef>>,
    /// Content version established by the successful read.
    pub selected_sha256: RwSignal<Option<String>>,
    /// Whether a read is in flight.
    pub loading: RwSignal<bool>,
    /// What went wrong with the last read, if anything.
    pub error_msg: RwSignal<Option<String>>,
    /// Whether this directory has ever been read.
    pub read: RwSignal<bool>,
    /// Instant search/filter query in current directory.
    pub filter_query: RwSignal<String>,
    /// Active sorting mode.
    pub sort_by: RwSignal<FileSortMode>,
    /// Whether sorting direction is ascending.
    pub sort_ascending: RwSignal<bool>,
    /// Whether the create file modal is open.
    pub create_modal_open: RwSignal<bool>,
    /// Target name of the new file to create.
    pub create_name: RwSignal<String>,
    /// Error message for file creation failure.
    pub create_error: RwSignal<Option<String>>,
}

impl FileManagerSignals {
    /// A File Manager that has read nothing.
    fn new() -> Self {
        Self {
            current_path: RwSignal::new("/".to_owned()),
            entries: RwSignal::new(Vec::new()),
            selected_file: RwSignal::new(None),
            file_content: RwSignal::new(String::new()),
            selected_location: RwSignal::new(None),
            selected_sha256: RwSignal::new(None),
            loading: RwSignal::new(false),
            error_msg: RwSignal::new(None),
            read: RwSignal::new(false),
            filter_query: RwSignal::new(String::new()),
            sort_by: RwSignal::new(FileSortMode::Name),
            sort_ascending: RwSignal::new(true),
            create_modal_open: RwSignal::new(false),
            create_name: RwSignal::new(String::new()),
            create_error: RwSignal::new(None),
        }
    }
}

/// One open file buffer / tab in the Text Editor.
#[derive(Clone, Debug, PartialEq)]
pub struct EditorTab {
    /// Stable server-side recovery identity, independent of browser session.
    pub recovery_id: String,
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
    /// Whether this buffer came from recovery and remains unsaved even if text is unchanged.
    pub recovered_unsaved: bool,
    /// Line number (1-indexed).
    pub line: usize,
    /// Column number (1-indexed).
    pub col: usize,
    /// Detected language / syntax format.
    pub language: String,
    /// Read-only protection mode.
    pub read_only: bool,
    /// Last server-established content version, absent for unbound drafts.
    pub expected_sha256: Option<String>,
    /// Current server version discovered after a stale write, if unresolved.
    pub conflict: Option<FileConflict>,
    /// Monotonic debounce generation for server-side draft autosave on this tab.
    pub autosave_generation: u64,
}

impl EditorTab {
    /// Reconstruct an unsaved editor buffer from the principal's durable draft store.
    #[must_use]
    pub fn from_recovery(draft: cybou_web_contracts::UserDraftProjection) -> Self {
        let location = cybou_protocol::LocationRef::Draft {
            draft_id: draft.draft_id.clone(),
        };
        let mut tab = Self::from_location(
            location,
            draft.content.clone(),
            draft.base_sha256.unwrap_or_default(),
        );
        tab.recovery_id = draft.draft_id;
        tab.name = draft.title;
        tab.original_content = draft.content;
        tab.dirty = true;
        tab.recovered_unsaved = true;
        tab.autosave_generation = 0;
        if tab.expected_sha256.as_deref() == Some("") {
            tab.expected_sha256 = None;
        }
        tab
    }

    /// Restore a file-backed draft after the current owner re-mints its location and version.
    #[must_use]
    pub fn from_recovery_against_file(
        draft: cybou_web_contracts::UserDraftProjection,
        current: cybou_web_contracts::FileContentProjection,
    ) -> Self {
        let base_changed = draft.base_sha256.as_deref() != Some(current.content_sha256.as_str());
        let mut tab = Self::from_location(
            current.location,
            current.text.clone(),
            current.content_sha256.clone(),
        );
        tab.recovery_id = draft.draft_id;
        tab.name = draft.title;
        tab.content = draft.content;
        tab.dirty = true;
        tab.recovered_unsaved = true;
        tab.autosave_generation = 0;
        if base_changed {
            tab.conflict = Some(FileConflict {
                server_content: current.text,
                server_sha256: current.content_sha256,
            });
        }
        tab
    }

    /// Create a new empty untitled buffer.
    #[must_use]
    pub fn untitled() -> Self {
        Self::draft(1)
    }

    /// Create a distinct empty draft buffer within one editor instance.
    #[must_use]
    pub fn draft(number: u32) -> Self {
        Self {
            recovery_id: format!("editor-draft-{}", uuid::Uuid::new_v4()),
            name: format!("untitled-{number}.txt"),
            location: cybou_protocol::LocationRef::Draft {
                draft_id: format!("untitled-{number}"),
            },
            content: String::new(),
            original_content: String::new(),
            dirty: false,
            recovered_unsaved: false,
            line: 1,
            col: 1,
            language: "text".to_string(),
            read_only: false,
            expected_sha256: None,
            conflict: None,
            autosave_generation: 0,
        }
    }

    /// Create a buffer from an owner-issued location and content.
    #[must_use]
    pub fn from_location(
        location: cybou_protocol::LocationRef,
        content: String,
        expected_sha256: String,
    ) -> Self {
        let path = location.display_path();
        let name = path.rsplit('/').next().unwrap_or(&path).to_string();
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
        }
        .to_string();

        let read_only = location.is_read_only();

        Self {
            recovery_id: format!("editor-file-{}", uuid::Uuid::new_v4()),
            name,
            location,
            content: content.clone(),
            original_content: content,
            dirty: false,
            recovered_unsaved: false,
            line: 1,
            col: 1,
            language,
            read_only,
            expected_sha256: Some(expected_sha256),
            conflict: None,
            autosave_generation: 0,
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
    /// Whether discarding the local buffer for a conflict is awaiting confirmation.
    pub conflict_discard_open: RwSignal<bool>,
    /// Tab awaiting explicit confirmation before its unsaved buffer is discarded.
    pub pending_close_tab: RwSignal<Option<usize>>,
    /// Next editor-local draft identity.
    pub next_draft_number: RwSignal<u32>,
    /// Whether closing the whole editor with unsaved buffers awaits confirmation.
    pub card_close_open: RwSignal<bool>,
    /// Monotonic debounce generation for server-side draft autosave.
    pub autosave_generation: RwSignal<u64>,
    /// Whether the exclusive Save As dialog is open.
    pub save_as_open: RwSignal<bool>,
    /// Relative jail path currently entered in Save As.
    pub save_as_path: RwSignal<String>,
    /// Whether in-editor Search/Replace bar is visible.
    pub search_open: RwSignal<bool>,
    /// Whether replace mode is expanded in the search bar.
    pub replace_mode: RwSignal<bool>,
    /// Current search query string.
    pub search_query: RwSignal<String>,
    /// Current replace string.
    pub replace_query: RwSignal<String>,
    /// Whether search query matching is case-sensitive.
    pub search_case_sensitive: RwSignal<bool>,
    /// 0-indexed index of active highlighted match.
    pub search_match_index: RwSignal<usize>,
}

/// What admitting a file into an editor did.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EditorTabAdmission {
    /// The same owner-issued location was already open and was only focused.
    FocusedExisting,
    /// The initial pristine draft was replaced by the file.
    ReplacedPristineDraft,
    /// The file was added as another tab.
    Added,
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
            conflict_discard_open: RwSignal::new(false),
            pending_close_tab: RwSignal::new(None),
            next_draft_number: RwSignal::new(2),
            card_close_open: RwSignal::new(false),
            autosave_generation: RwSignal::new(0),
            save_as_open: RwSignal::new(false),
            save_as_path: RwSignal::new(String::new()),
            search_open: RwSignal::new(false),
            replace_mode: RwSignal::new(false),
            search_query: RwSignal::new(String::new()),
            replace_query: RwSignal::new(String::new()),
            search_case_sensitive: RwSignal::new(false),
            search_match_index: RwSignal::new(0),
        }
    }

    /// Focus an already-open location or admit it without replacing local work.
    pub fn admit_file(&self, tab: EditorTab) -> EditorTabAdmission {
        let mut admission = EditorTabAdmission::Added;
        self.tabs.update(|tabs| {
            if let Some(position) = tabs
                .iter()
                .position(|existing| existing.location == tab.location)
            {
                self.active_tab_index.set(position);
                admission = EditorTabAdmission::FocusedExisting;
                return;
            }

            if tabs.len() == 1
                && matches!(tabs[0].location, cybou_protocol::LocationRef::Draft { .. })
                && !tabs[0].dirty
                && tabs[0].conflict.is_none()
                && tabs[0].content.is_empty()
            {
                tabs[0] = tab;
                self.active_tab_index.set(0);
                admission = EditorTabAdmission::ReplacedPristineDraft;
                return;
            }

            tabs.push(tab);
            self.active_tab_index.set(tabs.len().saturating_sub(1));
        });
        admission
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

/// One Universal Inspector card's interactive state.
#[derive(Clone, Copy)]
pub struct InspectorSignals {
    /// Active subject being inspected.
    pub target_subject: RwSignal<Option<cybou_protocol::SubjectRef>>,
    /// Status message or last action output.
    pub status_msg: RwSignal<Option<String>>,
}

impl InspectorSignals {
    fn new() -> Self {
        Self {
            target_subject: RwSignal::new(None),
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
    inspectors: StoredValue<HashMap<CardId, InspectorSignals>>,
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
            inspectors: StoredValue::new(HashMap::new()),
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
        if let Some(existing) = self.editors.with_value(|held| held.get(&card).copied()) {
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
        if let Some(existing) = self.diffs.with_value(|held| held.get(&card).copied()) {
            return existing;
        }
        let created = self.owner.with_value(|owner| owner.with(DiffSignals::new));
        self.diffs.update_value(|held| {
            held.insert(card, created);
        });
        created
    }

    /// This Inspector card's state, creating it the first time the card is shown.
    #[must_use]
    pub fn inspector(&self, card: CardId) -> InspectorSignals {
        if let Some(existing) = self.inspectors.with_value(|held| held.get(&card).copied()) {
            return existing;
        }
        let created = self
            .owner
            .with_value(|owner| owner.with(InspectorSignals::new));
        self.inspectors.update_value(|held| {
            held.insert(card, created);
        });
        created
    }

    /// Whether any editor instance holds changes that exist only in browser memory.
    #[must_use]
    pub fn has_unsaved_editor_buffers(&self) -> bool {
        self.editors.with_value(|editors| {
            editors.values().any(|editor| {
                editor
                    .tabs
                    .get_untracked()
                    .iter()
                    .any(|tab| tab.dirty || tab.conflict.is_some())
            })
        })
    }

    /// Restore recovered drafts into the primary editor state independently of DOM mounting.
    pub fn restore_drafts(&self, restored: Vec<EditorTab>) -> usize {
        let editor = self.editor(CardId::Editor(0));
        let conflicts = restored.iter().filter(|tab| tab.conflict.is_some()).count();
        editor.tabs.update(|all| {
            let pristine = all.len() == 1 && !all[0].dirty && all[0].content.is_empty();
            let to_restore = restored
                .into_iter()
                .filter(|draft| !all.iter().any(|tab| tab.recovery_id == draft.recovery_id))
                .collect::<Vec<_>>();
            if pristine && !to_restore.is_empty() {
                *all = to_restore;
            } else {
                all.extend(to_restore);
            }
        });
        editor.status_msg.set(Some(if conflicts == 0 {
            "Recovered durable editor drafts with current authority.".to_string()
        } else {
            format!(
                "Recovered durable editor drafts; {conflicts} changed on the server and require conflict review."
            )
        }));
        conflicts
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
        self.inspectors.update_value(|held| {
            held.remove(&card);
        });
    }
}

impl Default for ToolCardStates {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute 1-indexed (line, column) for a given character offset within a text string.
#[must_use]
pub fn calculate_line_column(text: &str, char_offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    for (i, ch) in text.chars().enumerate() {
        if i >= char_offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

/// Find all non-overlapping match character start and end offsets in text.
#[must_use]
pub fn find_matches(text: &str, query: &str, case_sensitive: bool) -> Vec<(usize, usize)> {
    if query.is_empty() || text.is_empty() {
        return Vec::new();
    }
    let mut matches = Vec::new();
    let text_chars: Vec<char> = text.chars().collect();
    let query_chars: Vec<char> = query.chars().collect();

    if query_chars.is_empty() || text_chars.len() < query_chars.len() {
        return matches;
    }

    let mut start_char = 0;
    while start_char + query_chars.len() <= text_chars.len() {
        let matches_here = if case_sensitive {
            text_chars[start_char..start_char + query_chars.len()] == query_chars[..]
        } else {
            text_chars[start_char..start_char + query_chars.len()]
                .iter()
                .zip(query_chars.iter())
                .all(|(a, b)| a.to_lowercase().eq(b.to_lowercase()))
        };

        if matches_here {
            matches.push((start_char, start_char + query_chars.len()));
            start_char += query_chars.len();
        } else {
            start_char += 1;
        }
    }
    matches
}

/// Replace all occurrences of query in text with replacement string.
#[must_use]
pub fn replace_all_matches(
    text: &str,
    query: &str,
    replacement: &str,
    case_sensitive: bool,
) -> (String, usize) {
    let matches = find_matches(text, query, case_sensitive);
    if matches.is_empty() {
        return (text.to_string(), 0);
    }
    let count = matches.len();
    let text_chars: Vec<char> = text.chars().collect();
    let mut result = String::new();
    let mut last_idx = 0;
    for (start, end) in matches {
        result.extend(&text_chars[last_idx..start]);
        result.push_str(replacement);
        last_idx = end;
    }
    result.extend(&text_chars[last_idx..]);
    (result, count)
}

/// Format file size into human-readable representation.
#[must_use]
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Parse a filesystem path into hierarchical breadcrumbs (label, target_path).
#[must_use]
pub fn parse_path_breadcrumbs(path: &str) -> Vec<(&str, String)> {
    let trimmed = path.trim();
    if trimmed.is_empty() || trimmed == "/" {
        return vec![("root", "/".to_string())];
    }
    let mut crumbs = vec![("root", "/".to_string())];
    let segments: Vec<&str> = trimmed.split('/').filter(|s| !s.is_empty()).collect();
    let mut accumulated = String::new();
    for seg in segments {
        accumulated.push('/');
        accumulated.push_str(seg);
        crumbs.push((seg, accumulated.clone()));
    }
    crumbs
}

/// Sort directory entries by the selected mode with folders placed appropriately.
pub fn sort_directory_entries(
    entries: &mut [(String, bool, u64)],
    mode: FileSortMode,
    ascending: bool,
) {
    entries.sort_by(|a, b| match mode {
        FileSortMode::Name => match (a.1, b.1) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => {
                let ord = a.0.to_lowercase().cmp(&b.0.to_lowercase());
                if ascending { ord } else { ord.reverse() }
            }
        },
        FileSortMode::Size => match (a.1, b.1) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => {
                let ord = a.2.cmp(&b.2);
                if ascending { ord } else { ord.reverse() }
            }
        },
        FileSortMode::Kind => {
            let ord =
                b.1.cmp(&a.1)
                    .then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
            if ascending { ord } else { ord.reverse() }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use cybou_protocol::LocationRef;
    use cybou_web_contracts::UserDraftProjection;

    #[test]
    fn recovered_file_buffer_stays_dirty_until_a_verified_save() {
        let tab = EditorTab::from_recovery(UserDraftProjection {
            draft_id: "recovery-id".into(),
            title: "notes.txt".into(),
            content: "unsaved recovery".into(),
            base_location: Some(LocationRef::SafeShellJail {
                session_id: "issued-seat".into(),
                path: "notes.txt".into(),
            }),
            base_sha256: Some("a".repeat(64)),
            updated_at_utc: "2026-08-28T00:00:00Z".into(),
        });

        assert_eq!(tab.recovery_id, "recovery-id");
        assert!(tab.dirty);
        assert!(tab.recovered_unsaved);
        assert_eq!(
            tab.expected_sha256.as_deref(),
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        );
    }

    #[test]
    fn each_tab_tracks_its_own_autosave_generation() {
        let mut tab_a = EditorTab::draft(1);
        let mut tab_b = EditorTab::draft(2);

        assert_eq!(tab_a.autosave_generation, 0);
        assert_eq!(tab_b.autosave_generation, 0);

        tab_a.autosave_generation += 1;
        assert_eq!(tab_a.autosave_generation, 1);
        assert_eq!(tab_b.autosave_generation, 0);

        tab_b.autosave_generation += 1;
        assert_eq!(tab_a.autosave_generation, 1);
        assert_eq!(tab_b.autosave_generation, 1);
    }

    #[test]
    fn calculates_line_and_column_positions() {
        let text = "hello\nworld\ncybou";
        assert_eq!(calculate_line_column(text, 0), (1, 1));
        assert_eq!(calculate_line_column(text, 5), (1, 6));
        assert_eq!(calculate_line_column(text, 6), (2, 1));
        assert_eq!(calculate_line_column(text, 11), (2, 6));
        assert_eq!(calculate_line_column(text, 12), (3, 1));
        assert_eq!(calculate_line_column(text, 17), (3, 6));
    }

    #[test]
    fn finds_matches_with_and_without_case_sensitivity() {
        let text = "Cybou system cybou desktop CYBOU";
        let insensitive = find_matches(text, "cybou", false);
        assert_eq!(insensitive, vec![(0, 5), (13, 18), (27, 32)]);

        let sensitive = find_matches(text, "cybou", true);
        assert_eq!(sensitive, vec![(13, 18)]);
    }

    #[test]
    fn replaces_all_matching_occurrences() {
        let text = "Rust is great. I love rust.";
        let (replaced, count) = replace_all_matches(text, "rust", "Go", false);
        assert_eq!(count, 2);
        assert_eq!(replaced, "Go is great. I love Go.");
    }

    #[test]
    fn formats_file_sizes_across_scales() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024 * 2), "2.0 GB");
    }

    #[test]
    fn parses_hierarchical_path_breadcrumbs() {
        assert_eq!(parse_path_breadcrumbs("/"), vec![("root", "/".to_string())]);
        assert_eq!(
            parse_path_breadcrumbs("/docs"),
            vec![("root", "/".to_string()), ("docs", "/docs".to_string())]
        );
        assert_eq!(
            parse_path_breadcrumbs("/crates/living-canvas/src"),
            vec![
                ("root", "/".to_string()),
                ("crates", "/crates".to_string()),
                ("living-canvas", "/crates/living-canvas".to_string()),
                ("src", "/crates/living-canvas/src".to_string()),
            ]
        );
    }

    #[test]
    fn sorts_directory_entries_with_folders_first() {
        let mut entries = vec![
            ("zebra.txt".to_string(), false, 200),
            ("alpha".to_string(), true, 0),
            ("beta.rs".to_string(), false, 50),
            ("docs".to_string(), true, 0),
        ];

        sort_directory_entries(&mut entries, FileSortMode::Name, true);
        assert_eq!(
            entries.iter().map(|e| e.0.as_str()).collect::<Vec<_>>(),
            vec!["alpha", "docs", "beta.rs", "zebra.txt"]
        );

        sort_directory_entries(&mut entries, FileSortMode::Size, true);
        assert_eq!(
            entries.iter().map(|e| e.0.as_str()).collect::<Vec<_>>(),
            vec!["alpha", "docs", "beta.rs", "zebra.txt"]
        );
    }
}
