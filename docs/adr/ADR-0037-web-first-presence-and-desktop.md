<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0037: Web-First Presence and Chromium Desktop

## Status

Proposed

## Context

Cybou currently presents Mind through a KDE Plasma/QML dock. That implementation proved the
presentation boundary, process restart independence, degraded-state projection, keyboard access,
and durable-before-visible ordering. It also binds the product experience to Plasma panels, QML,
and one local graphical session.

The target product needs one interface that can run:

- as the complete local Cybou desktop;
- in a normal browser from a Cybou server;
- as an installed PWA where browser capabilities are sufficient;
- without creating separate desktop and web frontends.

ADR-0001 already makes Presence replaceable. This decision changes the rendering and delivery
boundary; it does not change Mind ownership.

## Decision

### One frontend artifact

Cybou has one versioned web frontend artifact, called **Living Canvas**.

The same content-hashed HTML, Rust-generated WebAssembly and loader, CSS, fonts, and raster assets are served in local and
remote modes. Mode-specific behavior comes from a negotiated session contract, never from a forked
UI implementation.

```text
                         one frontend build
                                │
                 ┌──────────────┴──────────────┐
                 ▼                             ▼
       Chromium/Ozone desktop             HTTPS browser
            local session                 remote session
                 │                             │
                 └──────────────┬──────────────┘
                                ▼
                       cybou-web-gateway
                                │
                         Presence1 / future
                         typed Mind services
```

### The browser is a renderer and untrusted client

Frontend WebAssembly does not become a Mind owner, Journal client, D-Bus peer, authorization authority, or
privileged executor.

```text
frontend request ≠ permission
frontend state   ≠ canonical state
button enabled   ≠ capability granted
HTTP success     ≠ observed outcome
```

The frontend talks only to `cybou-web-gateway`. It never receives a generic D-Bus bridge, shell
bridge, filesystem bridge, or unrestricted native IPC object.

ADR-0038 defines both frontend and gateway as Rust. Generated WASM loader code is packaging output,
not an authored JavaScript application or a privileged native bridge.

### A dedicated gateway owns the browser/network boundary

`cybou-web-gateway` is a separate process from `cybou-presenced`.

It owns:

- static frontend delivery;
- browser-session authentication and expiry;
- origin, CSRF, request-size, and rate-limit enforcement;
- schema/version negotiation;
- conversion between web transport and explicit Presence/Mind operations;
- projection cursors and event streaming;
- remote-session audit metadata;
- refusal before a request reaches a Mind service when the session lacks permission.

It does not own biography, identity, intentions, attention, health, lifecycle, epistemic state,
context, or authorization policy.

Network parsing and browser authentication do not move into `cybou-presenced`. Presence remains the
presentation aggregator for current projections until a later owner decision replaces or narrows
it.

### Local and remote are different trust contexts

The visual interface is identical. The security context is not.

```text
LocalDesktopSession
  device-bound
  loopback-only gateway
  isolated Chromium profile
  local Presence consumer policy

RemoteBrowserSession
  authenticated person/device
  TLS
  explicit remote access policy
  narrower default disclosure
  revocable session and device grants
```

Changing a Local/Remote control in the UI cannot widen the current session. It selects or explains
a separately established transport/session.

### Local desktop composition

The target local graphical environment is:

```text
NixOS
  ├── systemd --user Mind services
  ├── cybou-web-gateway (loopback only)
  ├── minimal Wayland compositor
  └── Chromium/Ozone application surface
          └── Living Canvas
```

The compositor provides display, input, output management, lock/session transitions, and recovery
of the single Cybou surface. It does not become Mind.

The first implementation should evaluate a maintained single-application Wayland compositor rather
than building a compositor. The exact compositor is an implementation choice gated by multi-output,
input, accessibility, lock-screen, GPU, and recovery tests.

Chromium runs with:

- Wayland/Ozone;
- an isolated Cybou profile;
- no arbitrary extensions;
- no developer tools in production policy;
- no navigation outside allowed origins in desktop-shell mode;
- renderer sandbox enabled;
- a pinned frontend/gateway origin;
- crash recovery that does not restart Mind owners.

