<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# CYBOU Desktop (Living Canvas) Web UI Integration Architecture

## Purpose and status

This document is the implementation blueprint for [ADR-0037](adr/ADR-0037-web-first-presence-and-desktop.md)
and [ADR-0040](adr/ADR-0040-spatial-card-desktop-and-bounded-body-capabilities.md). It describes the
target architecture and operational boundaries of **CYBOU Desktop**. [Current State](CURRENT_STATE.md)
remains authoritative for implemented capabilities.

The objective is to make the Rust/WASM CYBOU Desktop the complete, extensible spatial environment
for CYBOU while preserving the ownership, continuity, failure, privacy, and authorization properties
already proved by Mind, and introducing bounded Body capability surfaces such as CYBOU Shell.

## Documentation analysis

The project documentation establishes constraints that the new surface must carry forward.

| Existing contract | Integration consequence |
|---|---|
| Body, Mind, and Presence are separate | browser and gateway remain replaceable Presence components |
| durable before visible | web events follow committed owner projections; optimistic UI is labelled pending |
| one canonical Journal writer | gateway never reads or writes Journal directly |
| process-isolated owners | HTTP routing cannot collapse owners into a web monolith |
| Presence is an aggregator, not an owner | gateway may translate a snapshot but cannot reconstruct domain truth |
| attention is not biography | canvas layout/cache is disposable; accepted history remains in Mind |
| degraded capability is explicit | unavailable sections degrade independently and retain cause/freshness |
| bounded RPC and unknown outcome | HTTP/WebSocket timeouts cannot be converted into empty success |
| context delivery is consumer-specific | every local/remote browser session is a named consumer |
| UI is not authorization | frontend affordances never replace backend capability/policy gates |
| action dispatch is not outcome | Living Canvas shows proposed, authorized, attempted, observed, and terminal states separately |
| retention and erasure reach derived storage | browser cache, IndexedDB, exports, screenshots, and service workers are governed surfaces |

Documentation in `mind/UI.md` identifies
discoverability, keyboard access, unavailable state, short-window behavior, shell recreation, and
stable Presence ownership as product requirements. Their implementation mechanism is not carried
forward.

## Target topology

```text
  PERSON
  
  --------------|--------------
  ▼                             ▼
  local Chromium/Ozone              remote Chromium
  isolated appliance UI             HTTPS browser/PWA
  
  - |--------------
  ▼
  cybou-web-gateway
  auth / session / schema / CSP
  CSRF / cursor / rate / origin
  
  local session D-Bus
  ▼
  cybou-presenced
  
  ----------|-------------+--------------|-----------
  ▼          ▼             ▼              ▼           ▼
  healthd   lifecycled    intentiond     contextd     other owners
  
  Event1
  ▼
  eventd
  ▼
  Journal
```

The gateway is local to the managed Cybou node even when the browser is remote. A later multi-node
gateway requires a separate authority and routing decision; it is not implied by this design.

## Repository and package target

Recommended target layout under the ADR-0038 Cargo workspace:

```text
crates/
  living-canvas/         Rust/WASM Living Canvas frontend
    src/
      app/               composition and routing
      canvas/            spatial objects, selection, viewport
      features/          commitments, evidence, health, lifecycle
      platform/          session/API/cache abstractions
      design-system/     tokens, typography, components, icons
  web-contracts/         shared HTTP/event DTOs and schema generation
  web-gateway/           Axum browser boundary and zbus Presence adapters
  desktop-shell/         Chromium/session policy launcher

modules/
  cybou-web-gateway.service
  living-canvas.service

packages/
  cybou-web-ui/
  cybou-web-gateway/
  cybou-desktop-shell/
```

The production Rust/WASM shell lives in `crates/living-canvas`. The earlier React `living-canvas/`
prototype has been removed; its visual and interaction evidence is preserved in the design
references under `docs/`. Its production browser adapter is
`GatewayMindClient`, which reads the typed session and atomic snapshot from the same-origin gateway;
`MockMindClient` is retained for deterministic contract tests. The first read-only Presence
connection and same-origin static delivery are implemented; subscriptions and mutations remain
future gates.

## Frontend architecture

### Runtime adapter

The Rust frontend depends on one trait:

```text
MindClient
  session()
  snapshot()
  object(id)
  capabilities()
  subscribe(cursor)
  propose(command, input)
  confirm(challenge, proof)
  cancel(operationId)
```

