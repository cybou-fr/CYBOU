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

use leptos::prelude::LocalStorage;
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

/// One Terminal card's live session.
///
/// The screen and the socket are local signals rather than ordinary ones. Neither a `vt100` parser
/// nor a `WebSocket` is `Send`, and neither should be: this is one screen belonging to one tab, and
/// a type that could be moved between threads would be claiming otherwise.
#[derive(Clone, Copy)]
pub struct TerminalSignals {
    /// The screen, fed by the host and read by the view.
    pub screen: RwSignal<crate::terminal::TerminalScreen, LocalStorage>,
    /// Bumped whenever bytes arrive.
    ///
    /// The screen is mutated in place and is not a value the view can compare, so something has to
    /// tell it that a repaint is due. Without this the grid would redraw only when some other
    /// signal happened to change, which is a terminal that answers late.
    pub generation: RwSignal<u64>,
    /// The live socket, while there is one.
    pub socket: RwSignal<Option<web_sys::WebSocket>, LocalStorage>,
    /// What this card is doing, in one word for the bar.
    pub status: RwSignal<String>,
    /// Why there is no terminal, in prose a person can act on.
    pub refusal: RwSignal<Option<String>>,
    /// The window size the host was last told about.
    ///
    /// Held so a measurement that changed nothing sends nothing: a resize frame reaches
    /// `TIOCSWINSZ` and every program in the session gets `SIGWINCH`, which is not something to
    /// deliver on a timer for a panel nobody moved.
    pub window: RwSignal<(u16, u16)>,
}

impl TerminalSignals {
    /// A terminal that has not connected.
    fn new() -> Self {
        Self {
            screen: RwSignal::new_local(crate::terminal::TerminalScreen::new(80, 24)),
            generation: RwSignal::new(0),
            socket: RwSignal::new_local(None),
            status: RwSignal::new("Not connected".to_owned()),
            refusal: RwSignal::new(None),
            window: RwSignal::new((
                crate::terminal::DEFAULT_COLUMNS,
                crate::terminal::DEFAULT_ROWS,
            )),
        }
    }
}

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
    /// Active location category in sidebar.
    pub active_category: RwSignal<cybou_web_contracts::LocationCategory>,
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
    /// Monotonic identity of the latest directory request.
    pub directory_request_generation: RwSignal<u64>,
    /// Monotonic identity of the latest file preview request.
    pub file_request_generation: RwSignal<u64>,
    /// What went wrong with the last read, if anything.
    pub error_msg: RwSignal<Option<String>>,
    /// Success / action toast message.
    pub action_message: RwSignal<Option<String>>,
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
    /// Whether create directory modal is open.
    pub create_dir_modal_open: RwSignal<bool>,
    /// Target name of the new directory to create.
    pub create_dir_name: RwSignal<String>,
    /// Whether rename modal is open.
    pub rename_modal_open: RwSignal<bool>,
    /// Target item to rename.
    pub rename_target: RwSignal<Option<String>>,
    /// New name for renamed item.
    pub rename_new_name: RwSignal<String>,
    /// Whether delete confirmation modal is open.
    pub delete_modal_open: RwSignal<bool>,
    /// Target item to delete (name, `is_dir`).
    pub delete_target: RwSignal<Option<(String, bool)>>,
}

