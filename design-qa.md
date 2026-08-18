# Living Canvas design QA

- Source visual truth: `docs/cybou_ui.png`
- Implementation screenshot: `docs/ui-audit-2026-08-19/02-draggable-panels-deployed.png`
- Combined comparison: `docs/ui-audit-2026-08-19/03-reference-vs-drag-slice.png`
- Source pixels: 1536 × 1094
- Implementation pixels / browser viewport: 1280 × 720 at browser density
- Comparison normalization: both images scaled to 720 px height and placed side by side
- State: release panel selected after pointer drag, reload, and keyboard movement

## Findings

- **P1 — The complete spatial desktop is still missing.** The target has artifact, collaborators,
  sources, commitments, suggestion, minimap, command bar, runtime menu, connectors, and a centered
  focal system. The implementation still has only three small panels and status chrome.
- **P1 — Viewport composition and scale differ materially.** The target fills the canvas with a
  balanced graph. The implementation clusters content in the upper-left/center and leaves most of
  the screen empty.
- **P1 — Supplied background asset is not integrated.** The target's nebula and lower focal rings
  establish the spatial field; the implementation uses only the earlier CSS atmosphere.
- **P2 — Typography and density are too small.** The implementation's header, metadata, and panels
  are significantly smaller than the target at the captured desktop viewport.
- **P2 — Visual tokens remain provisional.** Glass depth, teal selected glow, amber suggestion
  treatment, connector labels, iconography, and selected-object action rail do not yet match.
- **Passed interaction slice — draggable panels.** Pointer drag changed the release position from
  `(480, 140)` to `(630, 240)` and brought it forward. Reload restored `(630, 240)`. ArrowLeft moved
  it by 10 px and Shift+ArrowUp by 40 px after the focus fix. No warning/error console entries were
  observed.

## Focused evidence

The full comparison is sufficient to establish the P1 composition gaps. Focused browser checks were
used for the interaction slice: inline style coordinates, active-element state, reload persistence,
and console logs.

## Comparison history

1. Initial browser interaction found pointer drag and reload persistence working, but pointerdown
   prevented keyboard focus.
2. The implementation now focuses the panel before pointer capture. Post-fix evidence confirms
   `focused: true`, ArrowLeft `−10 px`, Shift+ArrowUp `−40 px`, and an empty warning/error log.

## Next implementation checklist

1. Integrate the supplied background and responsive canvas scale.
2. Add the missing draggable panels and typed content.
3. Render relationship connectors from live panel centers during drag.
4. Add selected-object actions, minimap, command bar, and runtime menu.
5. Repeat browser comparison at the target aspect ratio and fix remaining P1/P2 gaps.

final result: blocked