Implementations:

```text
MockMindClient     deterministic fixtures and visual tests
GatewayMindClient  local desktop and hosted web
```

There is deliberately no `DbusMindClient` in the browser.

### State model

Frontend state is divided into:

```text
canonical projection cache   server-versioned, replaceable
session state                authentication, grants, expiry, mode
canvas view state            pan, zoom, selection, arrangement
ephemeral interaction state  menus, drafts, pending previews
```

Canvas positions are preferences, not biography. If persisted through Mind later, they require an
explicit owner and protocol. Until then, they remain browser/profile state with bounded retention.

### Rendering rules

- Every object renders `known`, freshness, capability, and source state.
- Missing data does not become zero, false, or an empty list unless `known=true`.
- Pending mutations render separately from accepted projections.
- A reconnect replaces the projection atomically rather than merging unknown partial state.
- Remote mode never hides its external-boundary status.
- Held-back context is explainable to the person without disclosing the held-back content to an
  untrusted consumer.

### Responsive model

The same frontend supports:

- desktop shell: spatial canvas is primary;
- browser desktop: spatial canvas with normal browser chrome outside the app;
- tablet/small window: focused object plus overview/minimap;
- narrow/mobile browser: object stream and explicit canvas overview, not a compressed node graph.

Responsive behavior changes layout, not domain capability.

### CYBOU Desktop Spatial Card Model (vNext)

In accordance with ADR-0040, CYBOU Desktop transitions from hardcoded fixed panels to an extensible
spatial **Card** architecture:

```text
Debian 13       = Body (host execution, kernel, storage)
CYBOU Mind      = continuity + cognition + governance (canonical owner)
CYBOU Desktop   = Presence (spatial interactive surface)

Card            = primary interactive surface
Deck            = presentation composition (tabs), not identity
Relationship    = semantic system causality, not physical proximity
Arrangement     = deterministic spatial presentation, not cognition
Desktop Map     = spatial navigation and cluster overview
Ctrl+K          = Desktop command palette (Desktop command ≠ Body command)
CYBOU Shell     = bounded Body capability exploration (typed capability, not arbitrary execution)
Desktop state   ≠ biography (DOM/localStorage ≠ truth)
Public preview  = no Shell capability (strict boundary)
```

#### Generic Card Model

Every visible surface implements `CardInstance`:
- `CardId`: Stable identifier (System cards: `Identity`, `Session`, `Capabilities`, `Journal`, `Lifecycle`, `Commitments`, `SelfModel`, `Attention`, `Beliefs`, `Perception`, `Context`; Tool cards: `Shell(u32)`).
- `CardGeometry`: Spatial offset `(x, y)`, mutable dimensions `(width, height)`, and stacking order `z`.
- `CardPresentation`: Display mode flags (`collapsed: bool`, `pinned: bool`).
- `CardSpec`: Static contract defining `kind` (`System`, `Tool`, `Ephemeral`), capabilities (`movable`, `resizable`, `collapsible`, `closable`, `deckable`), and size constraints (`default_size`, `min_size`, `max_size`).

#### Layout Schema v9, Self-Healing Normalization and Migration

Layout persistence uses schema version 9 (`cybou.desktop.layout.v9`):
1. Loads `cybou.desktop.layout.v9` if present in browser `localStorage`.
2. Falls back to legacy schema v8 (`cybou.living-canvas.layout.v8`), migrating all fixed point positions into full `CardGeometry` with default spec dimensions, uncollapsed, unpinned presentation.
3. Transparently runs `validate_and_normalize()` on boot:
   - **System Cards Guarantee**: Instantiates defaults if any of the 11 Mind system cards are missing from storage.
   - **Bounds & Anchors Clamping**: Clamps dimensions to `[min_size, max_size]` and bounds offsets to reachable coordinates.
   - **Deck Resolution**: Deduplicates cards, dissolves single-card or corrupt decks, and enforces multi-deck exclusivity.
   - **Monotonic Z-Ordering**: Re-indexes z-order monotonically starting from 1.
4. Commits verified state without breaking user coordinates or disrupting active sessions.

#### Spatial Dynamics, Compositor Invariants (L1–L15), and Invariant-Safe Decks

