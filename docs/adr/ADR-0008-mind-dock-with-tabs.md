<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0008: Mind Dock with Organ Tabs

## Status

Accepted

## Context

Presence needs enough space to inspect Mind projections without turning the normal top panel into
a large popup.

Plasma layout templates provide an isolated, reusable boundary for the dedicated Mind surface.

## Decision

Use the dedicated `org.cybou.plasma.minddock` layout template loaded by the Cybou global layout.

The accepted desktop contract is:

```text
location   right
height     420        # Plasma vertical-panel width
lengthMode fill
alignment  center
hiding     autohide
floating   true
```

The panel reveals from the right screen edge and hides when the pointer leaves.

Inside the panel, the Presence applet uses its full representation. The shell is organized as:

```text
64px icon rail
+ compact page header
+ one active content page
```

The rail exposes Dashboard, Identity, Intentions, Activity, Self, Predictor, and Workspace.

Unavailable runtime state is presented inside the content area. It does not replace the dock shell
and it does not claim that QML owns or opens the Journal.

## Runtime boundary

After M4, `plasmashell` contains only the QML Presence proxy/cache.

The real presentation backend is `cybou-presenced`, and the organ processes remain outside Plasma.

## Consequences

### Positive

- normal top panel stays compact;
- Mind has a dedicated full-height inspection surface;
- the dock gives most horizontal space to content rather than navigation labels;
- auto-hide returns desktop space when Mind is not being inspected;
- unavailable state can expose the proxy error and retry connection;
- Plasma restart does not recreate the cognitive organ graph;
- QML validation checks the key Plasma 6 shell contracts.

### Trade-offs

- 420px is intentionally an inspection surface, not a general application window;
- richer degraded-mode diagnosis remains M6 work;
- panel reveal timing and animation are owned by Plasma rather than custom QML.

## Alternatives Considered

### Compact top-panel popup only

Rejected because the inspection surface is too dense for a normal panel popup.

### Text-heavy navigation rail

Rejected after VM visual testing because it consumed too much of the 420px dock width.

### Direct SQL/Journal diagnosis from QML

Rejected because the UI talks through Presence and must not infer cognitive persistence ownership.
