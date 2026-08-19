# Living Canvas design QA

- Source visual truth: `docs/cybou_ui.png`
- Implementation screenshot: `docs/ui-audit-2026-08-19/04-six-panel-canvas.png`
- Combined comparison: `docs/ui-audit-2026-08-19/05-reference-vs-six-panel.png`
- Source pixels: 1536 × 1094
- Implementation pixels / browser viewport: 1280 × 720 at browser density
- Comparison normalization: both images scaled to 720 px height and placed side by side
- State: six-panel canvas with the release panel selected

## Findings

- **Passed composition slice — six-panel spatial canvas.** Artifact, collaborators, release,
  sources, commitments, and suggestion panels now occupy the same broad regions as the target.
  The command bar and supplied nebula background are integrated, and the focal release panel uses
  the stronger glass and teal selected treatment.
- **Passed relationship/navigation slice.** Five labeled relationships now follow live panel
  boundaries, including the amber dependency edge. Selecting a panel promotes only its immediate
  relationships while dimming the rest of the graph. Selected-object actions, minimap, and the
  expanded runtime/user menu are also implemented.
- **Passed content-density slice.** The artifact now exposes its filename, digest, provenance, and
  trust badges; Release exposes object type plus Target/Owner/State; Sources and Collaborators now
  carry the complete rows and footer actions visible in the source. The supplied source does not
  include reusable avatar assets, so people remain text-first instead of using fabricated faces.
- **Passed compact-desktop density slice.** At 1280 × 720 the Release action rail, Commitments,
  and command bar now have zero measured intersection area. A target-aspect comparison at
  1536 × 1094 is still required before declaring full composition parity.
- **Passed iconography slice.** Rust-native Lucide icons now cover the card headers,
  selected-object actions, command bar, minimap, and runtime-menu controls.
- **Passed interaction slice — draggable panels.** Pointer drag changed the release position from
  `(480, 140)` to `(630, 240)` and brought it forward. Reload restored `(630, 240)`. ArrowLeft moved
  it by 10 px and Shift+ArrowUp by 40 px after the focus fix. No warning/error console entries were
  observed.
- **Passed command-navigation slice.** `Ctrl/Cmd+K` opens and focuses the Rust/WASM command
  palette, text input filters the six canvas objects, Enter selects the first match, and Escape
  closes and clears the palette. Selection continues to drive the active relationship subgraph.
- **Passed cross-panel workflow slice.** Release actions now navigate to Commitments, Mind, and
  linked Sources; More opens and focuses the command palette. Mind actions route Review evidence
  to Artifact and Add to plan to Commitments. Workspace-menu commands select their corresponding
  canvas objects and close the menu.
- **Passed live-projection slice.** The frontend retains the complete typed `SnapshotProjection`
  and atomically replaces it from SSE. Runtime and system surfaces derive their projection number
  and capability availability from that snapshot rather than from static presentation copy.
- **Passed capability-inspection slice.** The live system indicator opens an accessible inspector
  listing each typed capability, its availability, freshness/knowledge context, and owner-supplied
  reason. The inspector closes from its control or Escape without changing canvas selection.

## Focused evidence

The normalized side-by-side comparison confirms that the principal panel set, background, central
hierarchy, and command bar now follow the source. It also makes the missing relationship and
navigation layers unambiguous. Focused browser checks from the draggable-panel slice remain valid
because all six panels use the same pointer-capture and keyboard-movement implementation.

## Comparison history

1. Initial browser interaction found pointer drag and reload persistence working, but pointerdown
   prevented keyboard focus.
2. The implementation now focuses the panel before pointer capture. Post-fix evidence confirms
   `focused: true`, ArrowLeft `−10 px`, Shift+ArrowUp `−40 px`, and an empty warning/error log.
3. The six-panel deployment adds the reference background, collaborators, sources, commitments,
   and command bar. A fresh public-browser screenshot has no visible clipping or overlap at the
   tested viewport.