- **Compositor Invariants (L1–L15)**:
  - **L1–L4 (Deck Containment & Exclusivity)**: Cards in a Deck share the Deck's bounding box and z-index; switching tabs mutates only `active_card` without altering geometry. A card belongs to at most one Deck at any time.
  - **L5–L7 (Pinned Obstacles & Collision Avoidance)**: Pinned cards act as immoveable obstacles during auto-arrangement. Candidate placement via `PlacementResolver` scans spiral/grid offsets to prevent overlapping unpinned and pinned surfaces.
  - **L8–L10 (Multi-Deck Safety & Deterministic Geometry)**: A deck must contain at least 2 distinct cards. Decks cannot be nested within other decks.
  - **L11–L13 (Idempotent Transforms & Non-Destructive Focus)**: Layout arrangements are deterministic pure functions. Focus mode expands the target card to fill the viewport without mutating persisted canvas coordinates.
  - **L14–L15 (Unified Monotonic Z-Index & Presentation Layer)**: Z-index values form a strict monotonically increasing sequence across both cards and decks.
- **Magnetic Snap Guides (`compute_snap`)**: Dragging or resizing cards automatically snaps candidate boundaries against other desktop items within an 8px threshold, rendering real-time cyan alignment guides (`SnapGuide::Vertical`, `SnapGuide::Horizontal`).
- **Spatial Viewport Scaling ("Fit All")**: `DesktopLayout::fit_to_viewport` calculates the global bounding box of all active surfaces and centers the viewport with optimal zoom (0.4–1.2) via `Ctrl+0`, canvas double-click, floating toolbar, or command palette.
- **Interactive Minimap Navigation**: Mini-nodes on the desktop minimap provide pan-to-card and pan-to-deck viewport centering on click.
- **Accessible Keyboard Spatial Navigation**: `Alt+Arrow` moves cards while leaving plain Arrow keys free for internal controls, text editing, and terminal history. Pointer resizing and `Alt+Shift+Arrow` keyboard resize constrained to `[min_size, max_size]` with dynamic edge anchor recalculation for relationship vectors.
- **Collapse / Expand**: Single-line summary pill toggle to reclaim canvas space while maintaining presence.
- **Invariant-Safe Decks (Tabs)**: Grouping of cards governed by typed `DeckError` rules (minimum 2 distinct deckable cards, no duplicate or cross-deck assignments, valid `active_card` membership). Equipped with WAI-ARIA `role="tablist"` and keyboard navigation (`ArrowLeft`/`ArrowRight`/`Home`/`End`).
- **Resource Lifecycle Management**: External reactive handles (e.g. SSE `EventSource` in `JournalFeedCard`) are explicitly managed and closed on teardown to eliminate socket leaks.

#### Bounded Body Capability: CYBOU Shell

CYBOU Shell is an isolated, unprivileged capability surface to the Debian 13 host:
- Builtins strictly limited to ADR-0040 DemoReadOnly safe utilities: `pwd`, `cd`, `ls`, `cat`, `echo`, `whoami`, `uname`, `stat`, `head`, `tail`, `grep`, `clear`, `help`.
- No arbitrary execution, fork/exec, pipes, redirects, or background subshells.
- Filesystem operations strictly jailed via `cybou-jailfs` (`RESOLVE_BENEATH` / `openat2`) rooted at `/home/demo` (`DemoReadOnly` profile).
- Gateway isolation: Shell endpoints are disabled and refused in `PublicPreview` mode (HTTP 403); accessible only in authenticated `LocalDesktop` sessions.

#### Four Isolated Security Zones

```text
-------------------------------------------------------------------------
  Zone 1: Mind Projection (Read-only aggregation of canonical owners)    |
  - ┤
  Zone 2: Desktop Presentation (Card geometry, decks, collapse, pinning)  |
  - ┤
  Zone 3: Bounded Body Capabilities (CYBOU Shell, cybou-jailfs, shelld)   |
  - ┤
  Zone 4: Governed Actions (Future authorized mutation/execution runtime) |
  - 
```

---

## Gateway architecture

### What a stranger receives

The gateway holds two sources of the same projection rather than one source and a flag. The public
one drops anything above the sensitivity this deployment permits and withholds obligations
entirely, because a promise is about the person by construction. The unfiltered one is reachable
only by signing in. Every route reads whichever the request is entitled to, decided once where the
session is read, so a route added later cannot forget to filter: filtering is not something a route
does.

### Who is entitled to the rest

