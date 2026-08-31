<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Mind Dock UI

> This document describes the implemented Plasma/QML surface. The proposed replacement is the
> single-frontend [Living Canvas Web UI Architecture](../WEB_UI_ARCHITECTURE.md) and
> [ADR-0037](../adr/ADR-0037-web-first-presence-and-desktop.md). Until its acceptance gates pass,
> this Plasma contract remains the current implementation authority.

## Purpose

The Mind Dock is a persistent inspection surface for Cybou's cognitive runtime, not a traditional
application window.

## Shell

```text
[ Desktop Hidden State ]  -->  Right screen edge shows subtle Cybou handle
[ Desktop Revealed State ] -->  Expanded sidebar showing active Presence, Cards & Telemetry
```

The main panel contract is:

```text
width       420px
navigation  64px
placement   right
hiding      autohide
```

The discoverability handle is a separate small panel:

```text
width       18px
length      82px
placement   right / centered
hiding      none
```

## Opening Mind

The user has four equivalent paths:

```text
touch right screen edge -> native auto-hide reveal
hover handle            -> temporary peek
click handle            -> pin / unpin
Meta+M                  -> pin / unpin
```

Hover is intentionally non-committal: the controller briefly reveals the panel and then restores
its native `autohide` mode. If the pointer has moved into the dock, Plasma keeps it visible until
the pointer leaves.

Click and Meta+M deliberately pin the panel by switching it to `hiding = none`; the next click or
shortcut restores `autohide`.

## First-run hint

On the first profile run, the handle shows a short hint:

```text
Cybou Mind lives here · hover to peek · Meta+M to pin
```

The handle persists an `onboardingSeen` flag so this hint is not repeated every login.

## Navigation and keyboard interaction

The 64px rail is icon-only. The active page uses:

```text
2px accent indicator
accent-colored icon
very light selected surface
```

The rail does not use a large selected tile.

When a rail item has keyboard focus:

```text
Up / Down -> previous / next page
Home      -> Dashboard
End       -> Workspace
Enter     -> activate focused button
Tab       -> continue through actionable page controls
```

Focus is shown with the active theme focus color rather than relying only on hover.

## Visual hierarchy

Package 07 establishes the following hierarchy:

- the common header is compact and owns page title + runtime state;
- static metric cards use a quiet alternate surface with a small accent strip;
- generic file-looking icons are not shown in metric cards by default;
- emphasized information uses accent, not a thick border;
- list rows use subtle hover and keyboard-focus surfaces;
- empty states are deliberately quiet;
- scrollbars are thin, as-needed, and fade visually when not interacted with.

## Responsive behavior

Dashboard, Identity, and Self are scrollable even on short VM windows.

Two-column metric grids collapse to one column when their available width is below the local
threshold:

```text
>= 280px -> 2 columns
<  280px -> 1 column
```

This does not resize the Plasma panel itself; it only makes page content robust to narrower
containment geometry.

## Unavailable state

The dock shell remains stable when Presence cannot connect.

The content area shows:

```text
Mind unavailable
proxy diagnostic if available
Retry connection
```

It must not say that QML failed to open `journal.db`. Journal ownership belongs to eventd.

## Runtime boundary

The access handle and `DockAccess` live in the Plasma shell layer only.

`DockAccess` may alter Plasma panel visibility through the plasmashell scripting interface. It must
not own Presence state, call cognitive organs, or read/write cognitive persistence.

Visual polish is QML-only. It must not change organ ownership, Journal behavior, or the Presence
process boundary.

## Validation

`scripts/validate-qml-api.py` checks the main Presence shell.

`scripts/validate-mind-access.py` checks discoverability and access.

`scripts/validate-ui-polish.py` checks the Package 07 visual/interaction contracts:

- thin active rail indicator;
- keyboard rail navigation and focus rings;
- compact header;
- soft cards and reduced generic icon use;
- thin as-needed scrollbars;
- responsive static pages;
- keyboard-focusable list/action controls;
- animated and accessible Mind handle.
