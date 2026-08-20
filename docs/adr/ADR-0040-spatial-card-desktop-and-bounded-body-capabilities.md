<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0040: Spatial Card Desktop (CYBOU Desktop vNext) and Bounded Body Capabilities (CYBOU Shell)

## Status

Accepted

## Context

ADR-0037 established a unified, web-first Presence architecture delivered via `cybou-web-gateway` as a Rust/WASM application. The initial implementation presented 11 hardcoded system panels in a fixed-coordinate layout schema (v8).

While this proved process-isolated Mind aggregation, snapshot consistency, and live SSE streaming, it treated the desktop canvas as a rigid status monitor rather than an extensible agent-native workspace. Furthermore, users interacting with the host system needed a direct, bounded mechanism to inspect host state without breaking process isolation, capability governance, or host security invariants.

We require an architectural evolution that:
1. Replaces the fixed 11-panel structure with an extensible, generic **Card** and **Deck** model.
2. Establishes a spatial layout engine supporting user-driven geometry, interactive resize, collapse/expand, pinning, and deterministic multi-mode arrangement.
3. Introduces **CYBOU Shell** as the first bounded capability surface to the host (**Body**), strictly isolated from arbitrary command execution, unrestricted filesystem access, and remote public exposure.
4. Formally separates Mind projection, Desktop presentation, bounded Body capabilities, and governed action execution across four disjoint security zones.

## The Core Formula

```text
Debian 13       = Body (host execution, kernel, storage)
CYBOU Mind      = continuity + cognition + governance (canonical owner)
CYBOU Desktop   = Presence (spatial interactive surface)

Card            = primary interactive surface
Deck            = presentation composition (tabs), not identity
Relationship    = semantic system causality, not physical proximity
Arrangement     = deterministic spatial presentation, not cognition
Desktop Map     = spatial navigation and cluster overview
Ctrl+K          = Desktop command palette (Desktop command ≠ Body command)
CYBOU Shell     = bounded Body capability exploration (typed capability, not arbitrary execution)
Desktop state   ≠ biography (DOM/localStorage ≠ truth)
Public preview  = no Shell capability (strict boundary)
```

---

## Decision

### 1. Unified Surface Model: Generic Cards

Every visible surface in CYBOU Desktop is an instance of a **Card**:

```rust
pub struct CardInstance {
    pub id: CardId,
    pub geometry: CardGeometry,
    pub presentation: CardPresentation,
}

pub struct CardGeometry {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub z: u32,
}

pub struct CardPresentation {
    pub collapsed: bool,
    pub pinned: bool,
}
```

Cards are classified into three architectural categories (`CardKind`):
- **System Cards** (11 canonical singleton projections: `Identity`, `Session`, `Capabilities`, `Journal`, `Lifecycle`, `Commitments`, `SelfModel`, `Attention`, `Beliefs`, `Perception`, `Context`). System cards are singleton, movable, resizable, collapsible, deckable, and cannot be destroyed.
- **Tool Cards** (e.g. `CYBOU Shell`, Inspector, Debugger). Ephemeral or multi-instance, closable, resizable, bounded to specific capability profiles.
- **Ephemeral Cards** (e.g. search previews, temporary diffs, inspection overlays). Transient life-cycle, closable, discardable.

Each card type is governed by a static `CardSpec` defining bounds (`default_size`, `min_size`, `max_size`) and capabilities (`movable`, `resizable`, `collapsible`, `closable`, `deckable`).

### 2. Layout Schema v9 and Transparent Migration

The browser storage schema is upgraded to `cybou.desktop.layout.v9`:
- `DesktopLayout` holds `schema_version: 9` and `cards: Vec<CardInstance>`.
- Seamless backward compatibility: When `v9` is absent in browser `localStorage`, `DesktopLayout::load()` inspects `cybou.living-canvas.layout.v8`, imports exact coordinate offsets, applies default `CardSpec` dimensions and uncollapsed presentation, persists to `v9`, and continues without user interruption.

### 3. Spatial Dynamics and Arrangement Engine

