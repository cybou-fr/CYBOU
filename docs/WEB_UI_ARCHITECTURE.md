<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Living Canvas Web UI Integration Architecture

## Purpose and status

This document is the implementation blueprint for the Proposed
[ADR-0037](adr/ADR-0037-web-first-presence-and-desktop.md). It describes target architecture, not
current behavior. [Current State](CURRENT_STATE.md) remains authoritative until each migration gate
is demonstrated.

The objective is not to port the Plasma dock to HTML. It is to make the Rust/WASM Living Canvas the complete
web-first Cybou environment while preserving the ownership, continuity, failure, privacy, and
authorization properties already proved by Mind.

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

Plasma-specific documentation remains useful evidence: ADR-0008/0023 and `mind/UI.md` identify
discoverability, keyboard access, unavailable state, short-window behavior, shell recreation, and
stable Presence ownership as product requirements. Their implementation mechanism is not carried
forward.

## Target topology

```text
                                PERSON
                                   │
                    ┌──────────────┴──────────────┐
                    ▼                             ▼
          local Chromium/Ozone              remote Chromium
          isolated appliance UI             HTTPS browser/PWA
                    │                             │
                    └──────────────┬──────────────┘
                                   ▼
                          cybou-web-gateway
                    auth / session / schema / CSP
                    CSRF / cursor / rate / origin
                                   │
                            local session D-Bus
                                   ▼
                            cybou-presenced
                                   │
          ┌──────────┬─────────────┼──────────────┬───────────┐
          ▼          ▼             ▼              ▼           ▼
       healthd   lifecycled    intentiond     contextd     other owners
                                   │
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
  web-gateway.nix
  desktop-web.nix

packages/
  cybou-web-ui/
  cybou-web-gateway/
  cybou-desktop-shell/
```

The current React `living-canvas/` prototype remains visual evidence and interaction exploration.
The production Rust/WASM shell lives in `crates/living-canvas`. Its production browser adapter is
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

## Gateway architecture

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
  ├── accepted → replay bounded deltas
  └── expired  → fresh snapshot and atomic replace
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

Leaving KDE means deliberately replacing or deferring mature facilities.

| Facility | Initial target |
|---|---|
| compositor | maintained minimal Wayland compositor |
| app surface | Chromium/Ozone fullscreen application mode |
| login | existing SDDM only during transition; later decision required |
| lock screen | required before physical-device default |
| notifications | Living Canvas notification model; portal bridge only when needed |
| files | governed file picker/portal, never unrestricted browser filesystem access |
| settings | Living Canvas system objects backed by typed services |
| multi-monitor | explicit acceptance gate before KDE removal |
| input methods | test keyboard layouts, compose, IME, touch, and accessibility input |
| accessibility | Chromium accessibility tree plus product-level keyboard/focus semantics |
| crash recovery | compositor restarts renderer/gateway without restarting Mind |
| updates | Nix generation candidate, verification, switch, and rollback presentation |

The first appliance may intentionally be single-display and single-surface. That scope must be
declared; it cannot be presented as general desktop parity.

## NixOS integration

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

### Phase W0 — contracts and fixtures

- accept or revise ADR-0037;
- freeze session, snapshot, capability, error, cursor, and operation schemas;
- create deterministic frontend fixtures from current Presence projections;
- add schema compatibility tests;
- keep the prototype separate from production code.

Exit: generated frontend types and gateway validation agree; unknown/empty/stale sabotage tests fail
when distinctions are removed.

### Phase W1 — read-only gateway beside Plasma

- package `cybou-web-ui` and `cybou-web-gateway`;
- expose session, snapshot, capabilities, and SSE events on loopback;
- render Dashboard, Identity, Intentions, Activity, Self, Predictor, Workspace, health, and lifecycle
  as Living Canvas objects;
- keep Plasma as the default surface.

Implementation status: the read-only Rust gateway binary, fixture adapter, Linux zbus
`Presence1.Snapshot` adapter, outer request budget, security headers, same-origin static delivery,
live `GatewayMindClient`, immutable frontend derivation, and disabled-by-default NixOS user module
exist. Cursor-aware SSE snapshot delivery now provides reconnect and duplicate suppression through
`Last-Event-ID`. The Linux source retains a native `Presence1.Changed` zbus stream; deterministic
fixtures use a bounded two-second polling fallback. Authenticated bootstrap remains before the W1
exit gate
can pass.

Exit: browser read parity, ordered refresh, degraded-owner matrix, deadline budgets, and gateway
restart continuity pass.

### Phase W2 — local web desktop preview

- add minimal compositor and Chromium/Ozone module;
- boot a VM directly into Living Canvas as an opt-in session;
- implement launcher bootstrap, isolated profile, renderer crash recovery, keyboard access, scaling,
  and one-display behavior;
- preserve Plasma session as fallback.

Implementation status: the first opt-in development session package exists. It composes Cage with
Chromium/Ozone application mode, starts the loopback gateway, waits for the typed session endpoint,
uses an ephemeral isolated browser profile, and stops the gateway on session exit. It is selectable
beside Plasma and is deliberately not the default. The W2 exit gates listed below remain open.

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

This phase waits for ADR-0022/M10 implementation. Add proposal, preview, confirmation challenge,
typed execution, observation, outcome, and rollback surfaces. Remote policy may be narrower than
local policy.

Exit: no UI or session can bypass authorization; unknown outcomes are recoverable by operation ID.

### Phase W6 — default and retirement

- make web desktop the default in VM/evaluation images;
- retain one release of opt-in Plasma fallback;
- prove multi-output, lock, input, accessibility, recovery, update, and installer paths;
- retire Plasma/QML packages and their validators only after replacement gates are green.

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
8. how the installer and recovery console work after KDE retirement.

## Immediate engineering package

The active safe package is **W0 — contracts and fixtures**, not desktop replacement.

Deliverables:

- `/api/v1` OpenAPI/JSON Schema draft;
- deterministic current Presence snapshot fixture;
- Rust `MindClient` trait and `MockMindClient` in the WASM crate;
- state vocabulary tests for known/empty/stale/unavailable/unknown;
- gateway threat-model test plan;
- Nix package skeletons without enabling the new session by default.

The initial Rust types, fixtures, mock client, and browser shell now create that reviewable seam.
W0 is not complete until current `Presence1` projections and canonical cross-language values are
captured and checked against the Rust representation.
The wider native migration and component cutover rules are defined in
[Rust Migration Plan](RUST_MIGRATION.md).