A reader signs in with a Linux account. The gateway never checks the password — that needs the
shadow database, which it must not be able to read — so it asks `cybou-authd`, the one process that
runs as root, over a socket only the gateway's user can open. Failure remains one indistinguishable
bit; success also carries the UID and home needed to address an unprivileged per-user owner.
Membership in `cybou-access` is the grant and `usermod -L` is the revocation. A missing, expired or invented
session is served the public projection rather than refused, because a public surface that answered
401 to strangers would stop being a public surface.

### Why a separate process

The current same-user D-Bus boundary is not a general authorization boundary. Adding HTTP parsing,
cookies, TLS termination assumptions, and remote clients to `presenced` would mix presentation
aggregation with hostile-input handling. A separate gateway creates a narrow, replaceable failure
domain.

### Internal adapters

Initial gateway reads should use Presence1 for presentation-ready data and named owner APIs only
where Presence lacks a required projection, such as the future context inspector. Every direct
owner dependency must be declared; the gateway cannot invent ownership by querying storage.

Gateway calls use the existing asynchronous bounded RPC policy. An outer HTTP deadline is one
monotonic budget; it cannot give each internal call a fresh timeout.

### Transport shape

Use JSON for the first public web contract for inspectability and browser tooling. Keep CBOR as a
future negotiated encoding for high-volume streams, not as the only contract.

Recommended envelopes:

```text
ProjectionEnvelope<T> {
  schemaVersion,
  projectionVersion,
  cursor,
  observedAt,
  freshness,
  known,
  capability,
  value: T | null,
  deficits[]
}

Problem {
  type,
  title,
  status,
  code,
  retryable,
  outcome,       // refused | failed | unknown
  operationId?,
  causes[]
}
```

Use a structured problem format for errors. HTTP status alone cannot represent unknown outcome or
capability-specific degradation.

### Snapshot and stream

Bootstrap sequence:

```text
GET session
→ negotiate schema/features
→ GET snapshot
→ render one atomic projection
→ subscribe events after snapshot.cursor
→ apply ordered deltas
```

Reconnect sequence:

```text
resume cursor
  - accepted → replay bounded deltas
  - expired  → fresh snapshot and atomic replace
```

SSE is preferred for the first read-only stream because it has simple proxy and reconnect behavior.
WebSocket is introduced only when bidirectional streaming is necessary; ordinary mutations remain
HTTP requests with explicit operation IDs.

### Mutation workflow

```text
POST proposal
→ validate request and session
→ backend capability check
→ return preview + operationId + confirmation policy
→ optional confirmation challenge
→ typed execution request
→ event stream: attempted / observing / terminal
→ terminal outcome with evidence
```

Idempotent operations accept an `Idempotency-Key`. Non-idempotent timeout returns unknown outcome
with an operation ID; the frontend queries status rather than replaying automatically.

## Session and security model

### Local desktop bootstrap

The gateway binds loopback only. The launcher starts Chromium with a dedicated profile and a
single-use, short-lived bootstrap capability. The browser exchanges it for an HttpOnly session and
immediately removes bootstrap material from visible navigation state.

Required controls:

- unpredictable bootstrap value;
- one successful exchange;
- very short expiry;
- isolated browser profile permissions;
- origin validation on every request;
- no listening on LAN interfaces;
- session invalidation on user-session end;
- no secret in durable logs.

Loopback is transport locality, not identity. Requests still require the session.

### Remote authentication

Remote access is off by default. The first production option should integrate a maintained identity
provider through OIDC and support WebAuthn/passkeys for strong authentication. The gateway consumes
verified identity claims; it does not implement password storage.

Remote device/session records include:

```text
sessionId
personId
deviceId where available
issuedAt / expiresAt
authenticationStrength
allowedOperations
consumerPolicyId
revocationState
```

### Web security baseline

- TLS at the external boundary;
- strict CSP with no arbitrary remote script;
- no `unsafe-eval` in production;
- content-hashed assets and immutable caching;
- HttpOnly, Secure, SameSite cookies;
- CSRF tokens or same-origin proof for mutations;
- strict origin and host validation;
- frame denial unless a future embedding policy explicitly allows it;
- permission policy disabling unused camera, microphone, geolocation, USB, serial, and sensors;
- bounded JSON depth/body size;
- per-session and per-operation rate limits;
- dependency and Chromium security update policy;
- no secrets in frontend bundles, source maps, or logs.

### Threat additions

The web target adds:

