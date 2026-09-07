// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Command palette action launcher and fuzzy navigation menu.

use leptos::prelude::*;
use lucide_leptos::{Anchor, Box as BoxIcon, FileText, Link, ListChecks, Search, Sparkles};
use web_sys::KeyboardEvent;

use crate::interaction::usable_viewport;
use crate::{
    ArrangementMode, CardId, DesktopItemId, DesktopLayout, LayoutHistory, MindClient,
    components::icons::{
        IconExternalLink, IconGrid, IconLayers, IconMaximize, IconMinimize, IconPin, IconRedo,
        IconRefresh, IconUndo,
    },
    interaction::{apply_redo, apply_undo},
    state::command_matches,
};

/// High-level category for desktop commands.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaletteCategory {
    /// Sandboxed and desktop tools
    Tools,
    /// Cognitive mind organs
    Organs,
    /// Spatial desktop layout actions
    Layout,
    /// Security & session actions
    Session,
}

impl PaletteCategory {
    /// Category display title.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Tools => "Tools & Applications",
            Self::Organs => "Cognitive Organs",
            Self::Layout => "Canvas & Layout",
            Self::Session => "Session & Auth",
        }
    }
}

/// Action item displayed in the command palette.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PaletteAction {
    /// Unique identifier for dispatch.
    pub id: &'static str,
    /// Grouping category.
    pub category: PaletteCategory,
    /// Primary title.
    pub title: &'static str,
    /// Explanatory subtitle.
    pub subtitle: &'static str,
    /// Keywords for fuzzy search matching.
    pub keywords: &'static str,
    /// Keyboard shortcut hint, if any.
    pub shortcut: Option<&'static str>,
    /// Icon kind string.
    pub icon_kind: &'static str,
}

