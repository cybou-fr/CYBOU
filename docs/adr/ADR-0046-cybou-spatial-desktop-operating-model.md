<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0046: CYBOU Spatial Desktop Operating Model: Full Lifecycle, SubjectRef Primitives, System Inspection, Universal Search, Epistemic State Machine, Background Operations, and Resilience Acceptance Gates (SD1–SD15)

## Status
Proposed

## Context
Following [ADR-0037](ADR-0037-web-first-presence-and-desktop.md) (Web-First Presence), [ADR-0040](ADR-0040-spatial-card-desktop-and-bounded-body-capabilities.md) (Spatial Card Desktop), [ADR-0044](ADR-0044-cybou-spatial-desktop-architecture.md) (Spatial Desktop Architecture), and [ADR-0045](ADR-0045-cybou-core-desktop-pack-and-workspace-primitives.md) (Core Desktop Pack), the CYBOU Living Canvas has evolved from individual presentation cards into a cohesive spatial operating system running in the browser.

However, moving from individual panels to a complete operating model requires formalizing the entire operational lifecycle: from session authentication to multi-day uninterrupted workflows, background task survivability, clipboard and drag/drop typing, cross-panel inspection, epistemic state representation, multi-user isolation, performance virtualization, and strict hostile-content security boundaries.

CYBOU is neither a traditional desktop environment (like GNOME or KDE ported to HTML) nor a generic web dashboard. It is a spatial living model of the computer in which files, tools, communications, system daemons, autonomous agents, and cognitive Mind faculties coexist on a single infinite 2D plane.

## Decision

### 1. The CYBOU Desktop Operating Philosophy
1. **Core UX Invariant**:
   > *"In CYBOU, the human does not navigate between pages. They open objects, tools, and relations in space."*
2. **Structural Model**:
   ```text
                        CYBOU SPATIAL DESKTOP
   
                            Infinite Canvas
                                 │
           ┌─────────────────────┼─────────────────────┐
           │                     │                     │
         Panels                Clusters             Relations
           │                     │                     │
           └─────────────────────┼─────────────────────┘
                                 │
                           CYBOU Gateway
                                 │
              ┌──────────────────┼──────────────────┐
              │                  │                  │
             Mind               Body              Apps
              │                  │                  │
        cognition/state    Linux capabilities   mail/calendar/...
   ```

### 2. Panel Lifecycle & Decoupled State Invariant
1. **Presentation Decoupling**:
   - Panels are views into underlying systems; they do not own the underlying system entities.
   - **Lifecycle Invariant**: Closing a `ServicePanel` never stops the systemd service. Closing an `AgentPanel` never terminates the running agent capsule. Closing a `MailPanel` never disconnects the mail account.
2. **Panel State Structure**:
   ```rust
   pub struct PanelInstance {
       pub panel_id: String,
       pub panel_type: PanelKind,
       pub subject: Option<SubjectRef>,
       pub presentation: PresentationState,
       pub geometry: CardGeometry,
       pub parent_cluster: Option<String>,
       pub deck_id: Option<String>,
       pub created_from: Option<String>,
   }
   ```

### 3. Panel Registry & Capability Specification
Every panel type must register a static `PanelSpec` defining its structural requirements and interaction boundaries:
```rust
pub struct PanelSpec {
    pub kind: PanelKind,
    pub title: &'static str,
    pub icon: &'static str,
    pub multiplicity: Multiplicity, // Singleton | MultiInstance
    pub subject_type: Option<SubjectType>,
    pub supported_views: Vec<ViewMode>,
    pub spawn_rules: SpawnPolicy,
    pub accepts_drop: Vec<SubjectType>,
    pub context_actions: Vec<ActionDescriptor>,
    pub capability_requirements: Vec<CapabilityRequirement>,
}
```

### 4. Typed Subjects (`SubjectRef`)
All references across panels, relations, drag/drop, and inspector must use strongly-typed `SubjectRef` variants rather than raw strings:
```rust
pub enum SubjectRef {
    Service { name: String, node_id: Option<String> },
    Process { pid: u32, name: String },
    File { location: LocationRef },
    Agent { capsule_id: String, agent_type: String },
    MailMessage { account_id: String, folder: String, message_id: String },
    CalendarEvent { account_id: String, event_id: String },
    Certificate { domain: String, thumbprint: String },
    Filesystem { mount_point: String, fs_type: String },
    Package { name: String, installed_version: Option<String> },
    Anchor { anchor_id: String, label: String },
}
```

