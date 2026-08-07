<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0023: Mind Dock Discoverability and Access

## Status

Accepted

## Context

Native Plasma auto-hide gives the Mind Dock good desktop ergonomics but poor discoverability:
a new user has no visible indication that a cognitive surface exists at the right screen edge.

The access mechanism must not move cognition back into Plasma. It is presentation-shell behavior
only.

## Decision

Keep the main 420px Mind Dock as a native right-side auto-hide Plasma panel.

Add a second, tiny right-edge panel containing `org.cybou.mindhandle`:

```text
location   right
height     18
lengthMode custom
length     82
alignment  center
hiding     none
floating   true
```

The handle provides three access paths:

```text
right screen edge      -> native Plasma reveal
hover handle           -> temporary peek
click handle           -> pin / unpin
Meta+M                 -> pin / unpin
```

A first-run tooltip explains where Mind lives and mentions `Meta+M`. The hint is persisted as seen
through the handle applet configuration and is not shown on every login.

`DockAccess` is a shell-only QML type. It asks plasmashell asynchronously to change the hiding mode
of the panel that contains `org.cybou.presence`. It does not talk to eventd, organ services, or the
Journal.

## Consequences

### Positive

- the hidden Mind surface has a visible affordance;
- pointer, click, edge, and keyboard access all exist;
- the normal state still returns desktop space through native auto-hide;
- Meta+M works without requiring the pointer to be near the panel;
- Plasma owns reveal/hide animation rather than custom QML animation;
- the access controller remains outside the cognitive domain.

### Trade-offs

- the desktop now has a second very small Plasma panel;
- click/shortcut pinning temporarily changes the main panel from `autohide` to `none`;
- native edge reveal does not expose a perfect synchronized "open" state to the handle, so the
  handle remains a small visible grip even while the dock is open;
- shell scripting through plasmashell D-Bus is a desktop-integration dependency and must remain
  isolated from Mind/domain libraries.

## Failure behavior

If the plasmashell scripting D-Bus call fails, the handle stays present and surfaces a short tooltip
diagnostic. No cognitive process is restarted or mutated.

## Alternatives Considered

### Auto-hide only

Rejected because the VM showed that users could not discover how to open Mind.

### Permanent 420px dock

Rejected because it consumes too much desktop space.

### Handle drawn inside the auto-hide panel

Rejected because it disappears with the panel and therefore does not solve discoverability.

### Custom KWin effect for the handle

Rejected for v0.1 because a normal Plasma panel is simpler, theme-aware, and already managed by the
desktop shell.
