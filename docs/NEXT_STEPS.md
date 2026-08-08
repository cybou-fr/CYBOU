<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Next Engineering Steps

## Purpose

This is the executable plan after the M4 substrate and the M5–M9 architecture update. It translates
the capability roadmap into reviewable work packages. [Roadmap](ROADMAP.md) remains the milestone
definition; [Current State](CURRENT_STATE.md) remains the implementation authority.

The immediate objective is not language or autonomous action. It is:

> establish a green baseline, then prove that one identity and its commitments survive lifecycle
> transitions while bounded consolidation remains interruptible, evidence-linked, and owner-safe.

## Sequencing rules

1. A package starts only when its prerequisite gate is green.
2. Protocol/schema changes land before dependent UI behavior.
3. Every persistent change includes migration, interruption, and recovery tests.
4. `CURRENT_STATE.md` advances only with demonstrated implementation.
5. M8 language and M9 action work do not begin before the M5/M6 exit gates.

## P0 — Restore a trustworthy green baseline

**Status: complete.** The fast Nix gate, all four repository-specific validators, REUSE 3.3, both
primary packages, and the twelve Mind CTest suites pass from the complete working tree.

### Work

- remove the tracked Python bytecode cache and ignore `__pycache__/` and `*.pyc`;
- add REUSE/SPDX metadata for the Mind handle `metadata.json`;
- format `packages/cybou-layout-templates/default.nix`;
- expose the cognitive-doc, Mind-access, QML-API, and UI-polish validators as flake checks;
- run those checks in the fast GitHub workflow;
- make documentation link validation part of the same canonical gate;
- record one clean fast-check command in README/BUILDING/CI.

### Exit gate

```bash
nix build --print-build-logs \
  .#checks.x86_64-linux.formatting \
  .#checks.x86_64-linux.reuse \
  .#checks.x86_64-linux.package-metadata \
  .#checks.x86_64-linux.cognitive-docs \
  .#checks.x86_64-linux.mind-access \
  .#checks.x86_64-linux.qml-api \
  .#checks.x86_64-linux.ui-polish \
  .#packages.x86_64-linux.cybou-mind \
  .#packages.x86_64-linux.cybou-presence-applet

nix fmt
git diff --exit-code
```

All repository-specific validators also run from a flake check and pass. No M5 implementation
starts from a knowingly red main branch.

## P1 — Freeze the M5 lifecycle contract

**Status: complete.** ADR-0026 selects the lifecycle owner and state roots; lifecycle schema v1,
transition legality, CBOR encoding, fail-closed validation, and focused protocol tests are present.

### Work

- decide whether lifecycle coordination is a new process or a narrow responsibility attached to an
  existing service; record the decision in an implementation ADR;
- define lifecycle/run wire types, mode-transition legality, and error vocabulary;
- define the persistent/runtime state locations and single owner of run state;
- define accepted high-water-mark semantics;
- define operation/idempotency keys and terminal outcome rules;
- specify concurrency: one active run, compatible shallow tasks, or explicit serialization;
- update protocol, IPC, ownership, failure, and threat-model documents together.

### Required artifacts

- accepted implementation ADR;
- versioned protocol schema/API;
- transition table including illegal transitions;
- state migration/rollback statement;
- focused codec and transition tests.

### Exit gate

The contract can represent requested, active, completed, interrupted, failed, recovering, and
degraded runs without relying on UI strings or direct database access.

## P2 — Prove continuity before consolidation

### Work

- build a restart/reboot/logout/upgrade transition test matrix;
- persist stable run/session transition records;
- verify identity and open intention reconstruction;
- distinguish clean suspension, abrupt process death, incomplete migration, and corrupt state;
- add backup/restore verification around persistent-state migration;
- expose freshness and last-verified transition in a typed projection.

### Test matrix

| Transition | Identity | Intentions | Journal | Expected mode |
|---|---|---|---|---|
| organ restart | unchanged | reconstructed | verified | `Awake` or `Degraded` |
| presenced/QML restart | unchanged | unchanged | unchanged | current mode |
| logout/login | same subject, new session | reconstructed | verified | `Recovering → Awake` |
| reboot | same subject | reconstructed | verified | `Recovering → Awake` |
| supported upgrade | explicit transition | migrated/restored | verified | `Maintenance → Recovering → Awake` |
| failed migration | no invented continuity | preserved backup | failure recorded | `Degraded` |