### 5. Universal Floating Inspector (`InspectorPanel`)
A contextual floating panel that inspects any active `SubjectRef`, rendering:
- Entity metadata, canonical state, and operational status.
- Connected spatial and causal relations (e.g. `:80`, `:443`, associated certificates, log streams).
- Active CYBOU Mind findings and autonomous remediation status.
- Context-appropriate typed actions (`Open Logs`, `Inspect Processes`, `Restart`, `Watch`, `Ask CYBOU`).

### 6. Typed Drag & Drop
- **Safety Invariant**: Drag and drop transfers structured intent hints; **drop never constitutes an authorized privileged action**.
```rust
pub struct DragPayload {
    pub subject: SubjectRef,
    pub intent_hint: DragIntent,
}
```
- Panels implement `accepts_drop(SubjectType)`. Examples:
  - `FileRef` → `EditorPanel` = Open file in a new editor tab.
  - `LocationRef` → `AgentPanel` = Propose directory as agent workspace.
  - `ServiceRef` → `LogsPanel` = Create service log filter.
  - `AttachmentRef` → `FileManager` = Save attachment to selected folder.

### 7. Multi-Format Desktop Clipboard
The desktop clipboard distinguishes between structured formats:
- Plain Text.
- `FileRef` / `LocationRef`.
- `MailAttachmentRef`.
- `PanelLink` / `DeepLink`.
- `SubjectRef` (pasting into Notes creates a rich interactive link; pasting into Ask CYBOU inserts a contextual token; pasting on Canvas creates a typed panel).

### 8. Deep Links & URI Scheme
Every meaningful entity, panel composition, or spatial anchor supports deep linking via `cybou://` and web hash fragments (`/#/service/nginx.service`, `/#/file/...`, `/#/agent/...`, `/#/anchor/production`).

### 9. Universal Search & Intent-Classified Command Language (Ctrl+K)
Search is a tri-layer indexing service:
1. **Desktop Index**: Panels, anchors, system services, processes, packages, settings, files by name, mail headers, contacts, calendar, agents, action history.
2. **Content Index**: Text files, cached mail bodies, personal notes, bounded log buffers.
3. **Semantic Query Layer**: Deterministic intent parser classifying queries into:
   - `OBJECT` (direct focus or open).
   - `ACTION` (execute proposal).
   - `VIEW` (open specific filtered projection).
   - `QUESTION` (route to Ask CYBOU / Mind).

### 10. File Associations & Spatial Dialogs
- `MimeHandlerRegistry` maps MIME types and file extensions to viewer/editor components (`.md` → Editor/Markdown Preview, `.pdf` → PDF Viewer, `.png/.jpg` → Image Viewer, `.zip/.tar` → Archive Panel).
- System dialogs (`Save As`, `Open File`, `Choose Directory`, `Export`) are non-blocking spatial ephemeral cards anchored near the originating tool.

### 11. Text Editor Project Modes, Tri-Zone Autosave & Undo Boundaries
- **Project/Workspace Mode**: Supports multi-file decks, outline navigation, search/replace, diff inspection, and live preview.
- **Tri-Zone Autosave Policy**:
  1. *Personal User Files* (`/home/user/...`): Autosave enabled.
  2. *Project Repositories*: Configurable explicit save or autosave.
  3. *System Configuration* (`/etc/...`): Direct autosave prohibited; edits remain drafts until reviewed in Diff Viewer and approved via Action1 FileWrite proposal.
- **Tri-Zone Undo Scopes**:
  - *Editor Undo*: Buffer text history.
  - *Canvas Undo*: Spatial presentation and layout history.
  - *System Undo*: Never tied to `Ctrl+Z`; system rollback is a typed, explicit Action1 operation.

### 12. Mail Synchronization, Local Store & Offline Queuing
- **Bounded Local Store**: Account metadata, folders, message headers, cached message bodies (30-day default retention), attachment metadata.
- **Secret Isolation**: OAuth refresh tokens and SMTP passwords reside exclusively in the server-side Secrets Center; the browser receives only bounded mail content DTOs.
- **Offline Semantics**: Cached messages remain readable offline; outgoing emails are queued locally with explicit status indicators (`Offline · Queued`). Queued mail send is explicitly distinguished from queued privileged OS mutations (which are prohibited).

### 13. Unified Accounts, Calendar & Contacts
- **Accounts**: Centralized configuration for OAuth providers (Google, Nextcloud, Microsoft).
- **Calendar Event Typing**: Visual and semantic distinction between Personal Events, Maintenance Windows, Backup Runs, Certificate Expiration Deadlines, and Agent Scheduled Tasks. Invariant: *A calendar event is informational and does not constitute a scheduled OS operation*.
- **Contacts**: Unified person objects referenced by Mail, sharing policies, and activity logs.
- **Spatial Sticky Notes**: Fast spatial notes anchored near clusters. Invariant: *User notes are personal documentation and are never interpreted as system policies*.

