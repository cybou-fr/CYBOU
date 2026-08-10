<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Project Checkpoint — 2026-08-10

## Checkpoint identity

| Field | Value |
|---|---|
| Analysed commit | `8857d32038f10892e718f7095da6b3f4207d6687` |
| Accepted capability boundary | M1–M6 plus P6.7 resilience hardening |
| Next milestone | M7 grounded perception and epistemic governance |
| Product maturity | research-grade pre-alpha; not production or stable-release qualified |
| Platform boundary | `x86_64-linux`, NixOS 26.05, KDE Plasma 6 Wayland |
| Analysis date | 2026-08-10 |

This document is a point-in-time engineering assessment. It records what was observed at the
commit above, the evidence supporting the assessment, and the recommended direction from that
state. [Current State](CURRENT_STATE.md) remains authoritative for later implementation changes;
[Next Engineering Steps](NEXT_STEPS.md) remains the live execution plan.

## Executive assessment

Cybou has moved beyond a visual NixOS customization and now contains a coherent, independently
testable cognitive-runtime substrate. Its strongest property is not intelligence: it is explicit
ownership. Biography, identity, commitments, prediction, self projection, attention, health,
lifecycle, and presentation have named owners and typed boundaries. The design consistently keeps
UI, models, authorization, and execution from becoming accidental authorities.

The M1–M6/P6.7 substrate is unusually strong for a pre-alpha project in four areas:

1. reproducible packaging and system composition;
2. durable-before-visible event semantics with one Journal writer;
3. process, restart, reboot, timeout, and split-commit fault evidence;
4. explicit current-versus-future documentation boundaries.

The project is not yet a generally useful cognitive system. It has no grounded perception owner,
epistemic projection, governed retention/erasure, inter-node continuity, language faculty, or
authorized executor. Same-user D-Bus is an architectural boundary but not a strong security
boundary. Release qualification, physical installation, upgrades across architecture changes,
and recovery from arbitrary state corruption remain incomplete.

The recommended next move is therefore narrow: implement one read-only, provenance-bearing local
system observation from acquisition through Event1 acceptance to an epistemic projection and
Presence display. Do not start distributed replication, language integration, or privileged action
in parallel with that slice.

## Quantitative repository snapshot

The following counts were generated from tracked files at the analysed commit. Generated build
trees and `result*` links are excluded.

| Measure | Observed value |
|---|---:|
| Tracked source/document files in measured extensions | 229 |
| Measured lines | 31,272 |
| C++ implementation/test files | 62 `.cpp` + 36 `.h` |
| C++ lines | 15,625 |
| Nix files / lines | 25 / 2,120 |
| QML files / lines | 17 / 1,896 |
| Markdown files / lines | 63 / 6,800 |
| Website HTML/CSS/JS lines | 2,718 |
| Mind CTest suites | 20 |
| NixOS VM/KVM gates | 4 |
| Mind D-Bus/systemd processes | 9 |
| Versioned Mind D-Bus interfaces | 9 |
| ADRs | 26: 17 Accepted, 9 Proposed |
| Flake checks | 11 |
| Fast CI checks | 7 plus two package builds and formatter-diff verification |

Lines of code are descriptive, not a quality target. The important observation is distribution:
roughly half of the measured tree is Mind C++ and tests, while documentation is large enough to be
an architectural control surface rather than an afterthought.

## System decomposition

### Reproducible Body

The Body is expressed through locked Nix inputs, reusable modules, separate packages, and three
NixOS compositions:

- QEMU/KVM development VM;
- live ISO/installer composition;
- Hyper-V development image.

Horizon colors, wallpaper, assets, Global Theme, Plasma style, Aurorae decoration, layout,
Presence applets, Mind runtime, and tools retain independent package boundaries. This makes visual
or runtime failures attributable and prevents a theme rebuild from being the only validation unit.

Current limitations:

- only `x86_64-linux` is exposed;
- ISO construction and physical installation are not routine hosted-CI gates;
- hardware coverage is not a release matrix;
- upgrade and rollback qualification is narrower than NixOS generation rollback in general;
- there is no published stable release contract.

### Typed Mind runtime

The current user-session topology is:

```text
cybou-eventd       canonical Journal and Event1
cybou-healthd      capability graph, health, deficits, measurements
cybou-lifecycled   lifecycle, run state, scheduling, recovery
cybou-identityd    persistent identity and logical-session continuity
cybou-intentiond   commitment operations and projection
cybou-predictord   observation, prediction, settlement, calibration
cybou-selfd        derived self projection and assessment
cybou-workspaced   bounded reconstructible attention
cybou-presenced    presentation aggregation and command routing
```

All nine executables are installed with D-Bus activation metadata and `systemd --user`
`Type=dbus` services. `Wants` and `After` express startup ordering without turning optional-owner
failure into teardown of the whole process graph. Plasma contains a QML proxy/cache, not local
copies of domain owners.

### Durable state and ordering

The canonical biography is a SQLite Journal owned only by `cybou-eventd`. Event1 validates and
serializes accepted envelopes, maintains canonical hashes, and publishes acceptance only after
commit. Identity, Health1, and Lifecycle1 have explicit persistent stores with version handling;
Workspace is bounded and reconstructible.

The critical ordering is implemented and tested:

```text
command
→ owning process
→ Event1 proposal
→ Journal commit
→ Event1 Accepted
→ Workspace admission
→ Presence projection
→ QML cache
```

This is a meaningful architectural achievement: a UI refresh cannot become evidence that a fact
was durably accepted.

### Lifecycle and degraded operation

Lifecycle1 owns persistent mode and run state, deterministic operation keys, owner dispatch,
terminal outcomes, interruption, recovery, cooldown arbitration, and evidence-bound automatic
scheduling. Health1 owns observations and capability derivation but cannot start lifecycle work.

P6.7 closes sequential timeout multiplication at the presentation boundary. Every compound
Presence read or mutation consumes one monotonic budget. An unresponsive registered owner can
therefore produce bounded partial data or a typed failure without extending latency once per
downstream call.

## Evidence and quality assessment

### Test hierarchy

The test strategy has four useful layers:

| Layer | Evidence |
|---|---|
| Protocol/unit | schema validation, transitions, persistence rollback, hashing, codec rules |
| Isolated process | real daemons under private D-Bus, duplicate ownership, restart, timeouts |
| Headless NixOS | reboot continuity and split-commit recovery |
| Plasma/KVM | shell recreation, D-Bus activation, required-owner loss, bounded recovery |

The `cybou-mind` package runs all 20 CTest suites during its Nix build. Four separate NixOS gates
cover the general VM smoke path, lifecycle continuity, Plasma lifecycle recreation, and the M6
recovery boundary. Fault injection includes process stop/restart, suspension while a D-Bus name
remains registered, deliberate delays, two lifecycle split-commit windows, scheduled-run crash,
late replies, Plasma recreation, and reboot.

### What the evidence proves well

- one canonical Journal writer and durable acceptance;
- nine distinct process identities and D-Bus owners;
- identity and lifecycle continuity across supported restarts/reboots;
- idempotent recovery across the tested split-commit windows;
- capability-specific degradation for tested optional owners;
- fail-closed mutation when Event1 is unavailable;
- responsive async paths and bounded compound Presence operations;
- lifecycle state remains separate from runtime reachability and capability health;
- QML recreation does not create a second Mind.

### What the evidence does not prove

- arbitrary corruption recovery or disaster recovery from backups;
- long-duration load, resource leakage, database growth, or latency under large histories;
- race freedom beyond exercised deterministic scenarios;
- hostile same-user D-Bus callers;
- hardware compatibility or installer safety across representative machines;
- in-place upgrades across multiple released schema/application versions;
- privacy erasure from source, derived data, backups, and future replicas;
- M7–M9 behavior.

## Architecture maturity matrix

Scale: 0 absent, 1 concept, 2 prototype, 3 implemented with focused evidence, 4 release-qualified,
5 production-proven.

