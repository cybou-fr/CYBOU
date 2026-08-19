# Living Canvas visual gap audit

## Scope

Comparison of the current local Living Canvas at `http://127.0.0.1:8787/` with the three provided
visual references: the background plate, the initial concept, and the refined concept.

## Verdict

The current build preserves the correct semantic nucleus but not the intended desktop experience.
It reads as a sparse web proof: three small cards occupy the upper-left portion of a mostly empty
viewport. The refined sketch reads as a spatial operating surface: a central selected object anchors
a meaningful relationship graph, with dense supporting context, persistent navigation, command
entry, minimap, runtime status, and high-quality depth treatment.

## Evidence and steps

1. **Background plate — healthy reference asset.** The dark cosmic field, left-weighted nebula, and
   lower focal rings establish depth without competing with foreground content. The current build
   does not expose this atmosphere strongly enough.
2. **Initial concept — directionally strong, somewhat crowded.** It establishes the core desktop
   grammar: central focus, evidence/people/build/commitment satellites, semantic connectors,
   minimap, command bar, and system state. Several small labels and the bottom-center emblem compete
   for attention.
3. **Refined concept — preferred target.** It simplifies the shell, strengthens the selected central
   card, makes relationships legible, consolidates actions into the selected object, and gives the
   Mind suggestion a clear decision surface. This should become the primary visual target.
4. **Current implementation — structurally incomplete.** The central release card, evidence card,
   suggestion, local runtime, and nominal status exist, but scale, placement, density, connectors,
   navigation, minimap, command surface, background, and selected-object behavior do not match the
   target.

## Highest-impact changes

1. Rebuild the viewport composition around a responsive central focus zone, not fixed small cards.
2. Implement the refined sketch's full graph: artifact, collaborators, sources, commitments, and
   Mind suggestion with visible labeled relationships.
3. Add the persistent shell: breadcrumb/workspace navigation, Local/Remote mode, profile/runtime
   menu, minimap, command palette, and bottom-right health indicator.
4. Use the supplied background plate at intentional cover/crop, then reproduce the sketch's layered
   glass, edge highlights, teal focus glow, amber suggestion state, and depth hierarchy.
5. Make selection meaningful: one focused node owns the action rail; secondary nodes remain quieter;
   suggestion actions must expose review-before-commit semantics.
6. Preserve the good semantic baseline already visible in the DOM—main, region, complementary,
   headings, and button semantics—while adding keyboard graph navigation, strong focus rings,
   non-color state labels, and readable minimum text sizes.

## Accessibility limits

Screenshots confirm likely risks from small secondary text, low-contrast connector labels, and
color-dependent teal/amber/green states. They cannot prove keyboard order, focus visibility, zoom
reflow, screen-reader graph descriptions, target sizes, reduced motion, or contrast ratios; those
require implementation inspection and interaction testing.