### 14. Notification Attention Matrix & Incident Aggregation
- **Importance Levels**: `Passive`, `Important`, `ActionRequired`, `Critical`, `Progress`, `Completed`.
- **Incident Episode Aggregation**: Prevents notification fatigue during self-healing cycles by bundling related alerts (e.g. failure → action proposal → remediation execution → health verified) into a single aggregated incident notification with an interactive timeline.
- **Do Not Disturb (DND)**: Suppresses non-critical notifications while ensuring critical safety events (disk full, security violations, backup failures > 7 days) remain visible.

### 15. Background Operations (`Operation1`) & Honest Progress
- Centralized tracking for background operations (package downloads, backups, restore procedures, indexing, agent tasks, mail sync).
- **Survivability Invariant**: Browser disconnection or page refresh does not interrupt running backend operations; reconnected clients restore active progress.
- **Honest Progress**: When exact percentage completion is unavailable from the underlying tool, progress is explicitly rendered as *indeterminate* rather than simulated.

### 16. Distributed Settings & Configuration Diff Engine
- Settings UI acts as a unified aggregator; individual domains (Telemetry, Agents, Mail, Appearance, Remote Access) retain sole ownership of their respective configuration schemas.
- **Config Classes**:
  1. *Instant Local*: Theme, zoom level, snap guides, minimap.
  2. *Ordinary User Config*: Mail signatures, notification filters, language.
  3. *Governed System Config*: SSH, network interfaces, firewall rules, package update policies (rendered with Before/After Diff and submitted via Action1).

### 17. Secrets Center & Zero-Trust Browser Projection
- Server-side repository for sensitive credentials (API tokens, OAuth secrets, database passwords, private keys).
- The browser interface displays secret metadata and validation status (`Configured ✓`, `Last Rotated 3d ago`) but never retrieves raw secrets.
- Replacement inputs submit directly to the server and clear memory immediately; raw secret copying from UI is prohibited.

### 18. Authentication, Lock Screen, Session Center & Multi-User Separation
- Dedicated remote login interface using Linux PAM credentials and session tokens.
- **Lock Screen**: Obscures canvas presentation and requires session re-authentication.
- **Session Center**: Inspects active web sessions across devices with remote revocation capabilities.
- **Multi-User Isolation**: System state is shared and canonical; desktop layouts, personal mail, notes, and local preferences are strictly user-scoped.

### 19. Multi-Tier Desktop Persistence
1. *Local Device Preferences*: Pan/zoom position, temporary tool placement.
2. *User Desktop Layout*: Named workspace layouts, clusters, anchors, dock items.
3. *Canonical System State*: Server-side single source of truth, never derived from browser storage.

### 20. Network Resilience & Latency-Aware Asynchrony
- **Refresh (F5) Resilience**: Full session revalidation, canonical snapshot reload, active operations recovery, and layout restoration without state loss.
- **Stale State Projection**: Disconnected clients display prominent `Stale / Disconnected` banners with timestamp of last known update; optimistic success on privileged actions is prohibited.
- **Local Latency Masking**: Pan, zoom, window drag, menu interactions, and editor keystrokes respond immediately in local WASM state; remote actions indicate `Request Submitted` until confirmed by backend events.

### 21. Large-File Protection, Chunked Transfers & Archive Inspection
- **Large File Handling**: Range reads, streaming tails, and pagination prevent loading multi-hundred-megabyte files into browser memory.
- **Resumable Transfers**: Uploads and downloads utilize chunked streams tied to `Operation1` tasks.
- **Archive Browser**: Safe virtual file exploration for `.zip`, `.tar.gz`, `.tar.zst` with strict traversal path validation preventing Zip-Slip vulnerabilities.

### 22. Input, Localization & Accessibility Outline
- **Keyboard Shortcut Hierarchy**: `Terminal` > `TextInput` > `Panel` > `Canvas` > `Global`.
- **International Input**: Full support for IME composition, dead keys, Cyrillic, and CJK text without global shortcut interception.
- **Date/Time Display**: Canonical UTC timestamps with user toggle for Local, Server, or UTC representation.
- **Canvas Outline Panel**: Hierarchical tree navigation of all clusters and panels, providing an accessible non-spatial representation for screen readers and structured exploration.

