<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0008: Mind Dock with Organ Tabs

## Status
Accepted

## Context
The Presence applet currently shows a subset of Mind data (narration, attention, obligations, activity). To provide full visibility into all 6 Mind organs, we need a comprehensive UI that displays data from each organ in an organized way.

The initial consideration was to add inline Panel creation in layout.js, but Panda's analysis revealed several issues:
- Inline Panel creation means a failure in the dock code breaks the entire layout
- The Plasma 6.7.3 API officially supports `loadTemplate()` for layout templates
- Using templates provides better isolation and reusability

## Decision
Implement a vertical dock panel with tabs for each Mind organ using the `loadTemplate()` approach:

1. **Extend Presence C++ API** with read-only getters for all organs:
   - `identityState()` → QVariantMap: Identity organ state
   - `calibrations()` → QVariantList: All Predictor calibrations
   - `predict(subject)` → QVariantMap: Prediction for a subject
   - `coalitions()` → QVariantList: All Workspace coalitions
   - `moment()` → QVariantMap: Current Workspace moment state

2. **Create layout-template package** (`cybou-layout-templates`):
   - Contains `org.cybou.plasma.minddock` template
   - Uses `loadTemplate()` in layout.js for better isolation
   - Template creates vertical dock with proper properties:
     - `location: "right"`
     - `hiding: "autohide"`
     - `height: 460` (width for vertical panel)
     - `lengthMode: "fill"` (stretches along full screen height)
     - `alignment: "center"`
     - `floating: true`

3. **Implement QML components**:
   - `MindDock.qml`: Main dock container with tab bar and stack
   - `MindTabBar.qml`: Tab navigation with 6 organ tabs
   - Individual tab components for each organ
   - `StatCard.qml`: Reusable component for displaying statistics

4. **Maintain architectural invariants**:
   - Presence remains the only interface the surface talks to
   - All data access goes through Presence methods
   - Fail-closed behavior: if an organ is not available, show empty/placeholder
   - QtTest coverage for all new methods

## Consequences

### Positive
- All 6 Mind organs are accessible through the UI
- Maintains architectural invariant (Presence as single interface)
- Fail-closed behavior preserved
- Better isolation with `loadTemplate()` (dock failure doesn't break entire layout)
- Official API usage (loadTemplate is documented in Plasma 6.7.3)
- Reusable template can be called from update scripts
- Extensible for future organs

### Negative
- Epic scope: C++ + QML + Nix changes
- Requires more development time
- Needs Designer input for optimal tab layout

## Alternatives Considered

### Alternative B: Tabs with Current API Only
- Only 4 working tabs (Dashboard, Intentions, Activity, Self)
- Placeholder pages for Identity, Predictor, Workspace
- Faster to implement but incomplete
- **Rejected**: Doesn't provide full visibility into Mind organs

### Alternative C: Direct SQL Access from QML
- QML accesses journal.db directly
- **Rejected**: Violates Presence.h:7 invariant ("the only class the surface talks to")

### Alternative D: Separate Applet per Organ
- Each organ has its own applet
- **Rejected**: No advantage over Solution A, organs are C++ classes anyway

## Related
- ADR-0003: AI in v0.1 - none (affirms fail-closed principle)
- ADR-0005: Calamares installer - upstream profile (similar isolation principle)
- Panda's analysis: Confirmed `loadTemplate()` is the recommended approach
