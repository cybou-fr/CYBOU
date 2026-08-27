<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0045: CYBOU Core Desktop Pack: Universal Workspace Primitives, Text Editor, Diff Engine, Files 2.0, Mail & Personal Core, System Control Center, and Authority Domains

## Status

Proposed

## Context

ADR-0044 established the infinite spatial Canvas, Panel 2.0 state machine, camera navigation, semantic zoom, and typed cognitive relations. However, an operating environment that only presents infrastructure telemetry, autonomous agent capsules, and introspective Mind graphs does not yet feel like a **complete browser operating system**.

To make CYBOU a self-sufficient, primary operating environment, operators need a complete suite of standard daily tools: a full-featured text and configuration editor, a multi-panel file manager, an interactive diff engine, terminal emulation, system settings and control center, disk and storage inspection, process and service management, journal logs, network monitoring, package management, background tasks, and communication tools (email, calendar, notes).

However, attempting to ship these tools by embedding a traditional desktop environment (like X11/Wayland window managers, floating window managers, or GNOME clones inside a canvas) or routing between separate Single Page Application (SPA) screens creates severe failure modes:

1. **Window-Manager Emulation Destroys Spatial Presence**: Floating overlapping windows with arbitrary min/max/close controls force operators to manage window clutter rather than focus on spatial comprehension. In CYBOU, every tool is a **Panel** that lives on the same continuous, zoomable 2D Canvas alongside live telemetry and autonomous agents.
2. **Authority and Security Boundary Confusion**: In an ordinary web app, editing `/etc/nginx/nginx.conf` and `/home/user/notes.md` are treated as identical text-box submissions. In CYBOU, modifying a system file must never bypass Action1 governance or execute as an unverified silent write. Saving a privileged configuration requires a typed `FileWrite` proposal, mandatory diff preview, operator authorization, atomic file swap, and post-write validation.
3. **Frontend Secret Leakage in Communication Tools**: Embedding an email client or calendar inside a browser often leads to storing IMAP passwords, SMTP credentials, or OAuth refresh tokens in browser `localStorage` or DOM memory. In CYBOU, the browser is strictly an untrusted renderer: credentials belong in host-managed systemd credential storage, and the web gateway talks to a backend mail daemon.
4. **Lack of a Unified Workspace Authority Model**: When an agent modifies code, a user edits a script, and a terminal runs a command, treating locations as arbitrary filesystem path strings (`/path/to/file`) creates ambiguity regarding sandbox boundaries, root permissions, and snapshot immutability.

This ADR defines the **CYBOU Core Desktop Pack**, formalizing universal workspace primitives (`LocationRef`), the Text Editor & Diff Engine, Files 2.0 multi-panel management, Mail & Personal Core, System Control Center, Dual Terminal architecture, and strict non-cognitive authority invariants.

---

## The Core Desktop Formula & Taxonomy