### 23. Performance Virtualization & Subscription Budgets
- **Level of Detail (LOD)**: Off-screen and distant panels unmount expensive child components and render lightweight proxy summaries.
- **Subscription Multiplexing**: `DesktopSubscriptionManager` aggregates event streams over a single multiplexed SSE channel, eliminating per-card HTTP polling.
- **Frontend Error Boundaries**: Isolated rendering failures in one tool card display a local recovery button without crashing the overall Living Canvas.

### 24. Upgrade Negotiation & Power Lifecycle
- **Schema Negotiation**: Client and server exchange version manifests; frontend prompts for graceful reload when backend upgrades occur.
- **Recovery Environment**: If cognitive daemons are degraded, a minimal recovery canvas provides Safe Shell, system service statuses, and raw logs.
- **Power Operations**: Graceful UI feedback during system reboot or shutdown, featuring host availability polling and automated reconnection.

### 25. Extensions, Diagnostics & Epistemic State Machine
- **Extension Safety**: Desktop extensions are compiled-in Rust modules ensuring memory and type safety.
- **Diagnostics Bundle**: Generates sanitized system diagnostics with explicit redaction previews excluding secrets, emails, and private files.
- **Epistemic State Primitives**: User interfaces represent state through explicit variants rather than bare `Option<T>`:
  ```rust
  pub enum EpistemicState<T> {
      Known(T),
      Unknown { reason: String },
      Unavailable { reason: String },
      Stale { data: T, last_observed: String },
      Forbidden { required_scope: String },
      NotConfigured,
      Empty,
  }
  ```

### 26. Privacy Dashboard, Unified History & Spatial Reporting
- **Privacy Dashboard**: Displays real-time metrics on external model deliveries, third-party provider calls, and explicitly withheld data categories.
- **Triad of History**:
  1. *Recent Activity*: High-level human-readable narrative.
  2. *Journal*: Canonical, immutable causal event chain.
  3. *Logs*: Bounded raw operational streams.
- **Spatial Reporting**: Generates structured Markdown or PDF summary reports from any spatial cluster.

### 27. Canvas Composition & Multi-Select
- Multi-selection bounding box (`Shift + Drag`) supporting batch movement, cluster creation, alignment, and distribution.
- **Ephemeral Investigation Cleanup**: Closes temporary cards spawned during an incident (`created_from`) while preserving permanent workspace anchors.

### 28. Security Levels & Untrusted Content Isolation
Three Content Security tiers prevent XSS and code execution:
1. *Tier 1: Trusted CYBOU UI* (native Rust Leptos components).
2. *Tier 2: Sanitized Content* (Markdown, clean text, filtered log lines).
3. *Tier 3: Sandboxed External Content* (HTML emails, untrusted agent outputs, external previews rendered in isolated frames with strict CSP).

### 29. Spatial Desktop Acceptance Gates (SD1–SD15)
To ensure system stability, every desktop release must satisfy fifteen formal gates:
- **SD1**: Canvas layout and open tools survive browser page refresh (F5).
- **SD2**: Canvas maintains smooth 60 FPS navigation with 100+ active panels.
- **SD3**: Off-screen panels do not poll the backend independently.
- **SD4**: Unknown or unavailable data never renders as an empty state.
- **SD5**: Closing a panel never modifies or terminates the underlying system entity.
- **SD6**: Drag and drop gestures never execute privileged mutations.
- **SD7**: Saving privileged system files (`/etc/...`) always requires Action1 authorization.
- **SD8**: Disconnected browsers never claim fresh operational state.
- **SD9**: Running background operations survive browser closure and reconnect seamlessly.
- **SD10**: Authentication secrets, API keys, and OAuth tokens never leak into frontend memory.
- **SD11**: Non-spatial Canvas Outline allows full screen-reader navigation.
- **SD12**: Semantic zoom level transitions never alter canonical system state.
- **SD13**: WebSocket/SSE reconnect correctly resumes event streams without duplicate events.
- **SD14**: Malicious email HTML, agent output, or file previews cannot execute script in the canvas origin.
- **SD15**: A single frontend build target seamlessly supports browser tabs, installed PWAs, and local kiosk shells.

## Consequences

### Positive
- Formalizes the complete operational model of CYBOU Spatial Desktop from login to daily workflows.
- Replaces raw strings and unstructured payloads with strongly typed `SubjectRef`, `LocationRef`, and `DragPayload` primitives.
- Enforces zero-trust boundaries: secrets remain server-side, privileged mutations require Action1 proposals, and untrusted contents are isolated.
- Establishes concrete acceptance gates (`SD1`–`SD15`) guaranteeing long-term architectural integrity.

### Negative
- Requires maintaining typed serializers and parsers across multiple domain subjects.
- Increased frontend state machine complexity to accommodate Level-of-Detail virtualization and epistemic states.
