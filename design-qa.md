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
- **P1 — Relationship and navigation layers remain missing.** The target still has live connectors,
  connector labels, selected-object actions, a minimap, and the expanded runtime/user menu.
- **P2 — Panel content fidelity is incomplete.** The artifact card, focal release metadata, and
  suggestion card need the detailed controls, badges, ownership data, and amber action treatment
  shown in the source.
- **P2 — Viewport density still differs.** At the 1280 × 720 browser viewport the implementation is
  more horizontally spread and vertically compressed than the 1536 × 1094 source composition.
- **P2 — Iconography is not integrated yet.** Real library or source icons are still required for
  the card headers, action rail, command bar, and status/menu controls.
- **Passed interaction slice — draggable panels.** Pointer drag changed the release position from
  `(480, 140)` to `(630, 240)` and brought it forward. Reload restored `(630, 240)`. ArrowLeft moved
  it by 10 px and Shift+ArrowUp by 40 px after the focus fix. No warning/error console entries were
  observed.

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

## Next implementation checklist

1. Render relationship connectors from live panel centers during drag.
2. Add the minimap.
3. Integrate real icon assets for the card headers, action rail, command bar, and runtime menu.
4. Repeat browser comparison at the target aspect ratio and fix remaining P1/P2 gaps.

final result: blocked