| Area | Score | Assessment |
|---|---:|---|
| Reproducible build/package graph | 3 | Locked flake, separate derivations, fast gates; not multi-platform/release-proven |
| Desktop integration | 3 | Plasma package, layout, access, UI validators and VM evidence |
| Canonical memory/Event1 | 3 | Single writer, hashing, migration and process evidence; no external trust anchor |
| Process ownership/IPC | 3 | Nine typed owners and activation; same-user authorization remains weak |
| Identity continuity | 3 | Restart/login/reboot semantics tested; multi-version reconciliation incomplete |
| Lifecycle/consolidation | 3 | Persistent owner, recovery, dispatch and scheduling tested |
| Degraded operation | 3 | Capability graph and representative fault matrix; not every component permutation |
| RPC resilience | 3 | Typed outcomes, retry policy, circuit behavior and shared deadlines tested |
| Security hardening | 2 | Clear boundaries and fail-closed paths; limited caller authorization/sandbox hardening |
| Privacy governance | 2 | Classification and inheritance exist; retention and erasure are designs only |
| Grounded epistemics | 1 | ADR/contract direction; no implemented perception or epistemic owner |
| Distributed continuity | 1 | Architecture direction only |
| Language faculty | 1 | Boundary and ADR only; deliberately no model runtime |
| Authorized action | 1 | Boundary and ADR only; deliberately no privileged executor |
| Release/operations | 2 | Build/release procedures exist; no stable release and incomplete artifact qualification |

No area is rated 4 because the project has not yet demonstrated a versioned public release with a
supported upgrade window, repeatable artifact provenance, and an operational compatibility matrix.

## Strengths worth preserving

### Ownership before capability

New behavior is required to name its owner, non-owners, persistence, failure behavior, and wire
contract. This is the central architectural advantage and should remain the admission rule for M7.

### Models are optional faculties

The absence of an LLM does not make the current system structurally incomplete. Identity, memory,
attention, lifecycle, health, and projection work through typed state. A future model can be
replaced or disabled without becoming a new identity or memory authority.

### Failure evidence is part of feature completion

The project treats restart, timeout, late reply, partial availability, and reboot as acceptance
behavior rather than post-feature hardening. This is appropriate for persistent personal state.

### Documentation has explicit authority

Accepted ADRs and enforced schemas outrank Current State; Current State outranks target models and
roadmaps; website text is non-normative. The documentation validator checks links and important
contract phrases. This substantially reduces accidental claim drift.

## Risk register

| Priority | Risk | Evidence / impact | Required response |
|---|---|---|---|
| P0 | No M7 epistemic owner or retention authority | New observations could become durable without governed freshness, contradiction, or erasure semantics | Freeze owner/state/retention ADR before implementation |
| P0 | Same-user D-Bus is not a capability-security boundary | Another user-session process may call mutation interfaces | Define caller/authentication policy before sensitive perception or M9 work |
| P0 | No performance envelope for Journal and compound projections | Correctness tests may hide growth-driven latency/resource failure | Add deterministic scale fixtures and budgets before expanding event volume |
| P1 | ADR status lags implementation in several historical decisions | 9 ADRs remain Proposed, including lifecycle direction and some implemented semantics | Audit each Proposed ADR: accept, supersede, split, or keep explicitly future |
| P1 | Migration evidence covers a narrow version set | Long-lived personal state can outlive current compatibility assumptions | Publish a schema compatibility matrix and multi-hop upgrade tests |
| P1 | VM/ISO release evidence is mostly local/tag-time | Artifact confidence depends on KVM-capable infrastructure and manual ISO work | Record clean-tree artifact provenance and establish a release candidate gate |
| P1 | systemd isolation is process separation, not strong sandboxing | Services have restart policy but few explicit hardening directives | Threat-model filesystem/network access and add justified unit hardening |
| P1 | Health tests sample representative failures, not all graph permutations | Dependency mistakes can leave commands incorrectly enabled | Generate a capability/command fault matrix from one canonical graph |
| P2 | `NEXT_STEPS.md` contains a long completed P0–P6 history | Current work can be obscured by historical packages | Keep history, but make M7 packages the first executable section |
| P2 | Website and docs are manually synchronized | Non-normative public claims may drift from Current State | Extend static validation for milestone/process/test-count claims |
| P2 | Only `x86_64-linux` is exposed | Portability and hardware breadth remain unknown | Defer expansion until M7 local slice is stable; document platform policy |

## Documentation and decision-governance findings

At this checkpoint, the documentation set is extensive and machine-linked, but breadth creates a
maintenance risk. The most valuable improvement is not more prose; it is stronger source-of-truth
discipline:

- keep this checkpoint immutable except for corrections;
- update Current State with every implemented capability change;
- keep Next Steps executable and move completed detail into clearly historical sections;
- derive counts and interface lists where practical;
- review Proposed ADR status whenever implementation cites an ADR as normative;
- forbid milestone-relative phrases that become false after the next milestone.

This analysis found two residual claim mismatches and corrects them in the same change: M6 was
still labelled current in Roadmap, and Threat Model still described lifecycle as only proposed.

## Recommended M7 sequence

### P7.0 — Freeze epistemic ownership and budgets

Decide, through an accepted ADR:

- the owner of the epistemic projection;
- source versus derived state locations;
- freshness vocabulary and clock semantics;
- privacy inheritance and retention authority;
- contradiction/reconciliation state machine;
- Event1 contribution kinds and canonical evidence links;
- maximum acquisition, projection, and Presence latency;
- what is reconstructible and what must be persisted.

Exit gate: no unresolved dual ownership, no write path from QML, and no perception source treated
as truth merely because it is available.

### P7.1 — Add one typed local perception envelope

Choose one bounded, low-risk source such as current system generation/build identity. The adapter
must attach source ID, acquisition time, freshness horizon, privacy class, and provenance. It may
propose an Observation through Event1; it may not mutate system configuration or become Journal
owner.

Exit gate: malformed/future schemas fail closed, duplicate acquisition is idempotent by declared
semantics, and source unavailability has a typed result.

### P7.2 — Build a reconstructible epistemic projection

Project accepted observations into explicit statuses such as observed, stale, disputed,
superseded, or unknown. Preserve evidence links and never rewrite Journal history.

Exit gate: restart reconstruction is deterministic; contradictory inputs do not silently use
last-write-wins; stale data cannot be presented as current.

### P7.3 — Implement minimal retention and erasure obligations

Define retention for the selected source, its projection, caches, migration backups, and derived
records. An unverified erase must remain an outstanding obligation rather than a reported success.

Exit gate: tests demonstrate expiry/erasure propagation for the complete local slice and show the
failure state when one target cannot be verified.

### P7.4 — Add read-only Presence projection

Expose provenance, status, freshness, and deficit information through Presence1 and the QML proxy.
The UI remains read-only for epistemic authority and must show unknown/stale/disputed distinctly.

Exit gate: one monotonic budget covers the new compound projection, the existing UI remains usable
when the epistemic owner is absent, and no new hidden owner appears in Plasma.

### P7.5 — Prove the failure matrix

Cover adapter loss, epistemic-owner restart, malformed input, stale source, contradiction,
retention failure, Event1 loss, timeout, late reply, and reboot reconstruction. Add load fixtures
large enough to enforce the agreed latency and Journal-growth budgets.

Exit gate: focused process tests plus one NixOS VM/KVM gate pass from a clean source revision.

### Explicitly deferred

- inter-node transport and replication until the local retention/erasure slice is verified;
- language faculty until typed context selection carries provenance, freshness, privacy, and
  deficits;
- privileged execution until proposal, criticism, authorization, typed execution, observation,
  outcome, and rollback are independently represented;
- broad sensor ingestion until one narrow adapter satisfies all P7.0–P7.5 gates.

## Checkpoint acceptance criteria

This checkpoint is considered reproducible when the analysed tree can demonstrate:

```bash
nix flake check --no-build
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
```

The KVM gates remain separate evidence and require an appropriate host:

```bash
nix build --print-build-logs \
  .#checks.x86_64-linux.lifecycle-continuity \
  .#checks.x86_64-linux.p4-plasma-lifecycle \
  .#checks.x86_64-linux.m6-recovery-boundary \
  .#checks.x86_64-linux.vm-smoke
```

Passing the fast set proves reproducible evaluation, packages, static contracts, and the 20 Mind
CTest suites. It does not substitute for fresh KVM or release-artifact evidence.

## Decision at this checkpoint

Proceed to M7 only through the local vertical slice above. Treat architecture governance,
security boundaries, retention semantics, scale budgets, and failure evidence as part of the
feature, not follow-up work. Preserve the current absence of hidden model authority and privileged
execution.