Chromium is replaceable. Living Canvas depends on web standards and the gateway contract, not on an
Electron preload API.

### Hosted composition

The same frontend build may be served behind an HTTPS reverse proxy. Remote access is disabled by
default and requires explicit configuration.

The remote boundary requires:

- authenticated user and device/session identity;
- secure, HttpOnly, SameSite session cookies;
- CSRF protection for mutations;
- origin allowlisting;
- short-lived sessions and revocation;
- rate and concurrency limits;
- WebSocket/SSE authorization equivalent to request authorization;
- no direct exposure of D-Bus or owner services;
- explicit policy for which data may cross the external boundary.

### Web contract

The web API is versioned independently of internal D-Bus encoding.

Minimum read contract:

```text
GET /api/v1/session
GET /api/v1/snapshot
GET /api/v1/objects/{id}
GET /api/v1/capabilities
GET /api/v1/events?after={cursor}
```

Every projection response carries enough state to avoid turning failure into fact:

```text
schemaVersion
projectionVersion
cursor
observedAt
freshness
known
capabilityState
sourceHighWaterMark where applicable
```

Unknown, unavailable, stale, timeout, and empty are different results.

Event delivery is resumable by cursor. A reconnect fetches a fresh snapshot when its cursor is no
longer valid. The browser never assumes that missed events can be reconstructed from local UI state.

### Mutation contract

No generic `POST /rpc` endpoint is permitted.

Each mutation has a typed resource, validated request schema, declared capability, idempotency
semantics, and typed outcome. Before M10, the gateway may expose only existing bounded Presence
commands and only after their current backend capability gates are preserved.

Future privileged mutations cross ADR-0022:

```text
proposal → preview → authorization challenge → typed execution
         → observed consequence → outcome
```

A browser confirmation dialog or UI modal is evidence of user interaction, not the authorization
decision itself.

### Context and privacy

The local desktop frontend and each remote browser session are named consumers under ADR-0030.

- The canvas renders delivered projections; it does not assemble unrestricted Mind context.
- A remote session is an external-boundary consumer.
- A local renderer does not gain unrestricted context merely because it is local.
- Held-back, unknown, stale, disputed, and unavailable states remain visible as states rather than
  being silently omitted.
- Secrets and raw credentials never enter ordinary frontend state.
- Browser telemetry, caches, service workers, downloads, clipboard, and IndexedDB are treated as
  retention surfaces.

### Offline behavior

Static assets may be cached. Canonical Mind data is not silently mirrored into an offline PWA.

When disconnected, the frontend may show a clearly stale, bounded last projection only if policy
permits that cache. It cannot queue privileged mutations by default. Reconnection revalidates the
session, capabilities, cursor, and projection freshness.

### Packaging

The target adds three independently testable packages:

```text
cybou-web-ui        immutable frontend build
cybou-web-gateway   browser/network boundary
cybou-desktop-shell compositor + Chromium policy/launcher
```

The existing Plasma packages remain during migration. They are retired only after parity and
continuity gates pass.

## Compatibility with existing decisions

Preserved without change:

- ADR-0001 Body/Mind/Presence separation;
- ADR-0009 one presentation backend per user session;
- ADR-0011 single Journal writer;
- ADR-0012 process isolation;
- ADR-0013 local D-Bus fabric behind the gateway;
- ADR-0019 degraded capability semantics;
- ADR-0022 authorized action boundary;
- ADR-0024/0026 lifecycle ownership and recovery;
- ADR-0030 named-consumer delivery policy.

Replaced for the target surface, after implementation gates pass:

- ADR-0008 Plasma Mind Dock layout;
- ADR-0020 v0.1 Presence surface direction;
- ADR-0023 Plasma handle, edge reveal, and `Meta+M` access contract.

Those ADRs remain accurate historical/current implementation decisions until this ADR is accepted
and its replacement is demonstrated. They are not retroactively rewritten.