4. The next deployed slice adds the selected release action rail, expanded amber Mind Suggestion,
   Local/Remote control, and an interactive runtime/user menu. Semantic browser QA confirms the
   menu toggles to `aria-expanded=true` and exposes all expected commands. The browser image capture
   failed repeatedly for this slice, so it is not yet promoted to a new visual comparison baseline.
5. Rust-native Lucide icons now replace the symbolic brand mark and add source-backed icons to the
   artifact, collaborators, sources, selected-object actions, and command bar. Full-width semantic
   browser QA confirms the desktop-only runtime controls and action rail remain visible. Browser
   image capture remains unavailable, so the last accepted visual baseline is still comparison 05.
6. The deployed minimap derives all six marker positions from the live `CanvasLayout`. Browser QA
   confirms its runtime-menu control hides and restores the map, and selecting the Sources marker
   applies the `selected` state to the full Sources panel. The topbar stacking context was raised so
   menu commands receive pointer input above the isolated canvas.
7. Rust-native Lucide icons now cover the remaining Release, Mind Suggestion, Commitments, and all
   runtime-menu commands. Debian Rust/WASM checks and production deployment pass. The follow-up
   semantic browser inspection timed out and reset the browser session, so this slice still relies
   on the previously validated control structure rather than a new visual baseline.
8. The deployed graph exposes five labeled SVG relationships derived from `CanvasLayout`. A real
   pointer drag moved Release from `(445, 105)` to `(525, 165)` and every connected endpoint moved
   from `(640, 230.5)` to `(720, 290.5)` in the same frame. No browser warnings or errors were
   observed. Screenshot capture still fails, so comparison 05 remains the visual baseline.
9. Relationship endpoints now intersect the panel rectangles instead of terminating at their
   centers. Selecting Sources leaves exactly one of five edges active (`validated by`); selecting
   the focal Release activates all five. The deployed browser reports no warning-level logs. A
   fresh 1237 × 720 browser capture also confirms the graph remains behind the cards with no new
   clipping or overlap, but the target-aspect normalized comparison is still pending.
10. The detailed-card deployment resets persisted layout data to the new `v2` geometry so edge
    anchors match the expanded card heights. Browser inspection confirms the artifact digest and
    badges, five collaborator rows, structured Release metadata, five Sources rows, and Sources
    footer are present. The fresh capture preserves the intended hierarchy at 1237 × 720; the
    short viewport still compresses the release action rail toward Commitments, consistent with
    the known viewport-density gap.
11. The command bar is now a working object navigator rather than a decorative input. Browser QA
    opened it with Ctrl+K, filtered `sources` to one result, selected Sources with Enter, and then
    confirmed Escape closes and clears a reopened palette. Sources became the selected panel with
    exactly one active relationship, and no browser warnings were emitted.
12. Persisted layout schema `v4` introduces a compact desktop arrangement for the six panels and
    lowers the command surface. At 1280 × 720, browser geometry reports `0 px²` intersection for
    both Release-actions/Commitments and Commitments/command-bar, with no warning-level logs. The
    follow-up image capture was unavailable, so the numerical check supplements rather than
    replaces the accepted visual baseline.
13. Browser workflow QA confirmed Plan selects Commitments, More opens the palette with its search
    field focused, Review evidence selects Artifact, Add to plan selects Commitments, and Open mind
    selects Mind Suggestion while closing the workspace menu. No warning-level logs were emitted.
14. The public gateway returned projection 42 with one available and one unavailable capability.
    The deployed UI independently rendered `Gateway · projection 42` and
    `1/2 capabilities available · projection 42`, confirming the presentation matches the typed
    live snapshot. Browser warning logs remained empty.
15. Browser QA opened the system indicator through a real pointer action and rendered two live
    rows: `mind.identity.read` as Available with Known/Current context, and
    `mind.lifecycle.command` as Unavailable with the gateway reason `authorization boundary not
    implemented`. `aria-expanded` tracked the open state, Escape removed the inspector, and no
    warnings were logged. A follow-up screenshot capture was unavailable.

## Next implementation checklist

1. Repeat browser comparison at the target aspect ratio and fix remaining P1/P2 gaps.

final result: blocked