### Exit gate

M5 continuity tests demonstrate that process and system transitions cannot silently create a new
identity, lose accepted commitments, or report unverified success.

## P3 — Implement the consolidation MVP

### Vertical slice

Implement one small end-to-end run:

```text
Idle policy or explicit request
→ ConsolidationRequested
→ accepted high-water mark
→ predictord calibration work
→ workspaced salience/episode maintenance
→ accepted owner results
→ ConsolidationCompleted | Interrupted | Failed
→ Presence projection
```

### Constraints

- the coordinator never opens Journal or owner storage;
- owner work cites accepted input evidence;
- new observations after the high-water mark stay outside the run;
- interruption followed by retry cannot duplicate calibration or maintenance effects;
- a missing optional owner produces an explicit degraded result;
- only an accepted terminal contribution permits `completed` presentation.

### Exit gate

The VM test interrupts the run at every persistence boundary, restarts the affected process, and
observes one correct terminal result with intact identity and biography.

## P4 — Make lifecycle visible without moving ownership into UI

### Work

- extend Presence1 with lifecycle mode, run status, progress class, freshness, and deficits;
- replace blocking lifecycle interactions with asynchronous calls;
- show `Idle`, `Consolidating`, `Recovering`, and `Degraded` distinctly;
- provide user interruption where policy permits;
- keep the existing shell usable when lifecycle services are absent;
- add QML static validation and VM interaction assertions.

### Exit gate

Destroying/recreating Plasma or the Presence proxy neither changes lifecycle state nor duplicates a
run. A timeout cannot freeze the shell for the current five-second blocking RPC window.

## P5 — Close M5 and publish evidence

### Work

- run the full package, process-integration, and VM-smoke suite;
- build the development VM and ISO from a clean revision;
- document supported and unsupported transition paths;
- update `CURRENT_STATE.md` from M4 to the demonstrated M5 subset;
- produce release notes and hashes for an evaluation build;
- keep unfinished retention/epistemic work labelled M7.

### M5 exit gate

- continuity matrix passes;
- consolidation interruption/recovery matrix passes;
- no duplicate owner or direct Journal write path exists;
- lifecycle capability can degrade without erasing identity/biography;
- docs, tests, and shipped UI describe the same behavior.

## P6 — Begin M6 only after M5

Order M6 work as:

1. explicit capability matrix and dependency graph;
2. asynchronous RPC plus retry/backoff/circuit-breaker policy;
3. typed health, freshness, backlog, latency, storage, and calibration-pressure metrics;
4. homeostatic scheduling rules;
5. metacognitive projection of unknown/stale/assumed/unsupported state;
6. degraded-mode UI and process/VM fault-injection tests.

M6 exits when loss of every optional organ has an explicit capability deficit, useful remaining
behavior, recovery rule, and test.

## Deferred behind gates

### M7

Start with one provenance-bearing system perception adapter and one epistemic contradiction slice.
Do not begin multi-node transport until retention, privacy, freshness, and erasure semantics are
testable locally.

### M8

Attach a language faculty only to selected typed context carrying provenance, epistemic status,
freshness, privacy, and deficits. Model absence/replacement must be an acceptance test.

### M9

No privileged executor work begins until proposal, criticism, value constraints, authorization,
typed capability, observation, outcome, and rollback boundaries are independently represented.

## Suggested PR decomposition

| PR | Scope |
|---|---|
| 1 | P0 repository hygiene and green gates |
| 2 | lifecycle implementation ADR and transition protocol |
| 3 | lifecycle codec/state-machine unit tests |
| 4 | transition/run owner and IPC service |
| 5 | continuity reconstruction and integration matrix |
| 6 | consolidation MVP owner requests |
| 7 | interruption/idempotency/fault-injection tests |
| 8 | asynchronous Presence lifecycle projection |
| 9 | VM smoke, docs, and M5 evaluation release |

Keep migrations separate from UI polish and avoid combining M6 capability policy with the first M5
vertical slice.

## Definition of done for every package

- explicit owner and non-owner list;
- typed success, failure, interruption, and degraded outcomes;
- persistence and migration statement;
- privacy/provenance/retention impact;
- unit plus process/VM test proportional to the boundary;
- updated contracts and `CURRENT_STATE.md` when behavior is implemented;
- reproducible Nix gate from a clean tree.