```text
CYBOU Desktop Pack  = Complete, sovereign browser operating environment on the infinite Canvas
Panel               = Discrete spatial tool / capability (code: Card, UX: Panel)
LocationRef         = Typed authority domain for files, workspaces, sandboxes, and snapshots

Tier 1: Desktop Core (P0)
  ├── Home                  = Live system status, recent actions, active agents, quick dispatch
  ├── Files 2.0             = Multi-panel filesystem manager with drag-and-drop & inline previews
  ├── Text Editor           = Multi-tab, syntax-highlighted editor with diff-before-save & authority gating
  ├── Diff Panel            = Standalone multi-source diff inspector (Editor / Agent / Package / Backup)
  ├── Terminal              = Dual-mode: CYBOU Safe Shell (bounded) + Linux PTY Terminal (interactive)
  ├── Settings              = Universal Control Center (System OS settings vs CYBOU Mind behavior)
  ├── Storage & Disks       = Filesystem mounts, inodes, block devices, SMART health, disk usage treemap
  ├── System Monitor        = CPU / RAM / Swap / IO pressure (PSI), process tree (host / capsule / service)
  ├── Services              = Systemd unit manager, dependency inspection, lifecycle control
  ├── Logs                  = Real-time journal logs viewer with unit, severity, and time filtering
  ├── Network               = Interfaces, IP routing, DNS, listening ports with spatial relation links
  ├── Packages & Updates    = APT package management, security updates, changelog diff, Action1 upgrade
  ├── Tasks & Transfers     = Asynchronous background job tracker & non-blocking file upload/download
  ├── Notifications         = Unified notification center with camera fly-to spatial navigation
  └── Ask CYBOU             = Deterministic natural-language command palette and state query engine

Tier 2: Personal Core (P1)
  ├── Mail                  = IMAP / SMTP / OAuth2 email client with backend secret isolation
  ├── Calendar              = CalDAV / local event scheduling (personal events ≠ system operations)
  ├── Notes                 = User Markdown scratchpad (strictly isolated from Mind Epistemic memory)
  ├── Contacts              = Address book & communication directory
  ├── PDF Viewer            = Lightweight multi-page document viewer
  ├── Image Viewer          = Image inspection, metadata display, and zoom/rotation
  └── Calculator            = Precision desktop calculator

Tier 3: CYBOU-native Core
  ├── Agents                = Autonomous Agent Capsule lifecycle, capacity management, and prompt turns
  ├── Agent Workspaces      = Isolated capsule file inspection, live edits, and execution tracking
  ├── Action Episodes       = Durable 5-stage self-healing lifecycle records and causality audit
  ├── Disclosure & Privacy  = Contextual delivery accounting and sensitivity boundary visualization
  ├── Mind Explorer         = 14-organ relational cognitive map and epistemics inspection
  ├── Telemetry & Forecast  = Predictive trend forecasting and resource degradation detection
  └── Backups & Restore     = Snapshot generation, integrity verification, and historical rollback
```

---

## Decision

### 1. Universal Workspace Primitives & The `LocationRef` Model

All file, editor, diff, terminal, and agent operations in CYBOU MUST NOT use raw unadorned filesystem path strings. Instead, every workspace target is represented by a typed **`LocationRef`** authority domain:

```text
LocationRef
  ├── HostUserPath(PathBuf)             = User workspace directory (e.g. /home/user/projects)
  ├── SystemConfigPath(PathBuf)         = Privileged system file (e.g. /etc/nginx/nginx.conf)
  ├── AgentWorkspace(CapsuleId, RelPath)= Ephemeral isolated agent capsule sandbox
  ├── SafeShellJail(SessionId, PathBuf) = Bounded demo/sandbox shell environment
  └── BackupSnapshot(SnapshotId, PathBuf)= Read-only immutable historical filesystem snapshot
```

#### Authority Invariants by Location Domain

1. **`HostUserPath`**: Standard read and write operations execute directly as the authenticated Linux user (`uid = 1000+`).
2. **`SystemConfigPath`**: Read access is governed by file permissions. Direct in-place writes from the browser are **strictly forbidden**. Saving any modification creates a typed `FileWrite` proposal in `Action1`, requiring diff review, operator policy authorization, atomic staging and replacement via `cybou-executord`, and post-write verification.
3. **`AgentWorkspace`**: Scoped strictly to the cgroup/namespace jail of the corresponding `CapsuleId`. Files can be inspected, dragged into user workspace, or edited, but cannot escape the capsule boundary.
4. **`SafeShellJail`**: Bounded to the demo/safe sandbox environment. Read-only or copy-on-write scratch storage.
5. **`BackupSnapshot`**: Strictly read-only. Files can be viewed, compared in the Diff Panel, or restored via a typed `RestoreFile` proposal.

---

### 2. Text Editor & Universal Diff Engine

#### 2.1 Text Editor Architecture

