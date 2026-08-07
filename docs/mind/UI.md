<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Mind Dock UI

## Purpose

The Mind Dock is a persistent inspection surface for Cybou's cognitive runtime, not a traditional
application window.

## Shell

```text
right screen edge

desktop hidden state
┌─────────────────────────────────────▌
│                                     ▌
│                                     ▌  Cybou handle
│                                     ▌
└─────────────────────────────────────▌

revealed state
┌──────────────────────┬───────────────┐
│                      │ page   Online │
│       desktop        │               │
│                      │ active page   │
│                      │               │
└──────────────────────┴───────────────┘
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

Only icons live in the navigation rail. Labels are shown through tooltips and the page header.

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

## Unavailable state

The dock shell remains stable when Presence cannot connect.

The content area shows:

```text
Mind unavailable
proxy diagnostic if available
Retry connection
```

It must not say that QML failed to open `journal.db`. Journal ownership belongs to eventd.

## Visual language

- use the active Plasma/Kirigami theme;
- use the theme highlight color for selection and healthy status;
- use compact cards rather than large centered labels;
- keep page headings in the common header, not duplicated inside every page;
- lists should use subtle hover surfaces and keep primary text left aligned;
- empty states should be explicit rather than leaving blank space;
- the handle is a small accent-colored capsule, not a second toolbar.

## Runtime boundary

The access handle and `DockAccess` live in the Plasma shell layer only.

`DockAccess` may alter Plasma panel visibility through the plasmashell scripting interface. It must
not own Presence state, call cognitive organs, or read/write cognitive persistence.

## Validation

`scripts/validate-qml-api.py` checks the main Presence shell.

`scripts/validate-mind-access.py` additionally checks:

- separate Presence and handle packages;
- hover/click/global-shortcut access hooks;
- persistent onboarding flag;
- main panel remains native `autohide`;
- handle panel is separate, visible, centered, and custom-length;
- the layout assigns `Meta+M` to the handle applet.
