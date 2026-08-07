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
┌──────────────────────────────────────┐
│ icon rail │ page title      ● Online │
│           ├───────────────────────────┤
│ Dashboard │                           │
│ Identity  │       active page         │
│ ...       │                           │
│           │                           │
└──────────────────────────────────────┘
```

The production desktop contract is:

```text
width       420px
navigation  64px
placement   right
hiding      autohide
```

Only icons live in the navigation rail. Labels are shown through tooltips and the page header.

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
- empty states should be explicit rather than leaving blank space.

## Interaction

- touching the right screen edge reveals the panel;
- leaving the panel lets Plasma hide it;
- navigation changes only the content stack;
- Intentions supports Fulfill and Abandon directly;
- Self exposes an explicit Reflect action;
- Predictor keeps prediction requests inside the Predictor page;
- connection retry is a presentation action against the Presence proxy.

## Validation

`scripts/validate-qml-api.py` checks:

- direct Plasma 6 `preferredRepresentation`;
- no pre-M4 Journal error copy in QML;
- icon-only navigation;
- required shell components;
- existing Layout and Icon API invariants.