const ALL_PALETTE_ACTIONS: &[PaletteAction] = &[
    // Tools & Apps
    PaletteAction {
        id: "files",
        category: PaletteCategory::Tools,
        title: "Open File Manager",
        subtitle: "Browse, edit and transfer files",
        keywords: "files file manager storage browse create edit directory breadcrumbs",
        shortcut: None,
        icon_kind: "external",
    },
    PaletteAction {
        id: "editor",
        category: PaletteCategory::Tools,
        title: "Open Text Editor",
        subtitle: "Code and configuration editor with drafts",
        keywords: "editor text code write config edit buffer markdown",
        shortcut: None,
        icon_kind: "external",
    },
    PaletteAction {
        id: "diff",
        category: PaletteCategory::Tools,
        title: "Open Diff Viewer",
        subtitle: "Inspect and review changes",
        keywords: "diff compare review changes patch side-by-side",
        shortcut: None,
        icon_kind: "external",
    },
    PaletteAction {
        id: "inspector",
        category: PaletteCategory::Tools,
        title: "Open Universal Inspector",
        subtitle: "Deep entity state & relations",
        keywords: "inspector inspect service entity process details subject",
        shortcut: None,
        icon_kind: "external",
    },
    PaletteAction {
        id: "operations",
        category: PaletteCategory::Tools,
        title: "Open Operations Monitor",
        subtitle: "Background tasks, installs, and progress",
        keywords: "operations tasks background progress jobs cancel logs monitor substrate",
        shortcut: None,
        icon_kind: "sparkles",
    },
    PaletteAction {
        id: "notifications",
        category: PaletteCategory::Tools,
        title: "Open Notifications Center",
        subtitle: "Attention alerts, proposals, and events",
        keywords: "notifications attention evidence alerts proposals approve reject dismiss bell",
        shortcut: None,
        icon_kind: "sparkles",
    },
    PaletteAction {
        id: "services",
        category: PaletteCategory::Tools,
        title: "Open Services Manager",
        subtitle: "Systemd units and daemon status",
        keywords: "services daemons systemd units start stop restart reload status",
        shortcut: None,
        icon_kind: "external",
    },
    PaletteAction {
        id: "processes",
        category: PaletteCategory::Tools,
        title: "Open Process Manager",
        subtitle: "Active OS tasks and CPU/RAM metrics",
        keywords: "processes tasks kill term top ps monitor signals pid cpu memory",
        shortcut: None,
        icon_kind: "external",
    },
    PaletteAction {
        id: "monitor",
        category: PaletteCategory::Tools,
        title: "Open System Monitor",
        subtitle: "Hardware telemetry and resource meters",
        keywords: "monitor system hardware cpu memory ram swap disks network load telemetry",
        shortcut: None,
        icon_kind: "sparkles",
    },
    PaletteAction {
        id: "terminal",
        category: PaletteCategory::Tools,
        title: "Open Terminal",
        subtitle: "Interactive shell running as your account",
        keywords: "terminal shell tty console pty bash sh command line prompt interactive",
        shortcut: None,
        icon_kind: "terminal",
    },
    PaletteAction {
        id: "system-logs",
        category: PaletteCategory::Tools,
        title: "Open System Logs",
        subtitle: "Journald logs and daemon feed",
        keywords: "logs system journal journald syslog errors warnings stdout stderr",
        shortcut: None,
        icon_kind: "external",
    },
    PaletteAction {
        id: "storage",
        category: PaletteCategory::Tools,
        title: "Open Storage Manager",
        subtitle: "Btrfs subvolumes and snapshots",
        keywords: "storage btrfs subvolumes snapshots backup restore pool quota",
        shortcut: None,
        icon_kind: "external",
    },
    PaletteAction {
        id: "network",
        category: PaletteCategory::Tools,
        title: "Open Network Connections",
        subtitle: "Interfaces, Wi-Fi, and VPN tunnels",
        keywords: "network wifi ethernet tailscale wireguard vpn ip dns gateway interfaces",
        shortcut: None,
        icon_kind: "external",
    },
    PaletteAction {
        id: "packages",
        category: PaletteCategory::Tools,
        title: "Open Package Manager",
        subtitle: "Software repository search & installs",
        keywords: "packages software apt dnf repo repositories install upgrade remove",
        shortcut: None,
        icon_kind: "external",
    },
    PaletteAction {
        id: "updates",
        category: PaletteCategory::Tools,
        title: "Open System Updates",
        subtitle: "Kernel and system software updates",
        keywords: "updates upgrade system kernel patches security reboot",
        shortcut: None,
        icon_kind: "sparkles",
    },
    PaletteAction {
        id: "user-settings",
        category: PaletteCategory::Tools,
        title: "Open Users & SSH Keys",
        subtitle: "Local user accounts & authorized keys",
        keywords: "users ssh keys pam password accounts credentials",
        shortcut: None,
        icon_kind: "sparkles",
    },
    PaletteAction {
        id: "security",
        category: PaletteCategory::Tools,
        title: "Open Security & Sandboxing",
        subtitle: "Landlock, Bubblewrap, Seccomp policies & audit",
        keywords: "security sandbox landlock bubblewrap seccomp apparmor firewall audit",
        shortcut: None,
        icon_kind: "sparkles",
    },
    PaletteAction {
        id: "backup",
        category: PaletteCategory::Tools,
        title: "Open Backup Vault",
        subtitle: "Borg deduplicating backups & snapshot archives",
        keywords: "backup borg btrfs vault snapshots restore schedule retention",
        shortcut: None,
        icon_kind: "sparkles",
    },
    PaletteAction {
        id: "mail",
        category: PaletteCategory::Tools,
        title: "Open Mail & Messages",
        subtitle: "Personal email accounts & messages",
        keywords: "mail email inbox compose messages personal communication",
        shortcut: None,
        icon_kind: "sparkles",
    },
    PaletteAction {
        id: "calendar",
        category: PaletteCategory::Tools,
        title: "Open Calendar & Schedule",
        subtitle: "Personal schedules & cognitive events",
        keywords: "calendar schedule events appointments time personal",
        shortcut: None,
        icon_kind: "sparkles",
    },
    PaletteAction {
        id: "notes",
        category: PaletteCategory::Tools,
        title: "Open Notes & Ideas",
        subtitle: "Markdown notes linked to cognitive subjects",
        keywords: "notes knowledge snippets markdown text memo ideas",
        shortcut: None,
        icon_kind: "sparkles",
    },
    PaletteAction {
        id: "contacts",
        category: PaletteCategory::Tools,
        title: "Open Contacts Directory",
        subtitle: "Address book & cognitive subject directory",
        keywords: "contacts people directory address book colleagues team",
        shortcut: None,
        icon_kind: "sparkles",
    },
    PaletteAction {
        id: "cognitive-graph",
        category: PaletteCategory::Tools,
        title: "Open Cognitive Graph",
        subtitle: "Deep cross-subsystem graph & causal DAG",
        keywords: "cognitive graph causal dag relations entities mind action causality",
        shortcut: None,
        icon_kind: "sparkles",
    },
    PaletteAction {
        id: "event-journal",
        category: PaletteCategory::Tools,
        title: "Open Canonical Event1 Journal",
        subtitle: "Chronological event journal & replay",
        keywords: "event journal log stream audit canonical event1 timeline",
        shortcut: None,
        icon_kind: "sparkles",
    },
    PaletteAction {
        id: "meaning",
        category: PaletteCategory::Tools,
        title: "Open Meaning & Dialogue Assistant",
        subtitle: "Deterministic semantic interpreter & response planner",
        keywords: "meaning assistant dialogue query natural language interpret speech act cognitive plan",
        shortcut: None,
        icon_kind: "sparkles",
    },
    PaletteAction {
        id: "learning",
        category: PaletteCategory::Tools,
        title: "Open Lifelong Learning & Governance",
        subtitle: "Skill induction, promotion gates & capability scopes",
        keywords: "learning skills adaptation induction promotion gates lineage governance scopes artifacts",
        shortcut: None,
        icon_kind: "sparkles",
    },
    PaletteAction {
        id: "outline",
        category: PaletteCategory::Tools,
        title: "Open Canvas Outline",
        subtitle: "Workspace hierarchy & tree view",
        keywords: "outline tree hierarchy accessibility navigation deck",
        shortcut: None,
        icon_kind: "external",
    },
    PaletteAction {
        id: "journal-feed",
        category: PaletteCategory::Tools,
        title: "Open Presence Stream",
        subtitle: "Live Presence1 snapshot projection",
        keywords: "events presence snapshots projection stream sse",
        shortcut: None,
        icon_kind: "external",
    },
    // Cognitive Organs
    PaletteAction {
        id: "insight",
        category: PaletteCategory::Organs,
        title: "Open System Insight",
        subtitle: "Telemetry1 host health & self-healing",
        keywords: "insight telemetry machine health findings why status",
        shortcut: None,
        icon_kind: "sparkles",
    },
    PaletteAction {
        id: "agents",
        category: PaletteCategory::Organs,
        title: "Open Agents",
        subtitle: "Agent1 bounded capsule runtime",
        keywords: "agents agent1 launch opencode task autonomous capsules",
        shortcut: None,
        icon_kind: "list",
    },
    PaletteAction {
        id: "capabilities",
        category: PaletteCategory::Organs,
        title: "Open Capabilities",
        subtitle: "Health1 capability dependencies",
        keywords: "capabilities health dependencies system",
        shortcut: None,
        icon_kind: "sparkles",
    },
    PaletteAction {
        id: "identity",
        category: PaletteCategory::Organs,
        title: "Open Identity",
        subtitle: "Identity1 subject continuity",
        keywords: "identity subject continuity provenance seat",
        shortcut: None,
        icon_kind: "pin",
    },
    PaletteAction {
        id: "session",
        category: PaletteCategory::Organs,
        title: "Open Session",
        subtitle: "Gateway trust and session mode",
        keywords: "session trust gateway authentication mode pam",
        shortcut: None,
        icon_kind: "pin",
    },
    PaletteAction {
        id: "journal",
        category: PaletteCategory::Organs,
        title: "Open Journal",
        subtitle: "Event1 canonical event log",
        keywords: "journal contributions causal integrity event1 history",
        shortcut: None,
        icon_kind: "link",
    },
    PaletteAction {
        id: "lifecycle",
        category: PaletteCategory::Organs,
        title: "Open Lifecycle",
        subtitle: "Lifecycle1 sleep and wake state",
        keywords: "lifecycle sleep wake consolidation idle",
        shortcut: None,
        icon_kind: "sparkles",
    },
    PaletteAction {
        id: "commitments",
        category: PaletteCategory::Organs,
        title: "Open Commitments",
        subtitle: "Intention1 open obligations",
        keywords: "commitments obligations intention1 tasks",
        shortcut: None,
        icon_kind: "list",
    },
    PaletteAction {
        id: "self",
        category: PaletteCategory::Organs,
        title: "Open Self-Model",
        subtitle: "Self1 autobiographical narration",
        keywords: "self assessment autobiographical narration self1",
        shortcut: None,
        icon_kind: "sparkles",
    },
    PaletteAction {
        id: "attention",
        category: PaletteCategory::Organs,
        title: "Open Attention",
        subtitle: "Workspace1 attention focus",
        keywords: "attention focus global workspace theory workspace1",
        shortcut: None,
        icon_kind: "sparkles",
    },
    PaletteAction {
        id: "beliefs",
        category: PaletteCategory::Organs,
        title: "Open Beliefs",
        subtitle: "Epistemic1 derived propositions",
        keywords: "beliefs epistemic1 validity propositions facts",
        shortcut: None,
        icon_kind: "sparkles",
    },
    PaletteAction {
        id: "perception",
        category: PaletteCategory::Organs,
        title: "Open Perception",
        subtitle: "Perception1 host facts",
        keywords: "perception observations host perception1 sensors",
        shortcut: None,
        icon_kind: "link",
    },
    PaletteAction {
        id: "context",
        category: PaletteCategory::Organs,
        title: "Open Context",
        subtitle: "Context1 associative graph",
        keywords: "context association concepts context1 nodes",
        shortcut: None,
        icon_kind: "link",
    },
    // Canvas & Layout Actions
    PaletteAction {
        id: "fit-all",
        category: PaletteCategory::Layout,
        title: "Fit All to Viewport",
        subtitle: "Center and scale entire canvas",
        keywords: "fit all zoom viewport center bounds",
        shortcut: Some("Ctrl+0"),
        icon_kind: "maximize",
    },
    PaletteAction {
        id: "undo",
        category: PaletteCategory::Layout,
        title: "Undo Layout Change",
        subtitle: "Revert position or deck state",
        keywords: "undo layout revert previous",
        shortcut: Some("Ctrl+Z"),
        icon_kind: "undo",
    },
    PaletteAction {
        id: "redo",
        category: PaletteCategory::Layout,
        title: "Redo Layout Change",
        subtitle: "Re-apply position or deck state",
        keywords: "redo layout forward next",
        shortcut: Some("Ctrl+Y"),
        icon_kind: "redo",
    },
    PaletteAction {
        id: "arrange-home",
        category: PaletteCategory::Layout,
        title: "Arrange: Home",
        subtitle: "Canonical workspace overview",
        keywords: "arrange home canonical default layout overview",
        shortcut: None,
        icon_kind: "refresh",
    },
    PaletteAction {
        id: "arrange-grid",
        category: PaletteCategory::Layout,
        title: "Arrange: Grid",
        subtitle: "Structured multi-track lanes",
        keywords: "arrange grid structured columns lanes",
        shortcut: None,
        icon_kind: "grid",
    },
    PaletteAction {
        id: "arrange-compact",
        category: PaletteCategory::Layout,
        title: "Arrange: Compact",
        subtitle: "Dense obstacle-free packing",
        keywords: "arrange compact packing fit dense",
        shortcut: None,
        icon_kind: "minimize",
    },
    PaletteAction {
        id: "arrange-relations",
        category: PaletteCategory::Layout,
        title: "Arrange: Relations",
        subtitle: "Mind causal organ graph flow",
        keywords: "arrange relations causal flow graph",
        shortcut: None,
        icon_kind: "link",
    },
    PaletteAction {
        id: "group-mind-core",
        category: PaletteCategory::Layout,
        title: "Create Deck: Mind Core",
        subtitle: "Group Identity and Session into tabbed deck",
        keywords: "group mind core deck cards merge",
        shortcut: None,
        icon_kind: "layers",
    },
    PaletteAction {
        id: "reset-desktop",
        category: PaletteCategory::Layout,
        title: "Reset Desktop Layout",
        subtitle: "Restore default positions and clear custom decks",
        keywords: "reset layout desktop restore initial",
        shortcut: None,
        icon_kind: "refresh",
    },
    // Session Actions
    PaletteAction {
        id: "auth-modal",
        category: PaletteCategory::Session,
        title: "Authenticate / Sign in",
        subtitle: "Unlock full capabilities with Linux PAM",
        keywords: "auth sign in login pam credentials password",
        shortcut: None,
        icon_kind: "pin",
    },
];