The CYBOU Text Editor is a professional, multi-panel code and configuration editor designed specifically for spatial desktop workflows:

1. **Buffer & Editing Capabilities**:
   - Multi-file tabs within a single Editor Panel.
   - Syntax highlighting for JSON, YAML, TOML, Markdown, Rust, Python, Shell, Nginx config, and Systemd unit files.
   - Line numbers, active line highlighting, column/row indicators (`Ln 18, Col 23`).
   - Find, replace, regular expression search, and Go-to-Line (`Ctrl+G`).
   - Multi-level Undo / Redo history stack.
   - Auto-indentation and bracket matching.
   - Markdown split live preview.
   - Character encoding (UTF-8, ASCII) and line ending (LF vs CRLF) indicators and converters.
   - Dirty buffer indicator (`●`) and unsaved changes guard on close.
   - Large-file protection (streaming / read-only truncation for files > 5 MB).

2. **Autosave vs Privileged Authority Boundary**:
   - For `HostUserPath`, optional background autosave may be enabled.
   - For `SystemConfigPath`, autosave is **strictly forbidden**. The editor maintains an in-memory dirty buffer. When the operator clicks `[Save]` or presses `Ctrl+S`:
     1. The editor automatically generates a structured diff against the current on-disk content.
     2. A modal or linked Diff Panel opens displaying `[Review Diff]`.
     3. Clicking `[Request Save]` submits a typed `FileWrite` proposal to `Action1`.
     4. Upon operator authorization, `cybou-executord` performs an atomic write and reports verification.

3. **Multi-Panel Spatial Drag-and-Drop**:
   - Dragging a file icon from any Files Panel onto an Editor Panel opens the file in a new tab.
   - Dragging an active tab out of an Editor Panel onto the Canvas spawns an independent, detached Editor Panel with that file.

```text
+-------------------------------------------------------------+
| Editor · /etc/nginx/nginx.conf               [Diff] [Save]  |
+-------------------------------------------------------------+
|  1  server {                                                |
|  2      listen 443 ssl;                                     |
|  3      server_name vps-d0669a91.vps.ovh.net;               |
|  4      ssl_certificate /etc/letsencrypt/live/fullchain.pem;|
|  5      ...                                                 |
|  6  }                                                       |
+-------------------------------------------------------------+
| Ln 18, Col 23 · UTF-8 · LF · nginx · [Dirty ●]              |
+-------------------------------------------------------------+
```

#### 2.2 Universal Diff Panel

The Diff Panel is a standalone, first-class Canvas component that renders side-by-side and unified diffs across diverse source providers:

- **Sources**:
  - `Editor Buffer` vs `On-disk File`.
  - `Agent Proposed Patch` vs `Repository Workspace`.
  - `Package Upgrade Config (.dpkg-new)` vs `Live System Config`.
  - `Backup Snapshot File` vs `Current Working File`.
- **Controls**:
  - `[Accept All / Accept Chunk]`: applies modifications to the target.
  - `[Reject]`: discards proposed changes.
  - `[Open in Editor]`: transfers content to an active Editor Panel.
  - `[Request Action1 Save]`: initiates authorized privileged commit.

---

### 3. Files 2.0 (Multi-Panel File Management & Previews)

#### 3.1 Multi-Panel Spatial Topology

Unlike traditional web file managers that lock the user into a single folder view, CYBOU Desktop allows spawning multiple independent Files Panels simultaneously across the Canvas.

An operator can place `/home/cybou/projects` beside `/var/log` and `/etc/systemd/system`, dragging files directly across panels with immediate visual feedback:

```text
+------------------------------+     +------------------------------+
| Files · /home/cybou/projects |     | Files · /var/log/nginx       |
+------------------------------+     +------------------------------+
| [←] [↑] [⌂] [/home/cybou...] |     | [←] [↑] [⌂] [/var/log/ngi...] |
| Search: [                  ] |     | Search: [                  ] |
| ---------------------------- |     | ---------------------------- |
| 📁 CYBOU                     |     | 📄 access.log   (4.2 MB)     |
| 📁 demo-app                  | ===>| 📄 error.log    (128 KB)     |
| 📄 compose.yml               |     |                              |
+------------------------------+     +------------------------------+
```

