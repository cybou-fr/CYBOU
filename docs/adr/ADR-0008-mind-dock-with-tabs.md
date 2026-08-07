<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0008: Mind Dock with Organ Tabs

## Status

Accepted

## Context

The Presence surface needs enough space to inspect Mind projections without turning the normal top
panel into a large popup.

Inline dock creation in the global layout would couple a failure in the Mind panel to the rest of
the default desktop layout. Plasma layout templates provide a reusable, isolated boundary that can
be loaded with `loadTemplate()`.

## Decision

Use a dedicated Plasma layout template, `org.cybou.plasma.minddock`, loaded from the Cybou global
layout.

The current accepted implementation uses:

```text
location   right
height     420        # Plasma vertical-panel width
lengthMode fill
alignment  center
hiding     none
floating   true
```

`hiding = "none"` is deliberate while the Mind UI is under active development.

The template contains the `org.cybou.presence` applet. The applet chooses its full representation
when Plasma gives it a vertical form factor, and the full representation embeds `MindDock`.

`MindDock` currently provides a Dashboard plus projections for Identity, Intentions, Activity,
Self, Predictor, and Workspace.

Presence remains the surface boundary: QML does not open the cognitive Journal directly.

## Consequences

### Positive

- Mind inspection has a dedicated full-height surface;
- the normal top panel remains compact;
- the dock is reusable as a Plasma layout template;
- template/package validation can fail during the build rather than silently at desktop runtime;
- Presence remains the normal UI boundary.

### Current limitations

- the Presence backend is still constructed in-process by the applet;
- adding multiple Presence applets can still create multiple backend object graphs;
- Workspace is not yet live for every direct Journal write;
- process isolation and reconnect behavior are separate future milestones.

These limitations are not contradictions of the dock decision; they are M1/M3 runtime work.

## Alternatives Considered

### Compact top-panel popup only

Rejected because the inspection surface is too dense for the normal panel role.

### Direct SQL access from QML

Rejected because the UI must talk through Presence rather than own/read cognitive persistence
directly.

### Separate applet per organ

Rejected because it fragments one presentation boundary and does not solve process ownership.