impl FileManagerSignals {
    /// A File Manager that has read nothing.
    fn new() -> Self {
        Self {
            active_category: RwSignal::new(cybou_web_contracts::LocationCategory::Home),
            current_path: RwSignal::new("/".to_owned()),
            entries: RwSignal::new(Vec::new()),
            selected_file: RwSignal::new(None),
            file_content: RwSignal::new(String::new()),
            selected_location: RwSignal::new(None),
            selected_sha256: RwSignal::new(None),
            loading: RwSignal::new(false),
            directory_request_generation: RwSignal::new(0),
            file_request_generation: RwSignal::new(0),
            error_msg: RwSignal::new(None),
            action_message: RwSignal::new(None),
            read: RwSignal::new(false),
            filter_query: RwSignal::new(String::new()),
            sort_by: RwSignal::new(FileSortMode::Name),
            sort_ascending: RwSignal::new(true),
            create_modal_open: RwSignal::new(false),
            create_name: RwSignal::new(String::new()),
            create_error: RwSignal::new(None),
            create_dir_modal_open: RwSignal::new(false),
            create_dir_name: RwSignal::new(String::new()),
            rename_modal_open: RwSignal::new(false),
            rename_target: RwSignal::new(None),
            rename_new_name: RwSignal::new(String::new()),
            delete_modal_open: RwSignal::new(false),
            delete_target: RwSignal::new(None),
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
    /// User selection awaiting an owner-backed resolution.
    pub subject_query: RwSignal<Option<cybou_protocol::SubjectQuery>>,
    /// Status message or last action output.
    pub status_msg: RwSignal<Option<String>>,
}

impl InspectorSignals {
    fn new() -> Self {
        Self {
            target_subject: RwSignal::new(None),
            subject_query: RwSignal::new(None),
            status_msg: RwSignal::new(None),
        }
    }
}

/// One Operations Manager card's interactive state.
#[derive(Clone, Copy)]
pub struct OperationsSignals {
    /// Active and historical operations list.
    pub operations: RwSignal<Vec<cybou_protocol::operation::OperationRecord>>,
    /// Currently inspected operation ID.
    pub selected_op_id: RwSignal<Option<uuid::Uuid>>,
    /// Execution log lines for the selected operation.
    pub selected_logs: RwSignal<Vec<cybou_protocol::operation::OperationLogEntry>>,
    /// Category/status filter.
    pub filter_status: RwSignal<Option<String>>,
    /// Whether a background fetch is running.
    pub loading: RwSignal<bool>,
    /// Status message or error toast.
    pub status_msg: RwSignal<Option<String>>,
}

impl OperationsSignals {
    fn new() -> Self {
        Self {
            operations: RwSignal::new(Vec::new()),
            selected_op_id: RwSignal::new(None),
            selected_logs: RwSignal::new(Vec::new()),
            filter_status: RwSignal::new(None),
            loading: RwSignal::new(false),
            status_msg: RwSignal::new(None),
        }
    }
}

/// One Notifications Center card's interactive state.
#[derive(Clone, Copy)]
pub struct NotificationsSignals {
    /// Notifications list.
    pub notifications: RwSignal<Vec<cybou_protocol::notification::NotificationItem>>,
    /// Selected category filter.
    pub selected_category: RwSignal<Option<cybou_protocol::notification::NotificationCategory>>,
    /// Search filter text.
    pub search_query: RwSignal<String>,
    /// Unread count.
    pub unread_count: RwSignal<usize>,
    /// Attention/critical count.
    pub attention_count: RwSignal<usize>,
    /// Whether a background fetch is running.
    pub loading: RwSignal<bool>,
    /// Status message or toast.
    pub status_msg: RwSignal<Option<String>>,
}

impl NotificationsSignals {
    fn new() -> Self {
        Self {
            notifications: RwSignal::new(Vec::new()),
            selected_category: RwSignal::new(None),
            search_query: RwSignal::new(String::new()),
            unread_count: RwSignal::new(0),
            attention_count: RwSignal::new(0),
            loading: RwSignal::new(false),
            status_msg: RwSignal::new(None),
        }
    }
}

/// One Services Manager card's interactive state.
#[derive(Clone, Copy)]
pub struct ServicesSignals {
    /// Listed system services.
    pub services: RwSignal<Vec<cybou_protocol::system::ServiceRecord>>,
    /// Currently selected service name.
    pub selected_service: RwSignal<Option<String>>,
    /// State filter (`Active`, `Failed`, etc.).
    pub filter_state: RwSignal<Option<cybou_protocol::system::ServiceState>>,
    /// Search filter string.
    pub search_query: RwSignal<String>,
    /// Background fetch in flight.
    pub loading: RwSignal<bool>,
    /// Status message or toast.
    pub status_msg: RwSignal<Option<String>>,
    /// Whether the panel keeps asking on its own.
    pub auto_refresh: RwSignal<bool>,
}

impl ServicesSignals {
    fn new() -> Self {
        Self {
            services: RwSignal::new(Vec::new()),
            selected_service: RwSignal::new(None),
            filter_state: RwSignal::new(None),
            search_query: RwSignal::new(String::new()),
            loading: RwSignal::new(false),
            status_msg: RwSignal::new(None),
            auto_refresh: RwSignal::new(true),
        }
    }
}

/// One Process Manager card's interactive state.
#[derive(Clone, Copy)]
pub struct ProcessesSignals {
    /// Listed operating system processes.
    pub processes: RwSignal<Vec<cybou_protocol::system::ProcessRecord>>,
    /// Selected Process ID.
    pub selected_pid: RwSignal<Option<u32>>,
    /// Search filter string.
    pub search_query: RwSignal<String>,
    /// Sort column (e.g. `cpu`, `memory`, `pid`, `name`).
    pub sort_by: RwSignal<String>,
    /// Background fetch in flight.
    pub loading: RwSignal<bool>,
    /// Status message or toast.
    pub status_msg: RwSignal<Option<String>>,
    /// Whether the panel keeps asking on its own.
    pub auto_refresh: RwSignal<bool>,
}

impl ProcessesSignals {
    fn new() -> Self {
        Self {
            processes: RwSignal::new(Vec::new()),
            selected_pid: RwSignal::new(None),
            search_query: RwSignal::new(String::new()),
            sort_by: RwSignal::new("cpu".to_owned()),
            loading: RwSignal::new(false),
            status_msg: RwSignal::new(None),
            auto_refresh: RwSignal::new(true),
        }
    }
}

/// One Hardware Telemetry & System Monitor card's interactive state.
#[derive(Clone, Copy)]
pub struct MonitorSignals {
    /// Latest monitor projection snapshot.
    pub monitor: RwSignal<Option<cybou_web_contracts::SystemMonitorProjection>>,
    /// Background fetch in flight.
    pub loading: RwSignal<bool>,
    /// Periodic auto-refresh toggle.
    pub auto_refresh: RwSignal<bool>,
    /// Status message or toast.
    pub status_msg: RwSignal<Option<String>>,
}

impl MonitorSignals {
    fn new() -> Self {
        Self {
            monitor: RwSignal::new(None),
            loading: RwSignal::new(false),
            auto_refresh: RwSignal::new(true),
            status_msg: RwSignal::new(None),
        }
    }
}

/// One System Log Viewer card's interactive state.
#[derive(Clone, Copy)]
pub struct SystemLogsSignals {
    /// Log records feed.
    pub logs: RwSignal<Vec<cybou_protocol::system::SystemLogEntry>>,
    /// Unit filter.
    pub selected_unit: RwSignal<Option<String>>,
    /// Severity level filter.
    pub selected_severity: RwSignal<Option<String>>,
    /// Search filter text.
    pub search_query: RwSignal<String>,
    /// Background fetch in flight.
    pub loading: RwSignal<bool>,
    /// Status message or toast.
    pub status_msg: RwSignal<Option<String>>,
    /// Why the feed is empty, when the reason is not that nothing matched.
    pub unavailable: RwSignal<Option<cybou_protocol::system::LogsUnavailable>>,
    /// Whether the server could see the whole system journal or only its own account's.
    ///
    /// Starts true so a card that has not asked yet draws no warning about a journal nobody has
    /// read.
    pub system_journal_readable: RwSignal<bool>,
    /// Whether the panel keeps asking on its own.
    pub auto_refresh: RwSignal<bool>,
}

impl SystemLogsSignals {
    fn new() -> Self {
        Self {
            logs: RwSignal::new(Vec::new()),
            selected_unit: RwSignal::new(None),
            selected_severity: RwSignal::new(None),
            search_query: RwSignal::new(String::new()),
            loading: RwSignal::new(false),
            status_msg: RwSignal::new(None),
            unavailable: RwSignal::new(None),
            system_journal_readable: RwSignal::new(true),
            auto_refresh: RwSignal::new(true),
        }
    }
}

/// One Storage & Snapshots card's interactive state.
#[derive(Clone, Copy)]
pub struct StorageSignals {
    /// Storage pool & snapshots projection.
    pub storage: RwSignal<Option<cybou_web_contracts::StorageProjection>>,
    /// Currently selected subvolume path.
    pub selected_subvolume: RwSignal<Option<String>>,
    /// Input field for new snapshot name.
    pub new_snap_name: RwSignal<String>,
    /// Background fetch in flight.
    pub loading: RwSignal<bool>,
    /// Status message or toast.
    pub status_msg: RwSignal<Option<String>>,
}

impl StorageSignals {
    fn new() -> Self {
        Self {
            storage: RwSignal::new(None),
            selected_subvolume: RwSignal::new(Some("@home".to_owned())),
            new_snap_name: RwSignal::new(String::new()),
            loading: RwSignal::new(false),
            status_msg: RwSignal::new(None),
        }
    }
}

/// One Network Connections card's interactive state.
#[derive(Clone, Copy)]
pub struct NetworkSignals {
    /// Network connections list.
    pub connections: RwSignal<Vec<cybou_protocol::system::NetworkConnectionRecord>>,
    /// Selected connection ID.
    pub selected_conn: RwSignal<Option<String>>,
    /// Background fetch in flight.
    pub loading: RwSignal<bool>,
    /// Status message or toast.
    pub status_msg: RwSignal<Option<String>>,
}

impl NetworkSignals {
    fn new() -> Self {
        Self {
            connections: RwSignal::new(Vec::new()),
            selected_conn: RwSignal::new(None),
            loading: RwSignal::new(false),
            status_msg: RwSignal::new(None),
        }
    }
}

/// One Package Manager card's interactive state.
#[derive(Clone, Copy)]
pub struct PackagesSignals {
    /// Available & installed packages list.
    pub packages: RwSignal<Vec<cybou_protocol::system::PackageRecord>>,
    /// Active filter tab (`all`, `installed`, `upgradable`).
    pub active_tab: RwSignal<String>,
    /// Search query string.
    pub search_query: RwSignal<String>,
    /// Selected package name.
    pub selected_package: RwSignal<Option<String>>,
    /// Background fetch in flight.
    pub loading: RwSignal<bool>,
    /// Status message or toast.
    pub status_msg: RwSignal<Option<String>>,
}

impl PackagesSignals {
    fn new() -> Self {
        Self {
            packages: RwSignal::new(Vec::new()),
            active_tab: RwSignal::new("installed".to_owned()),
            search_query: RwSignal::new(String::new()),
            selected_package: RwSignal::new(None),
            loading: RwSignal::new(false),
            status_msg: RwSignal::new(None),
        }
    }
}

/// One System Updates card's interactive state.
#[derive(Clone, Copy)]
pub struct UpdatesSignals {
    /// System updates projection.
    pub updates: RwSignal<Option<cybou_web_contracts::SystemUpdatesProjection>>,
    /// Background fetch in flight.
    pub loading: RwSignal<bool>,
    /// Status message or toast.
    pub status_msg: RwSignal<Option<String>>,
}

impl UpdatesSignals {
    fn new() -> Self {
        Self {
            updates: RwSignal::new(None),
            loading: RwSignal::new(false),
            status_msg: RwSignal::new(None),
        }
    }
}

/// One Users & SSH Keys card's interactive state.
#[derive(Clone, Copy)]
pub struct UserSettingsSignals {
    /// User accounts.
    pub users: RwSignal<Vec<cybou_protocol::system::UserAccountRecord>>,
    /// Authorized SSH keys.
    pub ssh_keys: RwSignal<Vec<cybou_protocol::system::SshKeyRecord>>,
    /// New user username input.
    pub new_user_name: RwSignal<String>,
    /// New user full name input.
    pub new_full_name: RwSignal<String>,
    /// New user admin checkbox.
    pub new_is_admin: RwSignal<bool>,
    /// New SSH key label input.
    pub new_key_name: RwSignal<String>,
    /// New SSH public key content.
    pub new_public_key: RwSignal<String>,
    /// Background fetch in flight.
    pub loading: RwSignal<bool>,
    /// Status message or toast.
    pub status_msg: RwSignal<Option<String>>,
}

impl UserSettingsSignals {
    fn new() -> Self {
        Self {
            users: RwSignal::new(Vec::new()),
            ssh_keys: RwSignal::new(Vec::new()),
            new_user_name: RwSignal::new(String::new()),
            new_full_name: RwSignal::new(String::new()),
            new_is_admin: RwSignal::new(false),
            new_key_name: RwSignal::new(String::new()),
            new_public_key: RwSignal::new(String::new()),
            loading: RwSignal::new(false),
            status_msg: RwSignal::new(None),
        }
    }
}

/// One Security Policy & Audit card's interactive state.
#[derive(Clone, Copy)]
pub struct SecuritySignals {
    /// Security policy rules.
    pub policy: RwSignal<Option<cybou_protocol::system::SecurityPolicyRecord>>,
    /// Recent security audit logs.
    pub audit_log: RwSignal<Vec<cybou_protocol::system::SecurityAuditEntry>>,
    /// Background fetch in flight.
    pub loading: RwSignal<bool>,
    /// Status message or toast.
    pub status_msg: RwSignal<Option<String>>,
}

impl SecuritySignals {
    fn new() -> Self {
        Self {
            policy: RwSignal::new(None),
            audit_log: RwSignal::new(Vec::new()),
            loading: RwSignal::new(false),
            status_msg: RwSignal::new(None),
        }
    }
}

/// One Backup & Vault card's interactive state.
#[derive(Clone, Copy)]
pub struct BackupSignals {
    /// Backup repository, archives, and schedule projection.
    pub backup_settings: RwSignal<Option<cybou_web_contracts::BackupSettingsProjection>>,
    /// Input for manual snapshot label.
    pub new_backup_name: RwSignal<String>,
    /// Background fetch in flight.
    pub loading: RwSignal<bool>,
    /// Status message or toast.
    pub status_msg: RwSignal<Option<String>>,
}

impl BackupSignals {
    fn new() -> Self {
        Self {
            backup_settings: RwSignal::new(None),
            new_backup_name: RwSignal::new(String::new()),
            loading: RwSignal::new(false),
            status_msg: RwSignal::new(None),
        }
    }
}

/// One Mail & Messages card's interactive state.
#[derive(Clone, Copy)]
pub struct MailSignals {
    /// Mail projection.
    pub mail: RwSignal<Option<cybou_web_contracts::MailProjection>>,
    /// Selected message to read.
    pub selected_message: RwSignal<Option<cybou_protocol::personal::MailMessageRecord>>,
    /// Compose recipient input.
    pub compose_to: RwSignal<String>,
    /// Compose subject input.
    pub compose_subject: RwSignal<String>,
    /// Compose body input.
    pub compose_body: RwSignal<String>,
    /// Whether the compose modal / pane is active.
    pub is_composing: RwSignal<bool>,
    /// Background fetch in flight.
    pub loading: RwSignal<bool>,
    /// Status message or toast.
    pub status_msg: RwSignal<Option<String>>,
}

impl MailSignals {
    fn new() -> Self {
        Self {
            mail: RwSignal::new(None),
            selected_message: RwSignal::new(None),
            compose_to: RwSignal::new(String::new()),
            compose_subject: RwSignal::new(String::new()),
            compose_body: RwSignal::new(String::new()),
            is_composing: RwSignal::new(false),
            loading: RwSignal::new(false),
            status_msg: RwSignal::new(None),
        }
    }
}

/// One Calendar & Schedule card's interactive state.
#[derive(Clone, Copy)]
pub struct CalendarSignals {
    /// Calendar projection.
    pub calendar: RwSignal<Option<cybou_web_contracts::CalendarProjection>>,
    /// New event title input.
    pub new_title: RwSignal<String>,
    /// New event description input.
    pub new_desc: RwSignal<String>,
    /// New event start time input.
    pub new_start: RwSignal<String>,
    /// New event end time input.
    pub new_end: RwSignal<String>,
    /// New event color category input.
    pub new_color: RwSignal<String>,
    /// Whether the create modal / pane is open.
    pub is_creating: RwSignal<bool>,
    /// Background fetch in flight.
    pub loading: RwSignal<bool>,
    /// Status message or toast.
    pub status_msg: RwSignal<Option<String>>,
}

impl CalendarSignals {
    fn new() -> Self {
        Self {
            calendar: RwSignal::new(None),
            new_title: RwSignal::new(String::new()),
            new_desc: RwSignal::new(String::new()),
            new_start: RwSignal::new("2026-08-29T10:00:00Z".to_owned()),
            new_end: RwSignal::new("2026-08-29T11:00:00Z".to_owned()),
            new_color: RwSignal::new("indigo".to_owned()),
            is_creating: RwSignal::new(false),
            loading: RwSignal::new(false),
            status_msg: RwSignal::new(None),
        }
    }
}

/// One Notes & Ideas card's interactive state.
#[derive(Clone, Copy)]
pub struct NotesSignals {
    /// Notes list.
    pub notes: RwSignal<Vec<cybou_protocol::personal::NoteRecord>>,
    /// Currently selected / edited note ID.
    pub selected_note_id: RwSignal<Option<String>>,
    /// Active edit title.
    pub edit_title: RwSignal<String>,
    /// Active edit markdown content.
    pub edit_content: RwSignal<String>,
    /// Active edit comma-separated tags.
    pub edit_tags: RwSignal<String>,
    /// Active edit pin status.
    pub edit_pinned: RwSignal<bool>,
    /// Background fetch in flight.
    pub loading: RwSignal<bool>,
    /// Status message or toast.
    pub status_msg: RwSignal<Option<String>>,
}

impl NotesSignals {
    fn new() -> Self {
        Self {
            notes: RwSignal::new(Vec::new()),
            selected_note_id: RwSignal::new(None),
            edit_title: RwSignal::new(String::new()),
            edit_content: RwSignal::new(String::new()),
            edit_tags: RwSignal::new(String::new()),
            edit_pinned: RwSignal::new(false),
            loading: RwSignal::new(false),
            status_msg: RwSignal::new(None),
        }
    }
}

/// One Contacts Directory card's interactive state.
#[derive(Clone, Copy)]
pub struct ContactsSignals {
    /// Contacts list.
    pub contacts: RwSignal<Vec<cybou_protocol::personal::ContactRecord>>,
    /// Currently selected contact to inspect.
    pub selected_contact: RwSignal<Option<cybou_protocol::personal::ContactRecord>>,
    /// New contact name input.
    pub new_name: RwSignal<String>,
    /// New contact email input.
    pub new_email: RwSignal<String>,
    /// New contact role input.
    pub new_role: RwSignal<String>,
    /// New contact organization input.
    pub new_org: RwSignal<String>,
    /// New contact tags input.
    pub new_tags: RwSignal<String>,
    /// New contact notes input.
    pub new_notes: RwSignal<String>,
    /// Whether create modal / pane is open.
    pub is_creating: RwSignal<bool>,
    /// Background fetch in flight.
    pub loading: RwSignal<bool>,
    /// Status message or toast.
    pub status_msg: RwSignal<Option<String>>,
}

impl ContactsSignals {
    fn new() -> Self {
        Self {
            contacts: RwSignal::new(Vec::new()),
            selected_contact: RwSignal::new(None),
            new_name: RwSignal::new(String::new()),
            new_email: RwSignal::new(String::new()),
            new_role: RwSignal::new(String::new()),
            new_org: RwSignal::new(String::new()),
            new_tags: RwSignal::new(String::new()),
            new_notes: RwSignal::new(String::new()),
            is_creating: RwSignal::new(false),
            loading: RwSignal::new(false),
            status_msg: RwSignal::new(None),
        }
    }
}

/// One Cognitive Graph & Causal DAG card's interactive state.
#[derive(Clone, Copy)]
pub struct CognitiveGraphSignals {
    /// Graph projection.
    pub graph: RwSignal<Option<cybou_web_contracts::CognitiveGraphProjection>>,
    /// Text filter for nodes.
    pub search_query: RwSignal<String>,
    /// Currently focused or inspected node ID.
    pub selected_node_id: RwSignal<Option<String>>,
    /// Selected node category filter (e.g. Agent, Service, Finding).
    pub type_filter: RwSignal<Option<String>>,
    /// Background fetch in flight.
    pub loading: RwSignal<bool>,
    /// Status message or toast.
    pub status_msg: RwSignal<Option<String>>,
}

impl CognitiveGraphSignals {
    fn new() -> Self {
        Self {
            graph: RwSignal::new(None),
            search_query: RwSignal::new(String::new()),
            selected_node_id: RwSignal::new(None),
            type_filter: RwSignal::new(None),
            loading: RwSignal::new(false),
            status_msg: RwSignal::new(None),
        }
    }
}

/// One Canonical Event1 Journal card's interactive state.
#[derive(Clone, Copy)]
pub struct EventJournalSignals {
    /// Journal entries projection.
    pub journal: RwSignal<Option<cybou_web_contracts::EventJournalProjection>>,
    /// Filter by originating organ.
    pub organ_filter: RwSignal<Option<String>>,
    /// Text search query.
    pub search_query: RwSignal<String>,
    /// Selected event ID to inspect.
    pub selected_entry_id: RwSignal<Option<String>>,
    /// Background fetch in flight.
    pub loading: RwSignal<bool>,
    /// Status message or toast.
    pub status_msg: RwSignal<Option<String>>,
}

impl EventJournalSignals {
    fn new() -> Self {
        Self {
            journal: RwSignal::new(None),
            organ_filter: RwSignal::new(None),
            search_query: RwSignal::new(String::new()),
            selected_entry_id: RwSignal::new(None),
            loading: RwSignal::new(false),
            status_msg: RwSignal::new(None),
        }
    }
}

/// One Meaning & Dialogue Assistant card's interactive state.
#[derive(Clone, Copy)]
pub struct MeaningSignals {
    /// Natural language input query.
    pub query: RwSignal<String>,
    /// Selected realization language ("en", "ru", "de", "fr").
    pub language: RwSignal<String>,
    /// Semantic interpretation and response projection.
    pub projection: RwSignal<Option<cybou_web_contracts::MeaningInterpretProjection>>,
    /// Active dialogue memory projection.
    pub memory: RwSignal<Option<cybou_web_contracts::DialogueMemoryProjection>>,
    /// Background fetch in flight.
    pub loading: RwSignal<bool>,
    /// Status message or toast.
    pub status_msg: RwSignal<Option<String>>,
}

impl MeaningSignals {
    fn new() -> Self {
        Self {
            query: RwSignal::new(String::new()),
            language: RwSignal::new("en".to_string()),
            projection: RwSignal::new(None),
            memory: RwSignal::new(None),
            loading: RwSignal::new(false),
            status_msg: RwSignal::new(None),
        }
    }
}

/// One Lifelong Learning & Governance card's interactive state.
#[derive(Clone, Copy)]
pub struct LearningSignals {
    /// Active learning candidates.
    pub candidates: RwSignal<Vec<cybou_protocol::learning::LearningCandidate>>,
    /// Selected learning layer filter (None = All).
    pub layer_filter: RwSignal<Option<String>>,
    /// Promoted durable artifacts.
    pub artifacts: RwSignal<Vec<cybou_protocol::learning::LearnedArtifactLineage>>,
    /// Active task scopes & capability grants.
    pub scopes: RwSignal<Vec<cybou_protocol::governance::TaskScope>>,
    /// Currently inspected candidate evaluation result.
    pub evaluation: RwSignal<Option<cybou_web_contracts::CandidateEvaluationProjection>>,
    /// Selected candidate ID.
    pub selected_candidate_id: RwSignal<Option<uuid::Uuid>>,
    /// New candidate proposal layer input.
    pub new_layer: RwSignal<String>,
    /// New candidate proposal generalization input.
    pub new_generalization: RwSignal<String>,
    /// New candidate proposal scope input.
    pub new_scope: RwSignal<String>,
    /// Whether proposal drawer/form is open.
    pub is_proposing: RwSignal<bool>,
    /// Background fetch in flight.
    pub loading: RwSignal<bool>,
    /// Status message or toast.
    pub status_msg: RwSignal<Option<String>>,
}

impl LearningSignals {
    fn new() -> Self {
        Self {
            candidates: RwSignal::new(Vec::new()),
            layer_filter: RwSignal::new(None),
            artifacts: RwSignal::new(Vec::new()),
            scopes: RwSignal::new(Vec::new()),
            evaluation: RwSignal::new(None),
            selected_candidate_id: RwSignal::new(None),
            new_layer: RwSignal::new("procedural".to_string()),
            new_generalization: RwSignal::new(String::new()),
            new_scope: RwSignal::new(String::new()),
            is_proposing: RwSignal::new(false),
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
    terminals: StoredValue<HashMap<CardId, TerminalSignals>>,
    file_managers: StoredValue<HashMap<CardId, FileManagerSignals>>,
    editors: StoredValue<HashMap<CardId, EditorSignals>>,
    diffs: StoredValue<HashMap<CardId, DiffSignals>>,
    inspectors: StoredValue<HashMap<CardId, InspectorSignals>>,
    operations: StoredValue<HashMap<CardId, OperationsSignals>>,
    notifications: StoredValue<HashMap<CardId, NotificationsSignals>>,
    services: StoredValue<HashMap<CardId, ServicesSignals>>,
    processes: StoredValue<HashMap<CardId, ProcessesSignals>>,
    monitors: StoredValue<HashMap<CardId, MonitorSignals>>,
    system_logs: StoredValue<HashMap<CardId, SystemLogsSignals>>,
    storage: StoredValue<HashMap<CardId, StorageSignals>>,
    networks: StoredValue<HashMap<CardId, NetworkSignals>>,
    packages: StoredValue<HashMap<CardId, PackagesSignals>>,
    updates: StoredValue<HashMap<CardId, UpdatesSignals>>,
    user_settings: StoredValue<HashMap<CardId, UserSettingsSignals>>,
    security: StoredValue<HashMap<CardId, SecuritySignals>>,
    backups: StoredValue<HashMap<CardId, BackupSignals>>,
    mails: StoredValue<HashMap<CardId, MailSignals>>,
    calendars: StoredValue<HashMap<CardId, CalendarSignals>>,
    notes: StoredValue<HashMap<CardId, NotesSignals>>,
    contacts: StoredValue<HashMap<CardId, ContactsSignals>>,
    cognitive_graphs: StoredValue<HashMap<CardId, CognitiveGraphSignals>>,
    event_journals: StoredValue<HashMap<CardId, EventJournalSignals>>,
    meanings: StoredValue<HashMap<CardId, MeaningSignals>>,
    learnings: StoredValue<HashMap<CardId, LearningSignals>>,
}

impl ToolCardStates {
    /// Build the store under the current reactive owner.
    #[must_use]
    pub fn new() -> Self {
        let owner = Owner::current().expect("a reactive owner to anchor tool card state to");
        Self {
            owner: StoredValue::new(owner),
            shells: StoredValue::new(HashMap::new()),
            terminals: StoredValue::new(HashMap::new()),
            file_managers: StoredValue::new(HashMap::new()),
            editors: StoredValue::new(HashMap::new()),
            diffs: StoredValue::new(HashMap::new()),
            inspectors: StoredValue::new(HashMap::new()),
            operations: StoredValue::new(HashMap::new()),
            notifications: StoredValue::new(HashMap::new()),
            services: StoredValue::new(HashMap::new()),
            processes: StoredValue::new(HashMap::new()),
            monitors: StoredValue::new(HashMap::new()),
            system_logs: StoredValue::new(HashMap::new()),
            storage: StoredValue::new(HashMap::new()),
            networks: StoredValue::new(HashMap::new()),
            packages: StoredValue::new(HashMap::new()),
            updates: StoredValue::new(HashMap::new()),
            user_settings: StoredValue::new(HashMap::new()),
            security: StoredValue::new(HashMap::new()),
            backups: StoredValue::new(HashMap::new()),
            mails: StoredValue::new(HashMap::new()),
            calendars: StoredValue::new(HashMap::new()),
            notes: StoredValue::new(HashMap::new()),
            contacts: StoredValue::new(HashMap::new()),
            cognitive_graphs: StoredValue::new(HashMap::new()),
            event_journals: StoredValue::new(HashMap::new()),
            meanings: StoredValue::new(HashMap::new()),
            learnings: StoredValue::new(HashMap::new()),
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

    /// This Terminal card's state, creating it the first time the card is shown.
    ///
    /// Held here rather than in the component, so closing a Terminal panel and reopening it finds
    /// the same session — closing a card is a presentation act and must not end a shell.
    #[must_use]
    pub fn terminal(&self, card: CardId) -> TerminalSignals {
        if let Some(existing) = self.terminals.with_value(|held| held.get(&card).copied()) {
            return existing;
        }
        let created = self
            .owner
            .with_value(|owner| owner.with(TerminalSignals::new));
        self.terminals.update_value(|held| {
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

    /// This Operations Manager card's state, creating it the first time the card is shown.
    #[must_use]
    pub fn operations(&self, card: CardId) -> OperationsSignals {
        if let Some(existing) = self.operations.with_value(|held| held.get(&card).copied()) {
            return existing;
        }
        let created = self
            .owner
            .with_value(|owner| owner.with(OperationsSignals::new));
        self.operations.update_value(|held| {
            held.insert(card, created);
        });
        created
    }

    /// This Notifications Center card's state, creating it the first time the card is shown.
    #[must_use]
    pub fn notifications(&self, card: CardId) -> NotificationsSignals {
        if let Some(existing) = self
            .notifications
            .with_value(|held| held.get(&card).copied())
        {
            return existing;
        }
        let created = self
            .owner
            .with_value(|owner| owner.with(NotificationsSignals::new));
        self.notifications.update_value(|held| {
            held.insert(card, created);
        });
        created
    }

    /// This Services Manager card's state, creating it the first time the card is shown.
    #[must_use]
    pub fn services(&self, card: CardId) -> ServicesSignals {
        if let Some(existing) = self.services.with_value(|held| held.get(&card).copied()) {
            return existing;
        }
        let created = self
            .owner
            .with_value(|owner| owner.with(ServicesSignals::new));
        self.services.update_value(|held| {
            held.insert(card, created);
        });
        created
    }

    /// This Process Manager card's state, creating it the first time the card is shown.
    #[must_use]
    pub fn processes(&self, card: CardId) -> ProcessesSignals {
        if let Some(existing) = self.processes.with_value(|held| held.get(&card).copied()) {
            return existing;
        }
        let created = self
            .owner
            .with_value(|owner| owner.with(ProcessesSignals::new));
        self.processes.update_value(|held| {
            held.insert(card, created);
        });
        created
    }

    /// This System Monitor card's state, creating it the first time the card is shown.
    #[must_use]
    pub fn monitor(&self, card: CardId) -> MonitorSignals {
        if let Some(existing) = self.monitors.with_value(|held| held.get(&card).copied()) {
            return existing;
        }
        let created = self
            .owner
            .with_value(|owner| owner.with(MonitorSignals::new));
        self.monitors.update_value(|held| {
            held.insert(card, created);
        });
        created
    }

    /// This System Log Viewer card's state, creating it the first time the card is shown.
    #[must_use]
    pub fn system_logs(&self, card: CardId) -> SystemLogsSignals {
        if let Some(existing) = self.system_logs.with_value(|held| held.get(&card).copied()) {
            return existing;
        }
        let created = self
            .owner
            .with_value(|owner| owner.with(SystemLogsSignals::new));
        self.system_logs.update_value(|held| {
            held.insert(card, created);
        });
        created
    }

    /// This Storage & Snapshots card's state, creating it the first time the card is shown.
    #[must_use]
    pub fn storage(&self, card: CardId) -> StorageSignals {
        if let Some(existing) = self.storage.with_value(|held| held.get(&card).copied()) {
            return existing;
        }
        let created = self
            .owner
            .with_value(|owner| owner.with(StorageSignals::new));
        self.storage.update_value(|held| {
            held.insert(card, created);
        });
        created
    }

    /// This Network Connections card's state, creating it the first time the card is shown.
    #[must_use]
    pub fn network(&self, card: CardId) -> NetworkSignals {
        if let Some(existing) = self.networks.with_value(|held| held.get(&card).copied()) {
            return existing;
        }
        let created = self
            .owner
            .with_value(|owner| owner.with(NetworkSignals::new));
        self.networks.update_value(|held| {
            held.insert(card, created);
        });
        created
    }

    /// This Package Manager card's state, creating it the first time the card is shown.
    #[must_use]
    pub fn packages(&self, card: CardId) -> PackagesSignals {
        if let Some(existing) = self.packages.with_value(|held| held.get(&card).copied()) {
            return existing;
        }
        let created = self
            .owner
            .with_value(|owner| owner.with(PackagesSignals::new));
        self.packages.update_value(|held| {
            held.insert(card, created);
        });
        created
    }

    /// This System Updates card's state, creating it the first time the card is shown.
    #[must_use]
    pub fn updates(&self, card: CardId) -> UpdatesSignals {
        if let Some(existing) = self.updates.with_value(|held| held.get(&card).copied()) {
            return existing;
        }
        let created = self
            .owner
            .with_value(|owner| owner.with(UpdatesSignals::new));
        self.updates.update_value(|held| {
            held.insert(card, created);
        });
        created
    }

    /// This Users & SSH Keys card's state, creating it the first time the card is shown.
    #[must_use]
    pub fn user_settings(&self, card: CardId) -> UserSettingsSignals {
        if let Some(existing) = self
            .user_settings
            .with_value(|held| held.get(&card).copied())
        {
            return existing;
        }
        let created = self
            .owner
            .with_value(|owner| owner.with(UserSettingsSignals::new));
        self.user_settings.update_value(|held| {
            held.insert(card, created);
        });
        created
    }

    /// This Security Policy & Audit card's state, creating it the first time the card is shown.
    #[must_use]
    pub fn security(&self, card: CardId) -> SecuritySignals {
        if let Some(existing) = self.security.with_value(|held| held.get(&card).copied()) {
            return existing;
        }
        let created = self
            .owner
            .with_value(|owner| owner.with(SecuritySignals::new));
        self.security.update_value(|held| {
            held.insert(card, created);
        });
        created
    }

    /// This Backup & Vault card's state, creating it the first time the card is shown.
    #[must_use]
    pub fn backup(&self, card: CardId) -> BackupSignals {
        if let Some(existing) = self.backups.with_value(|held| held.get(&card).copied()) {
            return existing;
        }
        let created = self
            .owner
            .with_value(|owner| owner.with(BackupSignals::new));
        self.backups.update_value(|held| {
            held.insert(card, created);
        });
        created
    }

    /// This Mail & Messages card's state, creating it the first time the card is shown.
    #[must_use]
    pub fn mail(&self, card: CardId) -> MailSignals {
        if let Some(existing) = self.mails.with_value(|held| held.get(&card).copied()) {
            return existing;
        }
        let created = self.owner.with_value(|owner| owner.with(MailSignals::new));
        self.mails.update_value(|held| {
            held.insert(card, created);
        });
        created
    }

    /// This Calendar & Schedule card's state, creating it the first time the card is shown.
    #[must_use]
    pub fn calendar(&self, card: CardId) -> CalendarSignals {
        if let Some(existing) = self.calendars.with_value(|held| held.get(&card).copied()) {
            return existing;
        }
        let created = self
            .owner
            .with_value(|owner| owner.with(CalendarSignals::new));
        self.calendars.update_value(|held| {
            held.insert(card, created);
        });
        created
    }

    /// This Notes & Ideas card's state, creating it the first time the card is shown.
    #[must_use]
    pub fn notes(&self, card: CardId) -> NotesSignals {
        if let Some(existing) = self.notes.with_value(|held| held.get(&card).copied()) {
            return existing;
        }
        let created = self.owner.with_value(|owner| owner.with(NotesSignals::new));
        self.notes.update_value(|held| {
            held.insert(card, created);
        });
        created
    }

    /// This Contacts Directory card's state, creating it the first time the card is shown.
    #[must_use]
    pub fn contacts(&self, card: CardId) -> ContactsSignals {
        if let Some(existing) = self.contacts.with_value(|held| held.get(&card).copied()) {
            return existing;
        }
        let created = self
            .owner
            .with_value(|owner| owner.with(ContactsSignals::new));
        self.contacts.update_value(|held| {
            held.insert(card, created);
        });
        created
    }

    /// This Cognitive Graph & Causal DAG card's state, creating it the first time the card is shown.
    #[must_use]
    pub fn cognitive_graph(&self, card: CardId) -> CognitiveGraphSignals {
        if let Some(existing) = self
            .cognitive_graphs
            .with_value(|held| held.get(&card).copied())
        {
            return existing;
        }
        let created = self
            .owner
            .with_value(|owner| owner.with(CognitiveGraphSignals::new));
        self.cognitive_graphs.update_value(|held| {
            held.insert(card, created);
        });
        created
    }

    /// This Canonical Event1 Journal card's state, creating it the first time the card is shown.
    #[must_use]
    pub fn event_journal(&self, card: CardId) -> EventJournalSignals {
        if let Some(existing) = self
            .event_journals
            .with_value(|held| held.get(&card).copied())
        {
            return existing;
        }
        let created = self
            .owner
            .with_value(|owner| owner.with(EventJournalSignals::new));
        self.event_journals.update_value(|held| {
            held.insert(card, created);
        });
        created
    }

    /// This Meaning & Dialogue Assistant card's state, creating it the first time the card is shown.
    #[must_use]
    pub fn meaning(&self, card: CardId) -> MeaningSignals {
        if let Some(existing) = self.meanings.with_value(|held| held.get(&card).copied()) {
            return existing;
        }
        let created = self
            .owner
            .with_value(|owner| owner.with(MeaningSignals::new));
        self.meanings.update_value(|held| {
            held.insert(card, created);
        });
        created
    }

    /// This Lifelong Learning & Governance card's state, creating it the first time the card is shown.
    #[must_use]
    pub fn learning(&self, card: CardId) -> LearningSignals {
        if let Some(existing) = self.learnings.with_value(|held| held.get(&card).copied()) {
            return existing;
        }
        let created = self
            .owner
            .with_value(|owner| owner.with(LearningSignals::new));
        self.learnings.update_value(|held| {
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
        self.meanings.update_value(|held| {
            held.remove(&card);
        });
        self.learnings.update_value(|held| {
            held.remove(&card);
        });
    }
}

impl Default for ToolCardStates {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute 1-indexed (line, column) for a browser UTF-16 code-unit offset.
#[must_use]
pub fn calculate_line_column(text: &str, utf16_offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    let mut consumed = 0;
    for ch in text.chars() {
        if consumed >= utf16_offset {
            break;
        }
        consumed += ch.len_utf16();
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

/// Convert Rust character offsets to the UTF-16 offsets expected by textarea selection APIs.
#[must_use]
pub fn char_range_to_utf16(text: &str, start: usize, end: usize) -> (usize, usize) {
    let mut utf16 = 0;
    let mut utf16_start = 0;
    let mut utf16_end = 0;
    for (index, ch) in text.chars().enumerate() {
        if index == start {
            utf16_start = utf16;
        }
        if index == end {
            utf16_end = utf16;
            return (utf16_start, utf16_end);
        }
        utf16 += ch.len_utf16();
    }
    if start == text.chars().count() {
        utf16_start = utf16;
    }
    if end >= text.chars().count() {
        utf16_end = utf16;
    }
    (utf16_start, utf16_end)
}

/// Preserve both output channels while making their provenance explicit when both contain data.
#[must_use]
pub fn merge_shell_output(stdout: &str, stderr: &str) -> String {
    match (stdout.is_empty(), stderr.is_empty()) {
        (false, true) => stdout.to_string(),
        (true, false) => stderr.to_string(),
        (true, true) => String::new(),
        (false, false) => format!("[stdout]\n{stdout}\n[stderr]\n{stderr}"),
    }
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

/// Parse a filesystem path into hierarchical breadcrumbs (label, `target_path`).
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

        let emoji = "A😀B\n𝄞C";
        assert_eq!(calculate_line_column(emoji, 3), (1, 3));
        assert_eq!(calculate_line_column(emoji, 4), (1, 4));
        assert_eq!(calculate_line_column(emoji, 5), (2, 1));
        assert_eq!(calculate_line_column(emoji, 7), (2, 2));
        assert_eq!(char_range_to_utf16(emoji, 1, 3), (1, 4));
    }

    #[test]
    fn shell_output_never_drops_stderr_when_stdout_exists() {
        assert_eq!(merge_shell_output("ok\n", ""), "ok\n");
        assert_eq!(merge_shell_output("", "warning\n"), "warning\n");
        assert_eq!(
            merge_shell_output("ok\n", "warning\n"),
            "[stdout]\nok\n\n[stderr]\nwarning\n"
        );
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
