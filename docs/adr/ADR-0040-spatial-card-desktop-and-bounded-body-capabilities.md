<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0040: Spatial Card Desktop (CYBOU Desktop vNext) and Bounded Body Capabilities (CYBOU Shell)

## Status

Accepted

> **Superseded in part by [ADR-0047](ADR-0047-interactive-terminal-under-the-authenticated-account.md).** The shell
> boundary below — *Zone 3 (Shell) cannot execute arbitrary Zone 4 actions*, and the
> `DemoReadOnly` profile as the only shell surface — no longer holds for an account an
> operator has explicitly enabled a terminal for. Everything else here stands, and the
> sandboxed shell remains what a deployment serves where no terminal has been enabled.

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
- **System Cards** (12 canonical singleton projections: `Identity`, `Session`, `Capabilities`, `Journal`, `Lifecycle`, `Commitments`, `SelfModel`, `Attention`, `Beliefs`, `Perception`, `Context`, `Disclosure`). System cards are singleton, movable, resizable, collapsible, deckable, and cannot be destroyed.
- **Tool Cards** (e.g. `CYBOU Shell`, Inspector, Debugger). Ephemeral or multi-instance, closable, resizable, bounded to specific capability profiles.
- **Ephemeral Cards** (e.g. search previews, temporary diffs, inspection overlays). Transient life-cycle, closable, discardable.

Each card type is governed by a static `CardSpec` defining bounds (`default_size`, `min_size`, `max_size`) and capabilities (`movable`, `resizable`, `collapsible`, `closable`, `deckable`).

### 2. Layout Schema v9 and Self-Healing Startup Pass

The browser storage schema is upgraded to `cybou.desktop.layout.v9`:
- `DesktopLayout` holds `schema_version: 9`, `cards: Vec<CardInstance>`, and `decks: Vec<DeckInstance>`.
- **Seamless Backward Compatibility**: When `v9` is absent in browser `localStorage`, `DesktopLayout::load()` inspects `cybou.living-canvas.layout.v8`, imports exact coordinate offsets, applies default `CardSpec` dimensions and uncollapsed presentation, persists to `v9`, and continues without user interruption.
- **Invariant-Safe Startup Normalization (`validate_and_normalize`)**:
  - Automatically instantiates default geometries for any missing canonical system cards.
  - Clamps dimensions to `[min_size, max_size]` and bounds spatial coordinates within visible/reachable limits.
  - Enforces deck integrity: dissolves decks with `< 2` cards, resolves cross-deck card conflicts, and normalizes z-order monotonically.

### 3. Spatial Dynamics, Focus Mode, and Deck Composition

