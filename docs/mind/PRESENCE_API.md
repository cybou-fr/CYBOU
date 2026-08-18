<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Presence API

## Process boundary

The real backend is `cybou-presenced` on:

```text
org.cybou.Mind.Presence1
```

The C++ `Presence` type exported to QML is only a proxy/cache.

## QML properties

```text
awake
runtimeReachable
aggregateCapabilityState
capabilityStates
capabilityDetails
capabilityDeficits
capabilityObservedAt
commandAvailability
lastError
narration
obligations
attention
contributions
stats
identityState
calibrations
coalitions
moment
organHealth
lifecycleMode
lifecycleStatus
lifecycleState
lifecycleProjection
lifecycleScheduling
lifecycleCommandPending
```

`awake` is a compatibility alias for presentation reachability, not a global capability grant.
`hasCapability(id)` is the QML command gate. `lifecycleScheduling` is the read-only Lifecycle1
policy evaluation and explains why automatic work is blocked or deferred.

`capabilityDetails[id]` is the UI-ready explanation for one capability. It contains `state`,
`available`, `causes`, `impacts`, `dependencies`, `lastVerifiedAt`, `recoveryPolicies`, and
`recoveryProgress`. Raw `capabilityDeficits` remains available for diagnostics, but UI code does
not need to group or rank those records itself.

`commandAvailability[id]` contains `available`, `requiredCapabilities`, and
`missingCapabilities`. `canCommand(id)` is the corresponding QML convenience gate. Presence owns
this presentation mapping; the target organ still enforces the same capability requirements.

`lastError` is presentation diagnostics for connection/retry UX. It does not make QML the owner of
the failing organ or storage resource.

## Commands

```text
wake()
promise(description)
reflect()
fulfillIndex(index)
abandonIndex(index)
observe(subject, value)
predict(subject)
hasCapability(id)
canCommand(id)
interruptLifecycle(cause)
```

`wake()` is explicitly invokable so the unavailable-state UI can retry the Presence1 connection.
If retry fails, `awake` and `runtimeReachable` become false while the last cached projection remains
available for stale-state presentation. A later successful `wake()` replaces it atomically.

presenced routes cognitive operations to the organ that owns them; it does not construct domain
organ objects itself.

Creating another QML Presence object creates another proxy only.

## Proposed web mapping

The Proposed [web-first Presence decision](../adr/ADR-0037-web-first-presence-and-desktop.md) keeps
Presence1 behind `cybou-web-gateway`; it does not translate this interface into a generic browser
RPC bridge.

The gateway may map presentation-ready fields into a separately versioned web snapshot while
preserving:

- `awake` as presentation reachability rather than authority;
- capability and command-availability explanations;
- known versus empty versus unavailable state;
- freshness and degraded causes;
- one bounded request budget;
- backend enforcement for every command;
- QML/browser recreation as projection recreation, not identity or biography restart.

The web schema is defined and tested independently so internal D-Bus evolution does not become a
public network protocol by accident.

The initial read-only implementation uses zbus on Linux to call only `Snapshot`. It validates the
fabric CBOR envelope version before mapping capability states into web schema v1. The gateway
assigns a process-local projection version and cursor because current `Presence1` does not expose a
durable web cursor. A gateway restart therefore forces a fresh snapshot; it cannot claim event
resumption yet. No browser route maps the existing mutation methods.