#### 3.2 File Operations & Navigation

- **Navigation**: Back / Forward history, Up-to-Parent directory, Breadcrumb bar with direct jump, Path auto-complete.
- **Views**: Icon Grid View vs Detailed List View (columns: Name, Size, Type, Permissions, Modified Date).
- **Sorting**: By Name, Size, Modification Date, Extension (ascending / descending).
- **Core Operations**: Create Folder, Create File, Rename, Copy, Move, Duplicate, Delete, Move to Trash, Restore from Trash, Upload (with progress), Download.
- **Bookmarks & Quick Access**: Pinned locations (Home, Root, Projects, Logs, System Configs, Trash).
- **Permissions Inspector**: Visual modal to view and modify UNIX permissions (`chmod`, `chown`) when authorized.

#### 3.3 Integrated File Previews

Files can be previewed instantly without opening an editor:
- **Text / Code**: Syntax-highlighted read-only viewer.
- **Markdown**: Formatted HTML presentation.
- **Images**: High-resolution viewer with zoom and EXIF metadata.
- **PDF**: Multi-page reader.
- **Audio / Media**: Duration, bitrate, codec metadata viewer.
- **Archives (.tar.gz, .zip, .deb)**: Interactive archive directory tree viewer.

---

### 4. Mail & Personal Core Architecture

#### 4.1 Strict Separation of Frontend and Mail Credentials

CYBOU email integration MUST follow a strict zero-trust architecture regarding client-side credentials:

1. **Credential Isolation**: Raw IMAP/SMTP passwords, OAuth client secrets, and refresh tokens are **never transmitted to or stored within the browser DOM, WebAssembly runtime, or localStorage**.
2. **Backend Daemon**: All mail synchronization and transport is handled by a host-level systemd service communicating with external mail servers over TLS.
3. **Gateway Translation**: `cybou-web-gateway` proxies sanitized message lists, thread trees, and message bodies over authenticated HTTP endpoints (`/api/v1/mail/...`).
4. **Client Capabilities**: Multiple accounts, folder management (Inbox, Starred, Sent, Drafts, Archive, Trash, Spam), message threading, search, rich-text / plain-text compose, attachments handling, and desktop notifications.

```text
[Browser Canvas: Mail Panel]
          │ (Session Cookie)
          ▼
[cybou-web-gateway]
          │ (D-Bus / Unix Socket)
          ▼
[cybou-maild: IMAP / SMTP / OAuth2 Engine] (Credentials in /etc/cybou/credentials)
          │ (TLS)
          ▼
[External Mail Provider]
```

#### 4.2 Cognitive Integration & Derived Relations

When an incoming email contains actionable system relevance (e.g. *"TLS Certificate for domain.com expires in 7 days"* or *"Security advisory for libssl3"*), CYBOU Mind identifies the entity and displays a **suggested derived relation edge** connecting the Mail Panel to the corresponding **Certificate**, **Package Update**, or **Systemd Service Panel**.

Derived relations remain strictly advisory; they do not automatically execute actions without operator confirmation.

#### 4.3 Calendar, Notes & Desktop Utilities

- **Calendar Panel**: Supports CalDAV synchronization and local events. Differentiates personal calendar entries from scheduled system operations (`personal event ≠ system maintenance`).
- **Notes Panel**: Fast, local Markdown note-taking panel. Notes are user-owned documents, strictly isolated from Mind Epistemic beliefs and Self narration memory.
- **Desktop Utilities**: Lightweight Calculator, PDF Viewer, and Image Viewer panels.
- **Rejection of Web Browser Panel**: An embedded web browser panel (rendering arbitrary internet web pages via iframe) is **deliberately rejected**. Embedding external websites inside the Canvas introduces Content Security Policy (CSP) degradation, clickjacking risks, cross-origin credential leakage, and phantom session spoofing. External links open in standard browser tabs.