## Consequences

### Positive

- one frontend and interaction model for local and hosted use;
- rendering iteration no longer requires QML or Plasma packaging;
- Chromium behavior is consistent across the desktop appliance and normal Chromium browsers;
- the graphical shell becomes replaceable independently of Mind;
- remote access gains an explicit boundary instead of exposing local IPC;
- Living Canvas can make evidence, context, and authorization states first-class.

### Costs and risks

- Chromium is materially heavier than a QML surface;
- the gateway becomes a high-value network-facing component;
- browser storage and caches add privacy/retention surfaces;
- accessibility and input behavior must be rebuilt and tested at web level;
- remote authentication, TLS, revocation, and rate limiting become product responsibilities;
- removing KDE also removes mature desktop facilities that must be replaced or deliberately
  excluded: lock screen, settings, multi-monitor control, input methods, portals, file dialogs,
  notifications, power UI, and accessibility integration.

## Acceptance gates

| | Gate |
|---|---|
| **W1** | Local and hosted modes run the same content-hashed frontend artifact |
| **W2** | No browser path reaches D-Bus, Journal, shell, or filesystem through a generic bridge |
| **W3** | Restarting Chromium/gateway preserves identity, Journal, intentions, lifecycle, and owner PIDs |
| **W4** | A remote session cannot receive a projection disallowed by its named-consumer policy |
| **W5** | Unknown, unavailable, stale, timeout, and empty remain distinguishable end to end |
| **W6** | Event reconnect resumes by cursor or atomically replaces state from a fresh snapshot |
| **W7** | A frontend-disabled control cannot substitute for backend capability enforcement |
| **W8** | Every mutating endpoint has typed capability, validation, idempotency/outcome semantics, and CSRF protection |
| **W9** | Desktop renderer/gateway recreation cannot create an identity session or second Mind owner |
| **W10** | Local desktop boots to Living Canvas without KDE/Plasma and recovers from renderer crash |
| **W11** | Keyboard-only, screen-reader, reduced-motion, zoom, contrast, and focus-order gates pass |
| **W12** | Remote access is off by default and has authentication, TLS, expiry, revocation, origin, and rate-limit tests |
| **W13** | Browser caches and offline state obey privacy, sensitivity, retention, and erasure policy |
| **W14** | The Plasma surface is not removed until read, degraded, lifecycle, and safe-command parity is demonstrated |

## Alternatives considered

### Keep KDE Plasma and embed a WebView

Rejected as the target because it retains two UI stacks and does not provide one identical desktop
and hosted frontend. It remains a possible migration bridge.

### Tauri or another system-WebView wrapper

Rejected as the rendering baseline because Linux system WebViews use WebKitGTK, while the hosted
target is primarily Chromium. This weakens visual and behavioral parity.

### Electron as the architecture

Rejected as the architecture because it couples native capability APIs and Node packaging to the
frontend. Electron may be a portability package on operating systems where a managed Chromium
surface is otherwise inconvenient, provided the frontend still uses only the gateway contract.

### Expose D-Bus directly to JavaScript

Rejected because it turns presentation code into a broad local capability client and makes the
remote/local contract diverge.

### Reimplement Mind as a web backend

Rejected because transport does not justify moving canonical ownership out of the existing Mind
processes.

## Related documents

- [Web UI Integration Architecture](../WEB_UI_ARCHITECTURE.md)
- [Architecture](../ARCHITECTURE.md)
- [Presence API](../mind/PRESENCE_API.md)
- [Threat Model](../security/THREAT_MODEL.md)
- [ADR-0001](ADR-0001-system-architecture.md)
- [ADR-0008](ADR-0008-mind-dock-with-tabs.md)
- [ADR-0022](ADR-0022-authorized-action-boundary.md)
- [ADR-0023](ADR-0023-mind-dock-discoverability-and-access.md)
- [ADR-0030](ADR-0030-transparent-context-delivery.md)
- [ADR-0038](ADR-0038-rust-first-codebase.md)