fn render_icon(kind: &'static str) -> AnyView {
    match kind {
        "sparkles" => view! { <Sparkles size=15 /> }.into_any(),
        "list" => view! { <ListChecks size=15 /> }.into_any(),
        "pin" => view! { <IconPin size=15 /> }.into_any(),
        "link" => view! { <Link size=15 /> }.into_any(),
        "external" => view! { <IconExternalLink size=15 /> }.into_any(),
        "layers" => view! { <IconLayers size=15 /> }.into_any(),
        "undo" => view! { <IconUndo size=15 /> }.into_any(),
        "redo" => view! { <IconRedo size=15 /> }.into_any(),
        "refresh" => view! { <IconRefresh size=15 /> }.into_any(),
        "grid" => view! { <IconGrid size=15 /> }.into_any(),
        "minimize" => view! { <IconMinimize size=15 /> }.into_any(),
        "maximize" => view! { <IconMaximize size=15 /> }.into_any(),
        _ => view! { <Sparkles size=15 /> }.into_any(),
    }
}

/// Unified item in the command palette search results.
#[derive(Clone, Debug, PartialEq)]
pub enum PaletteEntry {
    /// Built-in action launcher.
    Action(PaletteAction),
    /// Spatial camera landmark / anchor.
    Anchor {
        /// Unique anchor identifier.
        id: String,
        /// Human-readable label.
        name: String,
        /// Center X coordinate in pixels.
        center_x: f64,
        /// Center Y coordinate in pixels.
        center_y: f64,
        /// Preferred zoom level.
        preferred_zoom: f64,
    },
    /// Semantic panel cluster.
    Cluster {
        /// Unique cluster identifier.
        id: String,
        /// Human-readable label.
        label: String,
        /// Cluster color token.
        color: String,
        /// Number of cards in this cluster.
        card_count: usize,
    },
    /// Personal markdown note / knowledge snippet.
    Note {
        /// Unique note identifier.
        id: String,
        /// Note title.
        title: String,
        /// Markdown content snippet.
        content: String,
        /// Tag list.
        tags: Vec<String>,
        /// Whether note is pinned.
        is_pinned: bool,
        /// Referenced system subject.
        referenced_subject: Option<cybou_protocol::SubjectRef>,
    },
}