CYBOU Desktop provides spatial freedom combined with deterministic structure:
- **Interactive Resize**: Geometry clamps to `CardSpec.min_size` and `CardSpec.max_size`. Relationship lines continuously track dynamic card boundaries using center-to-edge anchor projection.
- **Collapse / Expand**: Cards can collapse into single-line summary pills to save canvas real estate while preserving presence.
- **Pinning**: Pinned cards (`pinned: true`) are locked against auto-arrangement algorithms.
- **Arrangement Modes**: Pure, deterministic function `arrange(mode, cards, relationships, bounds)` supporting:
  - `Free`: Unconstrained spatial dragging.
  - `Compact`: Removes dead space while preserving topological intent.
  - `Grid`: Structured modular alignment.
  - `Relations`: Force-directed graph positioning driven by actual causal system relationships (`writes to`, `evaluates`, `consolidates into`).
  - `Focus`: Radial focus on a selected card with related cards placed in orbit.
- **Decks**: Tabbed grouping allowing multiple cards to share one window frame without merging or mutating underlying card identities.

### 4. Bounded Body Capability: CYBOU Shell

CYBOU Shell is **not** an unrestricted terminal emulator, shell launcher, or `/bin/sh` process wrapper. It is a strictly typed, sandboxed capability surface to the Debian 13 host (Body):

```text
┌─────────────────────────────────────────────────────────────┐
│                    CYBOU Desktop (WASM)                     │
│               CardInstance { id: Shell(1) }                 │
└──────────────────────────────┬──────────────────────────────┘
                               │ JSON-RPC / WebSocket (Local only)
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                      cybou-web-gateway                      │
│         Refuses Shell in PublicPreview / Remote modes       │
└──────────────────────────────┬──────────────────────────────┘
                               │ D-Bus (org.cybou.Body.Shell1)
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                        cybou-shelld                         │
│       Dedicated unprivileged daemon (NoNewPrivileges=yes)   │
│              cybou-jailfs (RESOLVE_BENEATH)                 │
│                 DemoReadOnly -> /home/demo                  │
│       Builtins only: help, pwd, ls, cd, cat, clear          │
└─────────────────────────────────────────────────────────────┘
```

#### Shell Invariants:
1. **Builtins only**: Only `help`, `pwd`, `ls`, `cd`, `cat`, `clear` are recognized. No fork/exec of `/usr/bin/*`, no pipelines (`|`), no shell expansions, no redirection (`>`, `<`), no subshells.
2. **Filesystem Jail (`cybou-jailfs`)**: All filesystem operations use fd-relative lookup (`openat2` with `RESOLVE_BENEATH` on Linux), strict path canonicalization, no traversal beyond jail root, and bounded read budgets (max file size, max directory entries).
3. **Session Profile**: Default profile is `DemoReadOnly` rooted at `/home/demo`.
4. **No Public Preview Exposure**: `cybou-web-gateway` strictly omits and refuses shell capability endpoints when running under `PublicPreview` mode. Shell capabilities are available exclusively to authenticated `LocalDesktop` sessions.

### 5. Four Isolated Security Zones

```text
┌─────────────────────────────────────────────────────────────────────────┐
│ Zone 1: Mind Projection (Read-only aggregation of canonical owners)    │
├─────────────────────────────────────────────────────────────────────────┤
│ Zone 2: Desktop Presentation (Card geometry, decks, collapse, pinning)  │
├─────────────────────────────────────────────────────────────────────────┤
│ Zone 3: Bounded Body Capabilities (CYBOU Shell, cybou-jailfs, shelld)   │
├─────────────────────────────────────────────────────────────────────────┤
│ Zone 4: Governed Actions (Future authorized mutation/execution runtime) │
└─────────────────────────────────────────────────────────────────────────┘
```

Cross-zone rules:
- Zone 2 (DOM/localStorage) can never modify Zone 1 (Mind) or Zone 3 (Body).
- Zone 3 (Shell) cannot execute arbitrary Zone 4 actions.
- Zone 4 (Actions) require explicit Mind authorization and Journal commitment.

---

## Consequences

### Positive
- **Extensible spatial environment**: New tools, inspectors, and capabilities integrate cleanly as Cards without altering core layout infrastructure.
- **Deterministic state recovery**: Layout v9 transparently loads legacy v8 positions while giving users full control over resize, collapse, and pinning.
- **Safe host exploration**: Users can safely navigate and inspect host files via CYBOU Shell without risk of accidental system corruption or unauthorized execution.
- **Clear security boundary**: Public web previews remain completely protected against host access.

### Negative / Trade-offs
- Additional daemon (`cybou-shelld`) and crate (`cybou-jailfs`) to maintain.
- Bounded shell syntax requires explicit documentation for users expecting full bash/zsh capabilities.
