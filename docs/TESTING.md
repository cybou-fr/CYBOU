<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Testing Strategy

## Current CTest suites

The current Mind package runs ten suites:

```text
protocol
journal
identity
intentions
predictor
selfmodel
workspace
presence
presence-extended
m1-runtime
```

## M1 runtime invariants

`m1-runtime` specifically proves:

```text
Journal accepted event appears only after successful COMMIT
failed append never reaches the accepted stream
direct Journal append updates Workspace immediately
Workspace accepted admission is idempotent
two Presence surfaces for one state root share one runtime/session
commands from one Presence surface are visible to another
a new runtime session starts only after all old wrappers are gone
persistent root follows XDG_STATE_HOME/cybou on Unix
legacy state migrates while preserving an existing desktop marker
legacy/canonical collisions fail closed without overwriting either side
```

## Existing Journal/protocol invariants

The other suites continue to protect protocol structure, cause/evidence integrity, privacy,
Journal migration/hash integrity, domain lifecycles, Workspace behavior, and Presence projections.

## Pending runtime invariants

These belong to M3/M4 and later:

```text
eventd is the only process allowed to write journal.db
accepted events cross typed IPC without reordering
Mind remains alive after plasmashell restart
Presence reconnects to presenced
individual organ failure produces a capability deficit
process restart reconstructs owned projections
```

## Local validation

```bash
nix build .#packages.x86_64-linux.cybou-mind --print-build-logs
```

For a direct CMake build:

```bash
nix develop
cmake -S mind -B build/dev -G Ninja -DBUILD_TESTING=ON
cmake --build build/dev
ctest --test-dir build/dev --output-on-failure
```

Repository validation:

```bash
nix build --print-build-logs \
  .#checks.x86_64-linux.formatting \
  .#checks.x86_64-linux.reuse \
  .#checks.x86_64-linux.package-metadata \
  .#packages.x86_64-linux.cybou-mind \
  .#packages.x86_64-linux.cybou-presence-applet

nix fmt
git diff --exit-code
```

Full/tag validation remains:

```bash
nix flake check --print-build-logs
```

## Definition of Done

A runtime milestone is complete when the behavior is represented by code, focused tests, relevant
Nix package builds, and documentation that distinguishes Current from Target.