---

### 5. System Control Center & Dedicated System Panels

#### 5.1 Settings / Control Center

The Settings Panel acts as the central control plane, structured with an unambiguous division between **System OS Settings** and **CYBOU Cognitive Settings**:

```text
SETTINGS
├── SYSTEM (Debian 13 OS Infrastructure)
│   ├── Network & Hostname      = IP configuration, DNS, gateway, hostname
│   ├── Storage & Mounts        = Disk partitions, mount options, fstab
│   ├── Users & Accounts        = UNIX users, groups, sudo privileges
│   ├── SSH & Remote Access     = Authorized keys, SSH daemon configuration
│   ├── Date, Time & NTP        = Timezone, NTP server synchronization
│   └── Packages & Repositories = APT sources, repository signing keys
│
└── CYBOU (Cognitive Mind & Spatial Desktop)
    ├── General & Appearance    = Dark/Light/System theme, density, canvas grid
    ├── Spatial Canvas Engine   = Snap distance, relations filter, semantic zoom LOD
    ├── Cognitive Automation    = Remediation initiative policies, approval thresholds
    ├── Autonomous Agents       = Capsule concurrency limits, memory/CPU quotas, spend budgets
    ├── Model Brokerage         = Provider endpoints, model groups, token pricing
    ├── Epistemics & Privacy    = Sensitivity classifications, disclosure rules
    └── Backups & Recovery      = Automated snapshot intervals, retention policies
```

Any individual settings category can be detached by the operator into an independent standalone Panel.

#### 5.2 Storage & Disks vs Files

- **Conceptual Invariant**: `Files` manages directory contents; `Storage` manages block devices, partitions, and filesystem integrity.
- **Storage Panel**: Displays physical/virtual block devices (`vda`, `nvme0n1`), partitions, filesystems (`ext4`, `btrfs`), mount points, total/used capacity, and inode utilization.
- **Hardware Telemetry Integrity**: Displays SMART health, disk temperature, and NVMe wear on bare-metal systems. On virtualized VPS environments where hardware probes are unavailable, it states *"Hardware health: Not observable on this virtual host"* rather than rendering mock data.
- **Destructive Operation Safety**: Destructive actions (Partition, Format, Wipe) are classified as **Critical Risk**. They require explicit interactive confirmation (typing the exact device node name, e.g. `sdb1`) followed by a typed Action1 authorization permit. Automatic standing policies cannot authorize filesystem destruction.
- **Disk Usage Panel**: Interactive visual treemap and directory size hierarchy for rapid disk space diagnosis.

#### 5.3 System Monitor, Services, Logs, and Network

- **System Monitor**: Real-time CPU, RAM, Swap, Disk I/O, Network bandwidth, and Linux Pressure Stall Information (PSI: CPU, Memory, I/O pressure). Includes a process tree differentiating **Host System Processes**, **Agent Capsules**, and **Systemd Daemons**.
- **Services Panel**: Live systemd unit manager showing Active, Inactive, Failed, and Masked units. Inspecting a service opens a detailed panel with dependencies (`Requires`, `Wants`, `After`), cgroup resource consumption, and direct link to unit logs.
- **Logs Panel**: Interactive Journald stream with real-time tailing, unit filtering (`-u nginx`), severity filtering (`emerg` to `debug`), full-text search, and time range selection.
- **Network Panel**: Real-time interface status, IP addresses, MTU, routing table, DNS resolvers, and **Listening Ports** (`:22 sshd`, `:80 nginx`, `:443 nginx`, `:8787 cybou-web-gateway`). Ports can be dragged onto the Canvas to spawn the corresponding service or log panel.
- **Packages & Updates Panel**: APT package inspector, available upgrade list, security update prioritization, changelog diff viewer, and Action1-governed upgrade execution.