CYBOU Desktop provides spatial freedom combined with deterministic structure:
- **Interactive & Keyboard Resize**: Geometry clamps to `CardSpec.min_size` and `CardSpec.max_size`. Relationship lines continuously track dynamic card boundaries using center-to-edge anchor projection. Supports `Alt+Shift+Arrow` keyboard resize.
- **Accessible Spatial Movement**: `Alt+Arrow` moves cards, preserving standard Arrow keys for terminal and form input.
- **Collapse / Expand**: Cards can collapse into single-line summary pills to save canvas real estate while preserving presence.
- **Pinning**: Pinned cards (`pinned: true`) are locked against auto-arrangement algorithms.
- **Non-Destructive Focus Mode**: Focus expands cards to fill the active viewport without mutating underlying persisted spatial coordinates; `Escape` cleanly restores the previous desktop state.
- **Arrangement Modes**: Pure, deterministic function `arrange(mode, cards, relationships, bounds)` supporting `Free`, `Compact`, `Grid`, `Relations`, and `Home`. Focus is not an arrangement — see **Amendment 2**.
- **Invariant-Safe Decks**: Tabbed grouping governed by typed `DeckError` invariants, ensuring minimum 2 cards per deck, no duplicate cards, and WAI-ARIA `role="tablist"` keyboard traversal (`ArrowLeft`/`ArrowRight`/`Home`/`End`).

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
│  Builtins only; the accepted set is in Amendment 1 below.   │
└─────────────────────────────────────────────────────────────┘
```

#### Shell Invariants:
1. **Builtins only**: Only the set named in **Amendment 1** is recognized; anything else exits 127. No fork/exec of `/usr/bin/*`, no pipelines (`|`), no shell expansions, no redirection (`>`, `<`), no subshells. The invariant is that the set is closed and enumerated here, not that it has any particular size.
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

---

## Amendment 1 (2026-08-22): the accepted builtin set, and two capabilities withdrawn

This decision named six builtins and said only those were recognized. The implementation had grown
to thirteen, `CURRENT_STATE.md` described thirteen, and `TESTING.md` still described six. Three
documents and one program held three different answers, and the Accepted decision was the one
nobody had changed.

The growth did not weaken the sandbox — there is still no `exec`, no pipeline, no redirection, and
every path goes through `cybou-jailfs`. It happened without amending the decision, which is its own
failure and is what this amendment closes.

Two of the thirteen are withdrawn rather than accepted, because they could only answer with
something nobody established:

| Withdrawn | What it did | Why |
|---|---|---|
| `whoami` | printed `cybou` | A constant. It named no account, and there was no account it was naming. |
| `uname` | printed a fixed kernel string | A constant. It reported a kernel version, architecture and hostname that were compiled in, on every host. |

In a terminal both read as observations of the Body. They were not observations of anything. A
bounded surface may be small; nothing it prints may be invented.

Two more were printing invented fields and now print fewer, truthful ones:

- `ls -l` reported `-rwxr-xr-x 1 cybou cybou` for every entry — a mode nobody read and an owner
  nobody looked up — and `4096` for every directory. It now reports type, size and name, and a
  directory's size is a dash because the sandbox does not establish it.
- `stat` reported `Access: (0644/-rw-r--r--)` for everything, including directories. The line is
  gone. File, size and type remain because those come from a real `metadata` call.

**The accepted set was therefore eleven, all read-only** — see Amendment 4, which extends it:

```text
help  pwd  ls  cd  cat  echo  stat  head  tail  grep  clear
```

Each earns its place the same way: it reads bytes or paths that `cybou-jailfs` already resolved
inside the sandbox root, or it manipulates only text the caller supplied. None of them observes the
host, names a principal, or reports a property of the machine.

Extending this set again requires amending this document in the same commit as the code.

## Amendment 2 (2026-08-22): Focus is a view mode, not an arrangement

This decision listed `Focus` among the arrangement modes. The implementation separates the two, and
the separation is better than the original text:

```text
ArrangementMode :: Free | Grid | Compact | Relations | Home
DesktopViewMode :: Spatial | Focus
```

An arrangement computes persisted geometry. Focus does not: it fills the viewport without mutating
the coordinates underneath, and `Escape` restores what was there. Calling both "modes" of the same
thing made a non-destructive view look like a destructive rearrangement.

## Amendment 3 (2026-08-22): twelve system cards

`Disclosure` was added as a twelfth canonical system card, and is the one card that is not a
projection of an organ: it shows what the reader in front of it was supplied and what was kept from
them (ADR-0030 B1, B6).

## Amendment 4 (2026-08-22): five more read-only capabilities, and a prompt that stopped pretending

The accepted set grows to sixteen. Each addition was checked against the same question Amendment 1
withdrew two commands for — *can this answer with something nobody established?* — and each answers
only from what `cybou-jailfs` read or what the host clock says:

| Added | What it answers from | Why it cannot invent |
|---|---|---|
| `wc` | the bytes of one file | counts of what was read |
| `du` | the sizes the sandbox reported | a sum, and a statement when the sum is partial |
| `file` | whether it is a directory, and whether the bytes are UTF-8 | two checks, two answers, no format guessed from a name or a magic number |
| `find` | the directory listing, to a bounded depth | paths that exist, and a statement when the walk stopped early |
| `date` | the host clock, in UTC | a real reading; a compiled-in instant would be the fault `uname` was withdrawn for |

`du` and `find` walk, so they are bounded — eight levels, two thousand entries. **A bounded answer
always says it is bounded.** A total that stopped adding is not a smaller directory and a listing
that stopped is not a shorter one, and a surface that presented either as complete would be stating
what it had not established.

The prompt also changed. It read `cybou:/path ›`, which has the shape of `user@host:path` and named
neither: there is no account called `cybou` being reported and no host being named. It is the path
alone now. Removing `whoami` while leaving a prompt that implied the same answer would have been
half a decision.

**The accepted set is sixteen:**

```text
help  pwd  ls  cd  cat  echo  stat  head  tail  grep  wc  du  file  find  date  clear
```

Extending it still requires amending this document in the same commit as the code.