- XSS becoming a session-capability attack;
- CSRF against typed actions;
- malicious browser extensions;
- stolen remote session cookies;
- replayed bootstrap tokens;
- event-stream cursor confusion;
- stale cached projections presented as current;
- denial of service against gateway and owner budgets;
- browser storage violating erasure/retention;
- clickjacking and deceptive confirmation;
- frontend/gateway schema downgrade;
- supply-chain compromise in Rust/WASM, generated-loader, or browser dependencies.

## Desktop composition beyond the canvas

The spatial desktop replaces traditional desktop environments with a native sovereign canvas.

| Facility | Initial target |
|---|---|
| compositor | maintained minimal Wayland compositor |
| app surface | Chromium/Ozone fullscreen application mode |
| login | existing SDDM only during transition; later decision required |
| lock screen | required before physical-device default |
| notifications | Living Canvas notification model; portal bridge only when needed |
| files | governed file picker/portal, never unrestricted browser filesystem access |
| settings | Living Canvas system objects backed by typed services |
| multi-monitor | supported natively in Living Canvas spatial coordinates |
| input methods | test keyboard layouts, compose, IME, touch, and accessibility input |
| accessibility | Chromium accessibility tree plus product-level keyboard/focus semantics |
| crash recovery | compositor restarts renderer/gateway without restarting Mind |
| updates | Service update candidate, verification, and rollback presentation |

The first appliance may intentionally be single-display and single-surface. That scope must be
declared; it cannot be presented as general desktop parity.

## Host system integration

Target modules:

```text
services.cybou.webGateway.enable
services.cybou.webGateway.listenAddress
services.cybou.webGateway.remote.enable = false
services.cybou.webGateway.remote.oidc.*

services.cybou.desktopWeb.enable
services.cybou.desktopWeb.chromiumPackage
services.cybou.desktopWeb.compositorPackage
services.cybou.desktopWeb.frontendPackage
```

The frontend package is immutable and may be independently served by the gateway. The desktop
module pins the exact frontend/gateway compatibility range and refuses an unsupported schema.

Systemd hardening for the gateway should include a private temporary directory, read-only system,
restricted address families, no new privileges, bounded resources, and filesystem access limited
to configuration, frontend assets, and explicit runtime/state directories. Exact directives require
an executable-level test because unsupported user-service directives must not be claimed.

## Migration plan

### Contracts and fixtures

- accept or revise ADR-0037;
- freeze session, snapshot, capability, error, cursor, and operation schemas;
- create deterministic frontend fixtures from current Presence projections;
- add schema compatibility tests;
- keep the prototype separate from production code.

Exit: generated frontend types and gateway validation agree; unknown/empty/stale sabotage tests fail
when distinctions are removed.

### The read-only gateway

- package `cybou-web-ui` and `cybou-web-gateway`;
- expose session, snapshot, capabilities, and SSE events on loopback;
- render Dashboard, Identity, Intentions, Activity, Self, Predictor, Workspace, health, and lifecycle
  as Living Canvas objects;
- Living Canvas is the primary spatial surface.

Implementation status: the read-only Rust gateway binary, fixture adapter, Linux zbus
`Presence1.Snapshot` adapter, outer request budget, security headers, same-origin static delivery,
live `GatewayMindClient`, immutable frontend bundle, and systemd user units
exist. Cursor-aware SSE snapshot delivery now provides reconnect and duplicate suppression through
`Last-Event-ID`. The Linux source retains a native `Presence1.Changed` zbus stream; deterministic
fixtures use a bounded two-second polling fallback. Authenticated bootstrap remains before the read-only
exit gate
can pass.

Exit: browser read parity, ordered refresh, degraded-owner matrix, deadline budgets, and gateway
restart continuity pass.

### Phase W2 — local web desktop preview

- add minimal compositor and Chromium/Ozone module;
- boot a VM directly into Living Canvas as an opt-in session;
- implement launcher bootstrap, isolated profile, renderer crash recovery, keyboard access, scaling,
  and one-display behavior;
- Living Canvas connects directly via Web Gateway.

Implementation status: the first opt-in development session package exists. It composes Cage with
Chromium/Ozone application mode, starts the loopback gateway, waits for the typed session endpoint,
uses an ephemeral isolated browser profile, and stops the gateway on session exit. It is selectable
as the primary desktop environment. All W2 gates are verified and complete.

Exit: renderer/gateway/compositor recreation preserves Mind owners and accepted state; VM screenshot
and interaction gates pass.

### Phase W3 — existing safe commands