---

### 6. Dual Terminal Architecture

CYBOU Desktop provides two distinct terminal environments, preserving both determinism and full interactive power:

1. **CYBOU Safe Shell**:
   - Bounded, deterministic, command-oriented shell.
   - Operates within Zone 3 `DemoReadOnly` / `OperatorSandbox` boundaries.
   - Ideal for structured diagnostics, remote low-bandwidth sessions, and verifiable tool invocation.
2. **Linux PTY Terminal**:
   - Real, full-featured Linux Pseudo-Terminal (PTY) session.
   - Runs as the authenticated Linux user (`uid = 1000+`).
   - Supports ANSI escape codes, full curses/ncurses TUIs (`htop`, `vim`, `tmux`), shell pipes, signals (`SIGINT`, `SIGTSTP`), and job control.
   - Administrative commands requiring root privilege must go through standard `sudo` (authenticated via PAM/polkit) or typed Action1 operations.

---

### 7. Universal Clipboard, Background Transfers, and Notifications HUD

- **Universal Clipboard**: Seamless cross-panel copy/paste and drag-and-drop support for file paths, code snippets, structured JSON, attachments, and spatial coordinates.
- **Transfers Panel**: Dedicated background task panel tracking non-blocking file uploads and downloads with progress percentage, transfer rate, ETA, and cancellation controls.
- **Notifications Center**: Unified system notification feed (remediation outcomes, security alerts, agent task completions, package updates). Clicking any notification smoothly animates the camera viewport to the relevant Panel on the Canvas.
- **HUD / Floating Panels Mode**: Special overlay presentation mode allowing `Notifications`, `Tasks / Transfers`, and `Ask CYBOU` to remain pinned to the viewport during spatial canvas panning and zooming.

---

## 8. Implementation Phasing & Delivery Roadmap

The CYBOU Core Desktop Pack will be delivered across 10 sequential implementation milestones:

```text
CP0: LocationRef Primitives & Workspace Authority Domain
CP1: Files 2.0 Multi-Panel File Manager & Preview Renderers
CP2: Text Editor Engine & Universal Diff Panel (with Action1 Save Gating)
CP3: System Monitor, PSI Pressure & Process Tree Explorer
CP4: Services Manager & Interactive Journald Logs Viewer
CP5: Storage, Disks & Interactive Disk Usage Treemap
CP6: Network Interfaces, Listening Ports & Package Manager Updates
CP7: Interactive Linux PTY Terminal & Transfers Panel
CP8: Settings / Universal Control Center & Notifications HUD
CP9: Personal Core (Mail Client, Calendar, Notes, PDF/Image Viewers)
CP10: Unified Cognitive Composition (Agent Workspace + Diff + Live Canvas Integration)
```

---

## Non-Cognitive Presentation Invariants

1. **Spatial Representation ≠ Operating System Process**: Closing or hiding a Panel on the Canvas never terminates a systemd service, stops a running agent capsule, or closes an open network socket.
2. **Drag-and-Drop ≠ Authorization**: Dragging a configuration file or network port into a panel provides contextual input; it never grants security authority or bypasses Action1 policy.
3. **Editor Buffer ≠ Disk Reality**: An unsaved buffer in the Text Editor is strictly presentation state. Privileged files are only modified upon atomic execution of an authorized Action1 `FileWrite` permit.
4. **Browser State ≠ Mind Epistemology**: LocalStorage and DOM layout caches are disposable client-side view state. The Mind's Epistemic beliefs, Journal history, and Self narration are owned exclusively by host-level daemons.
5. **Secret Isolation Invariant**: Raw credentials for external services (IMAP, SMTP, OAuth, API keys) must never enter the browser DOM or WebAssembly memory; they reside exclusively in host-side secure credential stores.