impl PaletteEntry {
    /// Unique identifier for keyed list rendering.
    #[must_use]
    pub fn key(&self) -> String {
        match self {
            Self::Action(a) => format!("action:{}", a.id),
            Self::Anchor { id, .. } => format!("anchor:{id}"),
            Self::Cluster { id, .. } => format!("cluster:{id}"),
            Self::Note { id, .. } => format!("note:{id}"),
        }
    }
}

/// Command palette modal and shortcut launcher.
#[component]
pub fn CommandPalette(
    layout: RwSignal<DesktopLayout>,
    history: RwSignal<LayoutHistory>,
    set_selected: WriteSignal<Option<DesktopItemId>>,
    auth_modal_open: RwSignal<bool>,
    command_open: ReadSignal<bool>,
    set_command_open: WriteSignal<bool>,
    command_query: ReadSignal<String>,
    set_command_query: WriteSignal<String>,
    command_input: NodeRef<leptos::html::Input>,
    set_zoom: WriteSignal<f64>,
    set_pan: WriteSignal<(f64, f64)>,
    #[prop(default = RwSignal::new(crate::state::RuntimeState::Loading))] runtime: RwSignal<
        crate::state::RuntimeState,
    >,
) -> impl IntoView {
    let selected_index = RwSignal::new(0usize);
    let tool_states = use_context::<crate::tool_state::ToolCardStates>();

    let filtered_entries = Memo::new(move |_| {
        let q = command_query.get();
        let trimmed = q.trim();
        if trimmed.is_empty() {
            return ALL_PALETTE_ACTIONS
                .iter()
                .copied()
                .map(PaletteEntry::Action)
                .collect::<Vec<_>>();
        }

        let q_lower = trimmed.to_lowercase();
        let mut entries = Vec::new();

        // 1. Standard actions
        for action in ALL_PALETTE_ACTIONS {
            let search_target = format!("{} {} {}", action.title, action.subtitle, action.keywords);
            if command_matches(trimmed, &search_target) {
                entries.push(PaletteEntry::Action(*action));
            }
        }

        // 2. Spatial anchors
        let current_layout = layout.get();
        for anchor in &current_layout.anchors {
            let search_target = format!("anchor spatial {} {}", anchor.name, anchor.id);
            if command_matches(trimmed, &search_target)
                || anchor.name.to_lowercase().contains(&q_lower)
            {
                entries.push(PaletteEntry::Anchor {
                    id: anchor.id.clone(),
                    name: anchor.name.clone(),
                    center_x: anchor.center_x,
                    center_y: anchor.center_y,
                    preferred_zoom: anchor.preferred_zoom,
                });
            }
        }

        // 3. Spatial clusters
        for cluster in &current_layout.clusters {
            let search_target = format!("cluster group {} {}", cluster.label, cluster.id);
            if command_matches(trimmed, &search_target)
                || cluster.label.to_lowercase().contains(&q_lower)
            {
                entries.push(PaletteEntry::Cluster {
                    id: cluster.id.clone(),
                    label: cluster.label.clone(),
                    color: cluster.color.clone(),
                    card_count: cluster.card_keys.len(),
                });
            }
        }

        // 4. Personal notes & ideas
        if let Some(states) = tool_states {
            let notes_list = states.notes(CardId::Notes(0)).notes.get();
            for note in notes_list {
                let search_target = format!(
                    "note idea {} {} {}",
                    note.title,
                    note.content_markdown,
                    note.tags.join(" ")
                );
                if command_matches(trimmed, &search_target)
                    || note.title.to_lowercase().contains(&q_lower)
                {
                    entries.push(PaletteEntry::Note {
                        id: note.id,
                        title: note.title,
                        content: note.content_markdown,
                        tags: note.tags,
                        is_pinned: note.is_pinned,
                        referenced_subject: note.referenced_subject,
                    });
                }
            }
        }

        entries
    });

    // The camera, so a card opens where this person is looking rather than where the canvas
    // happens to begin. Optional because the palette is mounted in tests without one.
    let camera_pan = use_context::<ReadSignal<(f64, f64)>>();
    let camera_zoom = use_context::<ReadSignal<f64>>();
    let camera_history = use_context::<RwSignal<crate::layout::camera::CameraHistory>>();

    let fly_to = move |cx: f64, cy: f64, target_zoom: f64| {
        #[cfg(target_arch = "wasm32")]
        {
            if let (Some(pan), Some(zoom)) = (camera_pan, camera_zoom) {
                crate::apply_camera_fly_to(
                    camera_history,
                    pan,
                    set_pan,
                    zoom,
                    set_zoom,
                    cx,
                    cy,
                    target_zoom,
                );
            } else {
                set_pan.set((cx, cy));
                set_zoom.set(target_zoom);
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (camera_history, camera_pan, camera_zoom);
            set_pan.set((cx, cy));
            set_zoom.set(target_zoom);
        }
    };

    let fit_cluster_and_fly = move |cluster: &crate::layout::DesktopCluster| {
        let current_layout = layout.get_untracked();
        if let Some(rect) = current_layout.cluster_rect(cluster) {
            let (w, h) = (
                web_sys::window()
                    .and_then(|w| w.inner_width().ok())
                    .and_then(|v| v.as_f64())
                    .unwrap_or(1440.0),
                web_sys::window()
                    .and_then(|w| w.inner_height().ok())
                    .and_then(|v| v.as_f64())
                    .unwrap_or(900.0),
            );
            let (target_zoom, _) = DesktopLayout::fit_to_viewport(rect, w, h, 60.0);
            let cx = rect.x + rect.width / 2.0;
            let cy = rect.y + rect.height / 2.0;
            fly_to(cx, cy, target_zoom.clamp(0.6, 1.2));
        }
    };

    let focus_or_open_card = move |card: CardId| {
        set_selected.set(Some(DesktopItemId::Card(card)));
        if !layout.get().contains_card(card) {
            let view = crate::interaction::visible_canvas_rect(
                camera_pan.map_or((0.0, 0.0), |signal| signal.get()),
                camera_zoom.map_or(1.0, |signal| signal.get()),
            );
            let spot = layout
                .get_untracked()
                .free_spot_in(card.spec().default_size, view);
            layout.update(|l| l.open_card(card, spot.0, spot.1));
        } else if layout.get().presentation(card).collapsed {
            layout.update(|l| l.set_collapsed(card, false));
        }
        layout.update(|l| l.bring_forward(card));
        layout.get_untracked().save();

        #[cfg(target_arch = "wasm32")]
        {
            if let Some(camera) = use_context::<crate::components::camera_context::CanvasCamera>() {
                let geom = layout.get_untracked().geometry(card);
                if !camera.shows(geom) {
                    let cx = geom.x + geom.width / 2.0;
                    let cy = geom.y + geom.height / 2.0;
                    let target_z = camera_zoom
                        .map_or(1.0, |z| z.get_untracked())
                        .clamp(0.7, 1.2);
                    fly_to(cx, cy, target_z);
                }
            }
        }

        set_command_open.set(false);
        set_command_query.set(String::new());
    };

    let execute_action = move |action_id: &'static str| match action_id {
        "files" => focus_or_open_card(CardId::FileManager(0)),
        "editor" => focus_or_open_card(CardId::Editor(0)),
        "diff" => focus_or_open_card(CardId::Diff(0)),
        "inspector" => focus_or_open_card(CardId::Inspector(0)),
        "services" => focus_or_open_card(CardId::Services(0)),
        "processes" => focus_or_open_card(CardId::Processes(0)),
        "monitor" => focus_or_open_card(CardId::Monitor(0)),
        "terminal" => focus_or_open_card(CardId::Terminal(0)),
        "system-logs" => focus_or_open_card(CardId::SystemLogs(0)),
        "storage" => focus_or_open_card(CardId::Storage(0)),
        "network" => focus_or_open_card(CardId::Network(0)),
        "packages" => focus_or_open_card(CardId::Packages(0)),
        "updates" => focus_or_open_card(CardId::Updates(0)),
        "outline" => focus_or_open_card(CardId::Outline),
        "journal-feed" => focus_or_open_card(CardId::JournalFeed(0)),
        "auth-modal" => {
            auth_modal_open.set(true);
            set_command_open.set(false);
            set_command_query.set(String::new());
        }
        "undo" => {
            apply_undo(history, layout);
            set_command_open.set(false);
            set_command_query.set(String::new());
        }
        "redo" => {
            apply_redo(history, layout);
            set_command_open.set(false);
            set_command_query.set(String::new());
        }
        "arrange-home" => {
            history.update(|h| h.push(layout.get_untracked()));
            layout.update(|l| l.apply_arrangement(ArrangementMode::Home, Some(usable_viewport())));
            layout.get_untracked().save();
            set_command_open.set(false);
            set_command_query.set(String::new());
        }
        "arrange-grid" => {
            history.update(|h| h.push(layout.get_untracked()));
            layout.update(|l| l.apply_arrangement(ArrangementMode::Grid, Some(usable_viewport())));
            layout.get_untracked().save();
            set_command_open.set(false);
            set_command_query.set(String::new());
        }
        "arrange-compact" => {
            history.update(|h| h.push(layout.get_untracked()));
            layout
                .update(|l| l.apply_arrangement(ArrangementMode::Compact, Some(usable_viewport())));
            layout.get_untracked().save();
            set_command_open.set(false);
            set_command_query.set(String::new());
        }
        "arrange-relations" => {
            history.update(|h| h.push(layout.get_untracked()));
            layout.update(|l| {
                l.apply_arrangement(ArrangementMode::Relations, Some(usable_viewport()));
            });
            layout.get_untracked().save();
            set_command_open.set(false);
            set_command_query.set(String::new());
        }
        "fit-all" => {
            if let Some(bbox) = layout.get_untracked().bounding_rect() {
                let (w, h) = (
                    web_sys::window()
                        .and_then(|w| w.inner_width().ok())
                        .and_then(|v| v.as_f64())
                        .unwrap_or(1440.0),
                    web_sys::window()
                        .and_then(|w| w.inner_height().ok())
                        .and_then(|v| v.as_f64())
                        .unwrap_or(900.0),
                );
                let (z, (px, py)) = DesktopLayout::fit_to_viewport(bbox, w, h, 60.0);
                set_zoom.set(z);
                set_pan.set((px, py));
            } else {
                set_zoom.set(1.0);
                set_pan.set((0.0, 0.0));
            }
            set_command_open.set(false);
            set_command_query.set(String::new());
        }
        "group-mind-core" => {
            history.update(|h| h.push(layout.get_untracked()));
            layout.update(|l| {
                let _ = l.create_deck(
                    "Mind Core",
                    vec![CardId::Identity, CardId::Session],
                    70.0,
                    50.0,
                );
            });
            layout.get_untracked().save();
            set_command_open.set(false);
            set_command_query.set(String::new());
        }
        "reset-desktop" => {
            history.update(|h| h.push(layout.get_untracked()));
            layout.update(|l| l.reset_desktop(None));
            layout.get_untracked().save();
            set_command_open.set(false);
            set_command_query.set(String::new());
        }
        organ_key => {
            if let Some(card) = CardId::from_key(organ_key) {
                focus_or_open_card(card);
            } else {
                set_command_open.set(false);
                set_command_query.set(String::new());
            }
        }
    };

    let execute_entry = move |entry: PaletteEntry| match entry {
        PaletteEntry::Action(action) => execute_action(action.id),
        PaletteEntry::Anchor {
            center_x,
            center_y,
            preferred_zoom,
            ..
        } => {
            fly_to(center_x, center_y, preferred_zoom);
            set_command_open.set(false);
            set_command_query.set(String::new());
        }
        PaletteEntry::Cluster { id, .. } => {
            let current_layout = layout.get_untracked();
            if let Some(cluster) = current_layout.clusters.iter().find(|c| c.id == id) {
                fit_cluster_and_fly(cluster);
            }
            set_command_open.set(false);
            set_command_query.set(String::new());
        }
        PaletteEntry::Note {
            id,
            title,
            content,
            tags,
            is_pinned,
            referenced_subject,
        } => {
            if let Some(states) = tool_states {
                let notes_signals = states.notes(CardId::Notes(0));
                notes_signals.selected_note_id.set(Some(id));
                notes_signals.edit_title.set(title);
                notes_signals.edit_content.set(content);
                notes_signals.edit_tags.set(tags.join(", "));
                notes_signals.edit_pinned.set(is_pinned);
                notes_signals
                    .edit_referenced_subject
                    .set(referenced_subject);
            }
            focus_or_open_card(CardId::Notes(0));
        }
    };

    Effect::new(move |_| {
        if command_open.get() {
            if let Some(states) = tool_states {
                let notes_sig = states.notes(CardId::Notes(0));
                if notes_sig.notes.get_untracked().is_empty() {
                    leptos::task::spawn_local(async move {
                        if let Ok(proj) = crate::GatewayMindClient.get_notes().await {
                            notes_sig.notes.set(proj.notes);
                        }
                    });
                }
            }
        }
    });

    let ask_answer = move || crate::state::ask_cybou(&command_query.get(), &runtime.get());

    let (meaning_result, set_meaning_result) =
        signal(Option::<cybou_web_contracts::MeaningInterpretProjection>::None);
    let (_meaning_loading, set_meaning_loading) = signal(false);

    Effect::new(move |_| {
        let q = command_query.get();
        let trimmed = q.trim().to_owned();
        if trimmed.len() < 3 || ask_answer().is_some() {
            set_meaning_result.set(None);
            set_meaning_loading.set(false);
            return;
        }
        set_meaning_loading.set(true);
        leptos::task::spawn_local(async move {
            let req = cybou_web_contracts::MeaningInterpretRequest {
                utterance: trimmed,
                language: None,
            };
            match crate::GatewayMindClient.interpret_meaning(&req).await {
                Ok(proj) => {
                    if command_query.get_untracked() == q {
                        set_meaning_result.set(Some(proj));
                    }
                }
                Err(_) => {
                    if command_query.get_untracked() == q {
                        set_meaning_result.set(None);
                    }
                }
            }
            set_meaning_loading.set(false);
        });
    });

    view! {
        <section class="command-palette" aria-label="Action launcher">
            <Show when=move || command_open.get()>
                <nav class="command-menu" aria-label="Command palette actions">
                    {move || {
                        ask_answer().map(|ans| {
                            let target_click = ans.target;
                            view! {
                                <div class="ask-cybou-card">
                                    <div class="ask-cybou-header">
                                        <Sparkles size=14 />
                                        <b>"Ask CYBOU"</b>
                                        <span class="ask-cybou-headline">{ans.headline}</span>
                                    </div>
                                    <p class="ask-cybou-detail">{ans.detail}</p>
                                    {target_click.map(|(label, card)| {
                                        view! {
                                            <button
                                                type="button"
                                                class="ask-cybou-action-btn"
                                                on:click=move |_| focus_or_open_card(card)
                                            >
                                                {label}
                                            </button>
                                        }
                                    })}
                                </div>
                            }
                        })
                    }}

                    {move || {
                        if ask_answer().is_none() {
                            meaning_result.get().map(|proj| {
                                let act_kind = proj.interpretation.primary_act.kind;
                                let kind_str = format!("{act_kind:?}");
                                let realization = proj.realization.clone();
                                let plan_intent = proj.response_plan.as_ref().map(|p| p.intent.clone());
                                let target_card = match act_kind {
                                    cybou_web_contracts::CognitiveActKind::Inspect => CardId::Inspector(0),
                                    _ => CardId::Meaning(0),
                                };
                                let btn_label = match act_kind {
                                    cybou_web_contracts::CognitiveActKind::Inspect => "Open Universal Inspector",
                                    _ => "Open Meaning1 Assistant",
                                };

                                view! {
                                    <div class="ask-cybou-card">
                                        <div class="ask-cybou-header">
                                            <Sparkles size=14 />
                                            <b>"Meaning1 Cognitive Act"</b>
                                            <span class="ask-cybou-headline">{kind_str}</span>
                                        </div>
                                        {realization.map(|text| view! {
                                            <p class="ask-cybou-detail">{text}</p>
                                        })}
                                        {plan_intent.map(|intent| view! {
                                            <div style="font-size: 10px; color: var(--text-dim); margin-bottom: 6px;">
                                                "Plan: " <span style="color: var(--accent-light); font-family: monospace;">{intent}</span>
                                            </div>
                                        })}
                                        <button
                                            type="button"
                                            class="ask-cybou-action-btn"
                                            on:click=move |_| focus_or_open_card(target_card)
                                        >
                                            {btn_label}
                                        </button>
                                    </div>
                                }
                            })
                        } else {
                            None
                        }
                    }}

                    <Show
                        when=move || !filtered_entries.get().is_empty()
                        fallback=move || view! {
                            <div class="command-empty-state">
                                <span>"No matching actions, anchors, clusters, or notes found."</span>
                                <small>"Try searching for 'editor', 'anchor', 'cluster', or notes keywords."</small>
                            </div>
                        }
                    >
                        <For
                            each=move || filtered_entries.get().into_iter().enumerate()
                            key=|(_, entry)| entry.key()
                            children=move |(idx, entry)| {
                                let entry_click = entry.clone();
                                let is_active = move || selected_index.get() == idx;
                                let icon_view = match &entry {
                                    PaletteEntry::Action(action) => render_icon(action.icon_kind),
                                    PaletteEntry::Anchor { .. } => view! { <Anchor size=15 /> }.into_any(),
                                    PaletteEntry::Cluster { .. } => view! { <BoxIcon size=15 /> }.into_any(),
                                    PaletteEntry::Note { .. } => view! { <FileText size=15 /> }.into_any(),
                                };
                                let title_view = match &entry {
                                    PaletteEntry::Action(action) => action.title.to_string(),
                                    PaletteEntry::Anchor { name, .. } => format!("Anchor: {name}"),
                                    PaletteEntry::Cluster { label, .. } => format!("Cluster: {label}"),
                                    PaletteEntry::Note { title, is_pinned, .. } => {
                                        if *is_pinned { format!("📌 {title}") } else { title.clone() }
                                    }
                                };
                                let subtitle_view = match &entry {
                                    PaletteEntry::Action(action) => action.subtitle.to_string(),
                                    PaletteEntry::Anchor { preferred_zoom, .. } => {
                                        format!("Spatial camera landmark (zoom {:.1}x)", preferred_zoom)
                                    }
                                    PaletteEntry::Cluster { card_count, .. } => {
                                        format!("{} panel{} in spatial group", card_count, if *card_count == 1 { "" } else { "s" })
                                    }
                                    PaletteEntry::Note { content, tags, referenced_subject, .. } => {
                                        let first_line = content.lines().next().unwrap_or("").trim();
                                        if let Some(sub) = referenced_subject {
                                            format!("Ref: {} » {} | {}", sub.kind_name(), sub.display_title(), first_line)
                                        } else if !tags.is_empty() {
                                            format!("[{}] {}", tags.join(", "), first_line)
                                        } else {
                                            first_line.to_string()
                                        }
                                    }
                                };
                                let chip_view = match &entry {
                                    PaletteEntry::Action(action) => action.shortcut.map(|sc| view! {
                                        <kbd class="command-shortcut-chip">{sc}</kbd>
                                    }),
                                    PaletteEntry::Anchor { .. } => Some(view! {
                                        <kbd class="command-shortcut-chip">"Anchor"</kbd>
                                    }),
                                    PaletteEntry::Cluster { .. } => Some(view! {
                                        <kbd class="command-shortcut-chip">"Cluster"</kbd>
                                    }),
                                    PaletteEntry::Note { .. } => Some(view! {
                                        <kbd class="command-shortcut-chip">"Note"</kbd>
                                    }),
                                };

                                view! {
                                    <button
                                        type="button"
                                        class:active=is_active
                                        on:click=move |_| execute_entry(entry_click.clone())
                                        on:mouseenter=move |_| selected_index.set(idx)
                                    >
                                        {icon_view}
                                        <span>
                                            <b>{title_view}</b>
                                            <i>{subtitle_view}</i>
                                        </span>
                                        {chip_view}
                                    </button>
                                }
                            }
                        />
                    </Show>
                </nav>
            </Show>

            <label class:open=move || command_open.get() class="command-bar" aria-label="Search or act">
                <Search size=19 />
                <input
                    node_ref=command_input
                    type="search"
                    placeholder="Search tools, anchors, clusters, notes… (↑↓ to navigate, Enter to run)"
                    prop:value=move || command_query.get()
                    on:focus=move |_| set_command_open.set(true)
                    on:input=move |event| {
                        set_command_query.set(event_target_value(&event));
                        selected_index.set(0);
                    }
                    on:keydown=move |event: KeyboardEvent| {
                        let key = event.key();
                        if key == "ArrowDown" {
                            event.prevent_default();
                            let len = filtered_entries.get().len();
                            if len > 0 {
                                selected_index.update(|i| *i = (*i + 1) % len);
                            }
                        } else if key == "ArrowUp" {
                            event.prevent_default();
                            let len = filtered_entries.get().len();
                            if len > 0 {
                                selected_index.update(|i| *i = if *i == 0 { len - 1 } else { *i - 1 });
                            }
                        } else if key == "Enter" {
                            event.prevent_default();
                            let entries = filtered_entries.get();
                            let idx = selected_index.get();
                            if let Some(entry) = entries.get(idx) {
                                execute_entry(entry.clone());
                            } else if let Some(ans) = ask_answer()
                                && let Some((_, card)) = ans.target {
                                    focus_or_open_card(card);
                            } else if let Some(proj) = meaning_result.get() {
                                let target_card = match proj.interpretation.primary_act.kind {
                                    cybou_web_contracts::CognitiveActKind::Inspect => CardId::Inspector(0),
                                    _ => CardId::Meaning(0),
                                };
                                focus_or_open_card(target_card);
                            }
                        } else if key == "Escape" {
                            set_command_open.set(false);
                        }
                    }
                />
                <kbd class="shortcut">"Ctrl+K"</kbd>
            </label>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_entry_keys_are_distinct_and_deterministic() {
        let action = PaletteEntry::Action(ALL_PALETTE_ACTIONS[0]);
        let anchor = PaletteEntry::Anchor {
            id: "anc-1".to_string(),
            name: "Home View".to_string(),
            center_x: 100.0,
            center_y: 200.0,
            preferred_zoom: 1.0,
        };
        let cluster = PaletteEntry::Cluster {
            id: "cls-1".to_string(),
            label: "Production".to_string(),
            color: "cyan".to_string(),
            card_count: 3,
        };
        let note = PaletteEntry::Note {
            id: "note-1".to_string(),
            title: "Architecture notes".to_string(),
            content: "Detailed markdown content".to_string(),
            tags: vec!["design".to_string()],
            is_pinned: false,
            referenced_subject: None,
        };

        assert_eq!(action.key(), "action:files");
        assert_eq!(anchor.key(), "anchor:anc-1");
        assert_eq!(cluster.key(), "cluster:cls-1");
        assert_eq!(note.key(), "note:note-1");
    }

    #[test]
    fn universal_search_matches_anchors_and_clusters() {
        let q = "infra";
        let cluster_label = "Infrastructure Cluster";
        assert!(cluster_label.to_lowercase().contains(q) || command_matches(q, cluster_label));

        let q_anc = "camera";
        let anchor_text = "anchor spatial camera view";
        assert!(command_matches(q_anc, anchor_text));
    }

    #[test]
    fn universal_search_matches_notes_content_and_tags() {
        let note_tags = vec!["critical".to_string(), "database".to_string()];
        let note_text = format!("note idea Postgres crash details {}", note_tags.join(" "));
        assert!(command_matches("postgres", &note_text));
        assert!(command_matches("database", &note_text));
    }
}