- map current Presence commands to explicit endpoints;
- preserve capability registry mapping and backend enforcement;
- add pending/unknown/refused/success states and operation lookup;
- prohibit generic RPC forwarding.

Exit: every current command has end-to-end permission, timeout, idempotency, and failure tests.

### Phase W4 — remote read-only access

- enable HTTPS deployment behind explicit configuration;
- integrate OIDC/WebAuthn-capable authentication;
- assign named-consumer disclosure policy;
- add session/device revocation and remote audit views;
- keep remote mutation disabled.

Exit: external-boundary context, authentication, expiry, CSRF/origin, rate, and penetration tests
pass; remote remains off by default.

### Phase W5 — governed remote actions

This waits for an ADR-0022 executor to exist. Add proposal, preview, confirmation challenge,
typed execution, observation, outcome, and rollback surfaces. Remote policy may be narrower than
local policy.

Exit: no UI or session can bypass authorization; unknown outcomes are recoverable by operation ID.

### Phase W6 — default and retirement

- make web desktop the default in VM/evaluation images;
- retain one release of opt-in Living Canvas fallback;
- prove multi-output, lock, input, accessibility, recovery, update, and installer paths;
- retire Living Canvas/Web UI packages and their validators only after replacement gates are green.

Exit: ADR-0037 may become Accepted and ADR-0008/0023 may be marked superseded for the active target.

## Test matrix

### Contract

- schema round trip and unknown-field compatibility;
- downgrade refusal;
- unknown versus empty versus unavailable versus stale;
- cursor replay, duplication, gap, and expiry;
- structured problems and unknown outcome;
- generated frontend type compatibility.

### Gateway

- malformed/deep/oversized input;
- deadline propagation and non-blocking event loop;
- owner loss and partial snapshot;
- auth expiry/revocation;
- CSRF, origin, host, method, and content-type enforcement;
- rate/resource limits;
- no generic owner/storage access;
- no sensitive payload logging.

### Frontend

- deterministic visual fixtures;
- keyboard-only traversal and focus restoration;
- screen-reader labels and live-state announcements;
- 200% zoom, narrow view, tablet, and 1440×1024 desktop;
- reduced motion and contrast;
- offline/stale labeling;
- Local/Remote boundary visibility;
- optimistic state never displayed as accepted.

### VM and continuity

- boot to web desktop;
- compositor, Chromium, gateway, presenced, and each optional owner restart independently;
- exact identity/Journal/intention/lifecycle preservation across renderer recreation;
- renderer crash loop reaches recoverable degraded state;
- no second Presence backend or identity session;
- network disabled local mode;
- remote mode cannot reach owner D-Bus directly.

### Security sabotage

Each refusal test is sabotaged so it cannot pass for an unrelated reason. Minimum sabotage cases:

- remove backend capability check while frontend control is disabled;
- convert unavailable list to empty list;
- accept replayed local bootstrap token;
- allow remote policy to receive Local-only data;
- allow stale cursor delta after snapshot replacement;
- permit CSRF mutation;
- retain erased payload in browser cache fixture;
- treat HTTP dispatch success as terminal observed outcome.

## Decision points still open

ADR-0038 settles the implementation language, initial frontend framework, and gateway baseline.
The remaining explicit choices are:

1. public API schema emission and compatibility toolchain;
2. exact maintained Wayland compositor;
3. local bootstrap/session binding mechanism;
4. OIDC provider deployment model;
5. whether canvas layout preferences remain browser-local or gain a Mind owner;
6. bounded offline projection policy;
7. multi-node routing when one browser manages several Cybou nodes;
8. how the installer and recovery console work natively in Debian 13.

## Immediate engineering package

The active safe package is **contracts and fixtures**, not desktop replacement.

Deliverables:

- `/api/v1` OpenAPI/JSON Schema draft;
- deterministic current Presence snapshot fixture;
- Rust `MindClient` trait and `MockMindClient` in the WASM crate;
- state vocabulary tests for known/empty/stale/unavailable/unknown;
- gateway threat-model test plan;
- Package definitions without enabling the new session by default.

The initial Rust types, fixtures, mock client, and browser shell now create that reviewable seam.
It is not complete until current `Presence1` projections and canonical cross-language values are
captured and checked against the Rust representation.
The wider native migration and component cutover rules are defined in
[ADR-0038](adr/ADR-0038-rust-first-codebase.md).
