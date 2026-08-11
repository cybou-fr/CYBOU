<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Next Engineering Steps

## Purpose

This document preserves the executable packages that produced the completed M5, M6, and P6.7
boundaries. [Roadmap](ROADMAP.md) remains the milestone definition; [Current State](CURRENT_STATE.md)
remains the implementation authority. The current M7 entry sequence and risk priorities are
recorded in the [2026-08-10 Project Checkpoint](PROJECT_CHECKPOINT_2026-08-10.md).

The immediate objective is still not language or autonomous action. It is:

> add one provenance-bearing local perception path with explicit freshness, epistemic status,
> retention behavior, bounded projection, and fault evidence.

M5 continuity, M6 capability honesty, and P6.7 bounded compound IPC are demonstrated. The current
architectural bottleneck is grounded knowledge governance: source provenance, freshness,
contradiction, retention, and erasure must become testable before replication or language.

## Current architecture assessment

### Strengths to preserve

- one canonical Event1/Journal acceptance boundary;
- explicit process and state owners;
- durable-before-visible ordering;
- lifecycle run persistence, high-water marks, deterministic operation keys, and split-commit recovery;
- Presence and Plasma as replaceable projections rather than cognitive owners;
- layered unit, process-integration, focused KVM, and full Plasma gates.

### Immediate gaps

- Presence readiness is effectively the conjunction of almost every organ `Ready()` value;
- health strings identify components but do not define which user-visible capabilities remain;
- most internal RPC is synchronous and has no shared retry/backoff/circuit-breaker contract;
- timeout, rejection, unavailability, and unknown mutation outcome are not a common typed vocabulary;
- lifecycle request age is projected, but general measurement/evidence freshness is not;
- no owner currently maintains a capability dependency graph or homeostatic pressure projection;
- M7 retention/epistemic, M8 language, and M9 action boundaries must remain deferred.

### Architectural direction

M6 introduces a dedicated capability/health owner, not a second cognitive monolith. It observes
typed service health and dependency policy, exposes an immutable projection, and records only
significant transitions through Event1. Existing organs retain their state and domain ownership;
Presence remains read-only; lifecycle mode remains orthogonal to capability health.

## Sequencing rules

1. A package starts only when its prerequisite gate is green.
2. Protocol/schema changes land before dependent UI behavior.
3. Every persistent change includes migration, interruption, and recovery tests.
4. `CURRENT_STATE.md` advances only with demonstrated implementation.
5. M8 language and M9 action work do not begin before the M6 exit gate.

## P0 — Restore a trustworthy green baseline

**Status: complete.** The fast Nix gate, all four repository-specific validators, REUSE 3.3, both
primary packages, and the Mind CTest suites pass from the complete working tree.

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

**Status: complete.** `cybou-lifecycled`, atomic run persistence,
Lifecycle1, systemd activation, and active-run restart recovery are implemented. Process-level
D-Bus restart, duplicate-owner, simulated-login identity/open-intention continuity, active-run
reboot recovery, legacy-state backup/migration, and future-schema rejection are covered. The
focused headless NixOS gate proves identity and exact persisted-run continuity across a real booted
system transition; the two-node Plasma smoke remains a separate system/UI gate.

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

**Status: complete.** Lifecycle1 persists idempotent capability
acknowledgements tied to the run and accepted high-water mark, rejects premature completion,
represents optional deficits, resumes the same operation keys after recovery, and automatically
dispatches typed work to Predictor1 and Workspace1. Both owners now commit deterministic,
evidence-linked Event1 `Learning` contributions before returning typed receipts; repeated dispatch
does not duplicate them. Capability-to-contribution references are persisted atomically with
acknowledgement. `Completed` now requires a deterministic accepted Event1 terminal `Outcome`
caused by every owner result. Lifecycle mode/status/state and lifecycled health now project through
Presence1 to the QML proxy. Process-level coverage exercises the two
critical split-commit windows—owner Event1 commit before run acknowledgement, and terminal Event1
commit before terminal run state—and proves idempotent recovery. The headless NixOS gate repeats
both scenarios across real reboots and proves Event1 count is unchanged by replay.

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

The VM test interrupts both distributed split-commit boundaries, reboots the machine, and observes
one correct terminal result with intact identity and biography and no duplicate Event1 effect.

## P4 — Make lifecycle visible without moving ownership into UI

**Status: complete.** Presence1 and the QML proxy expose lifecycle mode, status, full state,
lifecycled health, and a read-only projection with progress class/percentage, request freshness,
and causal capability deficits. Mind Header and Dashboard render distinct lifecycle modes without
interpreting the durable run schema. Runtime availability remains orthogonal to lifecycle mode.

### Work

- [done] add explicit progress class, freshness, and deficit presentation to the lifecycle projection;
- [done] route user lifecycle interruption through a non-blocking QML D-Bus call with pending and
  timeout-safe completion state;
- [done] refine the current mode label into distinct `Idle`, `Consolidating`, `Recovering`, and `Degraded`
  visual treatments;
- [done] provide user interruption for active runs while lifecycled retains terminal-state ownership;
- [done] keep the existing shell usable when lifecycle services are absent;
- add VM interaction assertions (QML static validation is complete).

### Exit gate

Destroying/recreating Plasma or the Presence proxy neither changes lifecycle state nor duplicates a
run. A timeout cannot freeze the shell for the current five-second blocking RPC window.

Both invariants pass at process level. The focused single-node `p4-plasma-lifecycle` gate isolates
the shipped-Plasma restart assertion from the two-node Gate A smoke and passes under KVM.

## P5 — Close M5 and publish evidence

**Status: complete for the unversioned evaluation milestone.** The M5 evidence matrix, focused
lifecycle/Plasma gates, corrected two-node `vm-smoke`, and aggregate flake check pass. VM and ISO
were built from clean revision `ddd6c83`; immutable Nix outputs, size, SHA-256, environment, and
compatibility boundary are recorded in `M5_EVALUATION.md`. This is evaluation evidence, not a
stable tagged release.

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

## P5.1 — Close documentation and isolate the hardening track

**Status: complete in documentation; implementation gates remain authoritative.** M5 is the clean
evaluation baseline for M6. In-place upgrade reconciliation beyond the tested lifecycle schema
v0-to-v1 migration is a separate hardening track and must not be described as supported.

### Exit gate

- `ROADMAP.md`, `CURRENT_STATE.md`, release evidence, and this plan agree that M5 evaluation is complete;
- unsupported upgrade paths remain explicit;
- M6 work cannot weaken the existing lifecycle continuity and idempotency gates.

## P6.1 — Freeze the capability and health contract

**Status: complete.** Schema-v1 component health, capability state, typed deficit cause, recovery
policy, snapshot CBOR, fail-closed validation, component transitions, and focused tests are present.
The protocol deliberately contains no dependency graph, daemon, D-Bus service, or UI policy.

### Work

- define versioned `ComponentHealth`, `CapabilityState`, `CapabilityDeficit`, dependency, freshness,
  and recovery-policy wire types;
- keep component health separate from capability availability;
- distinguish `Unavailable`, `TimedOut`, `Rejected`, and `UnknownOutcome`;
- define significant transitions that cross Event1 without recording routine probe noise;
- define aggregate Mind health without reducing it to the conjunction of organ `Ready()` values;
- update IPC, ownership, failure, security, and migration contracts together.

### Exit gate

Codecs and transition tests prove fail-closed handling of unknown versions/states. A deficit names
the affected capability, dependency, cause, detection time, last verified success, impact, recovery
policy, and evidence/error reference where available.

## P6.2 — Implement the dependency graph and health owner

**Status: complete.** The initial dependency graph, aggregate policy, `cybou-healthd`, persistent
snapshot owner, `Health1`, D-Bus/systemd activation, corrupt-state failure, exact restart recovery,
explicit component recovery, and process-level optional-owner fault injection are implemented.
Presence does not consume Health1 yet.

### Work

- classify identity continuity and accepted biography/commitments as required capabilities;
- classify prediction, self-assessment, attention maintenance, and consolidation owners according
  to the operation that needs them;
- implement a dedicated `cybou-healthd` with versioned `Health1` rather than making Presence the
  durable health owner;
- observe D-Bus ownership and typed organ health without opening organ storage;
- persist only policy state that must survive process recreation;
- project an immutable capability snapshot to Presence1.

### Initial capability matrix

| Capability | Required dependencies | Expected partial-failure behavior |
|---|---|---|
| identity continuity | `identityd`, accepted Event1 history | fail closed; never invent identity |
| commitment access | `intentiond`, accepted Event1 history | preserve accepted commitments or report unavailable |
| prediction | `predictord` | prediction unavailable; independent capabilities remain usable |
| self-assessment | `selfd` | assessment unavailable; identity and commitments remain usable |
| attention/workspace | `workspaced` | attention limited; durable biography remains usable |
| consolidation | `lifecycled` plus run-specific owners | run limited/failed/degraded by typed policy |
| Presence presentation | `presenced` | UI unavailable; cognitive owners remain unchanged |

### Exit gate

Stopping an optional organ removes only dependent capabilities. Restart produces an explicit
`Recovering` transition and verified return to `Available`; Presence recreation changes neither.

## P6.3 — Add bounded asynchronous RPC resilience

**Status: complete for the shared transport and first required consumer.** Typed outcomes,
operation semantics, bounded deterministic backoff, retry eligibility, circuit breaking, and a
common async D-Bus client are implemented. Plasma lifecycle interruption is migrated as a
non-idempotent mutation: timeout becomes `UnknownOutcome` and is never retried. Legacy synchronous
read paths remain for deliberate future migration.

### Work

- add common bounded async calls for paths that must not block an owner or shell;
- define per-method timeout and idempotency metadata;
- retry only when the operation contract permits replay;
- add exponential backoff with jitter and a bounded circuit breaker;
- preserve `UnknownOutcome` when a mutation may have committed but its response was lost;
- reuse M5 operation keys for retryable durable work.

### Exit gate

Timeout cannot freeze Presence, exhaust the session bus, duplicate an Event1 effect, or turn an
unknown mutation result into success. Unit tests use a deterministic clock/backoff source.

## P6.4 — Introduce homeostatic signals without autonomous policy

**Status: complete for typed observation.** Health1 now publishes schema-v1 measurements with
source, units, observation/validity time, and explicit current, stale, unknown, or unsupported
status. Schema v1 forbids scheduling authority. Backlog, storage growth, and calibration pressure
remain explicitly unsupported until their owners expose typed contracts; no zero is fabricated.

### Work

- measure Event1 backlog, Journal/storage growth, RPC latency/error pressure, lifecycle backlog,
  projection age, and calibration pressure;
- attach units, observation time, freshness, source, and supported/unsupported status;
- expose measurements before allowing them to schedule work;
- define bounded thresholds and hysteresis separately from raw measurements.

### Exit gate

Every signal is typed, source-bearing, testable, and cannot silently trigger consolidation. Stale
or unavailable measurements remain explicit instead of falling back to fabricated zero values.

## P6.5 — Add capability-aware scheduling and metacognitive projection

**Prerequisite hardening: implemented.** Health1 probes now use parallel, bounded read-only async
RPC with a common deadline, D-Bus owner-change debounce, slow verification, and typed timeout
mapping. Consolidation owners reconstruct their computed values only through the accepted
high-water mark; lifecycle validation requires a cause for every missing capability; terminal
Outcome evidence preserves completed and missing work. CapabilitySnapshot schema v2 now preserves
each unhealthy `(capability, dependency)` pair and migrates persisted schema v1, completing the
protocol prerequisite for detailed P6.5 recovery causes.

**Slice 1: implemented.** Presence1 and its QML proxy now expose aggregate capability state,
per-capability states, typed deficits, and observation time. Commands use explicit capability gates
instead of one broad `awake` gate. Process integration proves predictor loss leaves identity,
commitments, biography, and attention usable, then restores prediction after Health1 recovery.
Presence readiness remains independent of Health1 to avoid the healthd→presenced probe cycle.

**Slice 2: implemented.** Lifecycle1 owns a deterministic read-only scheduling evaluator and
Presence projects its decision. It validates current capability/homeostasis evidence, blocks loss
of accepted biography, preserves remaining optional consolidation workers, and computes Event1
backlog hysteresis at 32/8. At that slice's boundary schema v1 still forbade scheduling authority,
so evaluation deferred without changing lifecycle mode or creating a run.

**Slice 3: implemented.** Event1 now owns atomically persisted, monotonic consumer offsets and an
exact backlog projection. Lifecycled registers `lifecycle.consolidation`, advances it only after a
durable completed run, and reconciles the same offset after restart. Consolidation-scoped owner and
terminal contributions do not count toward their own backlog. Health1 consequently exposes a
current Event1 backlog measurement instead of `Unsupported`.

**Slice 4: implemented.** Homeostasis schema v2 replaces the global boolean guard with unique,
bounded authorized policy IDs and migrates schema v1 strictly as observation-only. Health1 grants
`event-backlog-v1` only with a current owner-backed backlog. Lifecycle1 still owns all capability,
freshness, idleness, worker, and hysteresis gates. Process integration drives backlog to 32 and
proves Lifecycle1 and Presence return `Run` without creating or mutating a lifecycle run.

**Slice 5: implemented.** Lifecycle1 now exposes an explicit execution command that requires the
exact capability/homeostasis snapshot IDs returned by evaluation and revalidates both immediately
before mutation. It derives a deterministic run UUID from policy and evidence, making retries safe
after timeout, completion, and even after a later run replaces the current projection. Stale
evidence fails without changing lifecycle state. The accepted execution creates the existing
bounded consolidation transaction; dispatch and terminal completion remain explicit owner steps.

**Slice 6: implemented.** Lifecycled now runs the bounded orchestration cycle after a 100 ms
Health1-change debounce and on a 30-second verification timer. `Block` and `Defer` are no-ops;
`Run` invokes the evidence-bound idempotent command, dispatches owners, and commits completion.
Recovery always continues an existing scheduled run before considering new evidence. A crash after
durable run creation but before dispatch resumes and completes the same run with zero residual
backlog. Tests can disable automatic triggers while invoking the identical production method.

**Slice 7: implemented.** Presence command entry points now report explicit user activity to
Lifecycle1. Lifecycle schema v2 durably stores the last activity time and scheduler cooldown end;
evaluation and execution defer while the cooldown is active, including after lifecycled restart.
Activity wakes `Idle` and atomically interrupts an active `event-backlog-v1:*` run, but never
silently terminates a manually requested maintenance run. At this slice, synchronous owner
dispatch remained the interruption boundary.

**Slice 8: implemented.** Automatic owner dispatch is now a sequential asynchronous state machine
using the shared idempotent-mutation retry and circuit-breaker policy. `RunSchedulingCycle` returns
`started` after durable run creation and never blocks Lifecycle1 on an owner call. Every callback
rechecks the original run identity and active state before accepting its durable contribution, so
activity can persist `Interrupted` during an in-flight RPC and a late owner reply cannot resurrect
or complete that run. The explicit synchronous `Dispatch` command remains for administrative
compatibility; the production scheduler no longer uses it.

**Slice 9: implemented.** Presence1 now groups Health1 evidence into a presentation-ready
`capabilityDetails` map. Every known capability exposes its state, availability, typed causes,
operational impacts, dependencies, last verification, recovery policies, and a compact recovery
progress (`ready`, `waiting`, `verifying`, or `unknown`). QML no longer needs to interpret raw
deficit records to explain partial availability. Process integration proves predictor loss keeps
identity, commitments, attention, and biography useful, then projects verification and recovery.

**Slice 10: implemented.** Presence1 publishes `commandAvailability`, the authoritative UI mapping
from each command to required and currently missing capabilities, plus `canCommand(id)`. Backend
methods retain their own fail-closed gates. Process integration proves prediction loss disables
only Observe/Predict while promise, commitment, identity, and attention operations remain enabled.
It also proves lifecycle and capability health are orthogonal by observing both `Awake + Limited`
and `Recovering + Limited`. This completes the P6.5 exit gate.

### Work

No remaining P6.5 implementation work. Continue with P6.6 fault-injection coverage.

### Exit gate

The UI explains what is unavailable and what still works without interpreting owner storage or
durable schemas. Language, confidence, and general epistemic claims remain outside M6.

## P6.6 — Prove partial availability and recovery

**Slice 1: implemented.** Real process fault injection now stops and restores predictord, selfd,
and workspaced independently. Each loss produces only its mapped typed deficits and command gates;
failed reflection and unavailable attention are rejected before any Event1 mutation, while
unrelated identity, commitments, prediction, and presentation remain usable. Recovery is observed
through `waiting → verifying → ready` rather than inferred from process existence.

**Slice 2: implemented.** Lifecycled loss now proves lifecycle control and consolidation become
unavailable without disabling identity or commitments and without creating an Event1 effect for a
rejected interruption. Presenced loss makes an existing QML proxy explicitly unreachable while
retaining its last cached projection; the same proxy reconnects after restart with unchanged
identity, session count, Event1 count, and all owner PIDs. Lifecycled recovery is owner-verified.

**Slice 3: implemented.** A real scheduled owner now remains on D-Bus while exceeding a bounded
lifecycled deadline. The idempotent mutation is retried under the shared backoff/circuit policy;
because the timed-out owner was required by the accepted decision, the run fails closed in
`Recovering` and its consumer backlog is not advanced. Delayed replies converge on one deterministic
owner contribution and cannot change the failed run. After owner recovery, a new evidence-bound
run consumes the preserved backlog exactly once. The production deadline remains five seconds;
tests use the bounded `CYBOU_LIFECYCLE_OWNER_TIMEOUT_MS` override.

**Slice 4: implemented.** Scoped RPC fault injection crashes lifecycled both after the first
retryable owner failure and after the circuit opens on exhausted failures. In both cases the
durable scheduled run remains active, restart enters `Recovering`, and `RunSchedulingCycle`
continues the same run ID. Deterministic owner contributions absorb abandoned/late calls, terminal
completion occurs once, and consumer backlog reaches zero without a replacement run.

**Slice 5: implemented.** Required-owner fault injection stops Event1 while every other process
remains alive. Presence stays reachable but accepted biography, identity continuity, and commitment
access fail closed; a Promise attempt creates no accepted contribution. Restart opens the same
Journal with an unchanged count, UUID, session count, and existing commitment, and the rejected
description is absent. Owner-verified refresh restores commands. This completes the process-level
P6.6 matrix.

**Slice 6: implemented.** The focused single-node KVM gate boots the shipped Plasma session,
proves D-Bus/systemd activation of Presence without replacing plasmashell, and exercises a delayed
lifecycle interruption without permitting a false durable transition. It then recovers the same
service boundary, suspends the required Event1 owner, observes a fail-closed Promise with no Journal
growth, resumes Event1, and verifies both owner and Plasma continuity. This completes P6.6.

### Fault-injection matrix

All process-level cases above and the focused system boundary are implemented with recovery and
duplicate-effect assertions.

Use process integration for the complete matrix and one focused KVM gate for D-Bus/systemd
activation, timeout, recovery, and Plasma projection. Do not multiply the expensive two-node smoke
test for cases already proven below the system boundary.

### M6 exit gate

Loss of every optional organ has an explicit capability deficit, useful remaining behavior,
bounded retry/recovery rule, Presence projection, and test. Required-owner loss fails closed without
identity replacement, commitment loss, false acceptance, or duplicate durable effects.

**Status: satisfied.** The complete process matrix plus `m6-recovery-boundary` provide the M6 exit
evidence.

## P6.7 — Bound compound Presence commands

**Slice 1: implemented.** EventClient now uses an explicitly timed asynchronous D-Bus pending call
instead of relying on blocking-call timeout behavior. Promise validates its capability gates and
required Event1 owner before notifying auxiliary owners, so an unresponsive Journal fails closed
without accumulating the budgets of later steps. The KVM gate tightens its client deadline from 40
to 8 seconds and observes rejection in under one second with no Journal growth.

Next, introduce one command deadline context that propagates the remaining budget across compound
commands involving several independent owners. This is post-M6 latency hardening, not a reason to
weaken the completed M6 availability and continuity claims.

**Slice 2: implemented for Promise.** RpcClient and EventClient accept a per-call timeout while
preserving their five-second default. Promise creates one five-second `QDeadlineTimer`, reads one
Health1 snapshot, and passes only the remaining budget through Event1 preflight, Lifecycle1 activity,
the durable observation, and Intention1 formation. No later RPC is sent after exhaustion. The KVM
gate runs presenced with a one-second command budget and requires its server-side rejection inside a
three-second client deadline, followed by exact Journal and Plasma continuity.

Next, reuse the same deadline context for Reflect, Observe/Predict, and commitment mutation commands,
then remove the temporary per-command repetition behind a small shared helper.

**Slice 3: implemented.** A shared `CommandDeadline` helper now supplies the monotonic remaining
budget for Reflect, Observe, Predict, Fulfill, and Abandon as well as Promise. Each compound command
reads one Health1 snapshot and forwards only the remaining time to every required Event1,
Lifecycle1, Self1, Predictor1, and Intention1 call. Mutating paths preflight Event1 before auxiliary
notifications; read-only Predict avoids an unnecessary Journal dependency. Expiry rejects the
command before another RPC is sent. Process integration and the focused KVM gate preserve all M6
continuity and recovery guarantees under the stricter transport behavior.

Next, align the backend `InterruptLifecycle` compound path with the same server-side deadline model
while preserving its shell-facing non-idempotent `unknown-outcome` contract, then audit remaining
multi-owner orchestration for independently accumulated budgets.

**Slice 4: implemented.** `InterruptLifecycle` now creates its server-side `CommandDeadline` before
validation and passes the remaining budget through Lifecycle1 `State` and `FinishRun`. Expiry after
validation prevents the terminal mutation from being sent. The test-only delay now exercises this
deadline directly: Plasma remains responsive, its five-second non-idempotent transport timeout is
still reported as `unknown-outcome`, and Lifecycle1 state remains byte-identical. Recovery performs
one successful interruption through the same path.

Next, bound the read-only Presence snapshot aggregation. It currently visits several independent
owners sequentially and is the remaining visible source of accumulated default RPC budgets; the
projection must stay structurally valid when its shared budget expires partway through collection.

**Slice 5: implemented; P6.7 complete.** `Snapshot`, `Activity`, and `DetailedObligations` now each
own one monotonic deadline. Snapshot forwards only the remaining budget to Self1, Lifecycle1,
Intention1, Workspace1, Event1, Identity1, and Predictor1, skips later calls after expiry, and keeps
every projection key present with a typed empty/default value. Organ health is derived from the one
canonical Health1 snapshot instead of issuing another Ready/Health cascade. Process coverage
suspends a still-registered selfd under a 500 ms server budget and proves Snapshot returns a valid
partial projection in under 1.5 seconds without reaching later owners. The same remaining-budget
contract now covers every compound Presence read and mutation.

Next, close the substrate findings recorded in P6.8 before opening the first M7 vertical slice.

## P6.8 — Close the substrate audit findings

**Status: in progress.** The [Implementation Audit — 2026-08-10](CODE_AUDIT_2026-08-10.md) found
four places where the shipped implementation does not support a stated invariant, plus two hygiene
items. This package closes them before M7 raises event volume and adds a second projection, because
each finding becomes more expensive to fix once a perception adapter depends on it.

This is substrate repair on existing owners. It introduces no new process, no new persistent state
location, and no new wire contract.

### Work

- **A1 durability.** Raise the Journal to a commit mode that survives power loss, or restate the
  "durable before visible" invariant as durability to the operating system. Do not leave the
  invariant stated more strongly than the storage configuration supports.
- **A3 health honesty.** Derive presenced `Health()` from real aggregation outcomes and required
  downstream reachability, so `presence-presentation` can enter a deficit. Keep "the process is
  running" separate from "the process can present".
- **A4 bounded reads.** Replace the per-row `ConsumerBacklog` scan with one aggregate counting
  query. Do not cap `Recent` or bound `Verify`: `recent(0)` is how intentiond, predictord, and selfd
  replay their entire state, and selfd calls `Verify` on the ordinary self-assessment path, so a cap
  would silently truncate organ reconstruction and a partial verification would be reported as an
  integrity result. Those two need a cursor-carrying replay API and incremental verification against
  a persisted checkpoint, which is a contract change and is deferred below.
- **A5 failpoints — withdrawn, no change.** The hooks stay in the shipped binaries. `qFatal` grants
  no capability a same-user process lacks, since that process can already signal any daemon, and the
  reboot gate sets `CYBOU_LIFECYCLE_FAILPOINT` against the installed package — gating them out would
  move the split-commit evidence onto a binary that is not the shipped one. Recorded as an accepted
  property in the threat model.
- **A6 unit hardening.** Add justified systemd hardening to the Mind units, limited to directives a
  user manager can actually enforce — seccomp, rlimits, and no-new-privileges. Namespace-based
  options are omitted because an unprivileged user manager cannot apply them reliably, and
  `ProtectHome` would hide the Journal. Record the result as reducing the blast radius of a
  compromised Mind process, not as progress on the same-user D-Bus authorization boundary, which it
  does not address.

### Deferred within this package

**A2 — Presence projection migrated; healthd and the mutations are not.** `Snapshot` is now a
delayed-reply method that gathers every capability-gated owner concurrently through
`AsyncRpcClient`, so presenced serves other callers while aggregating and pays the slowest owner
rather than the sum. The projection clients use a single attempt and no circuit latching:
a latched circuit outlives the request that opened it and would render a transient stall as a
permanently empty section for five seconds.

**A2 is now closed.** The mutations, `Activity` and `DetailedObligations` are asynchronous
continuations with delayed replies, so no call on the Presence surface blocks. Mutations remain
ordered — gate, Event1 preflight, activity, durable Observation, domain mutation — because their
steps are causally dependent; asynchronous is not the same property as concurrent. Safety comes from
the operation semantics, not the retry policy: a non-idempotent step is never retried, and its
timeout surfaces as `unknown-outcome`.

A claim made when this package was opened has been withdrawn. healthd does **not** probe with the
synchronous client — `Refresh` already issues every probe concurrently through `AsyncRpcClient`
under a bounded deadline. No healthd work was needed.

**P6.8 is complete.** Do not open a P6.9 for further polish: the substrate findings are closed or
explicitly deferred with reasons, and the remaining risks are M7's to carry.

## P7.0-trust — Bind Event1 origin to the calling process

**Status: implemented.** `originOrgan` is provenance, and until now the caller simply asserted it.
eventd resolves the calling connection to its executable, caches that per connection, and refuses
any contribution claiming one of the nine organ identities unless the caller is that organ.

The binding is to the executable, not to D-Bus name ownership. identityd records its session to
Event1 from its constructor, before `ServiceHost` publishes its name, so requiring ownership would
reject that write and break identity continuity at startup — a worse failure than the forgery it
prevents. A same-user process cannot fake `/proc/<pid>/exe` without actually being that binary, and
the answer does not depend on startup ordering.

This is scoped deliberately. It closes impersonation, not authorship: a caller that is not an organ
may still contribute under a name of its own, which is how tools and the test suite write. Method-
level authorization and capability tokens remain outstanding and are recorded as such in the threat
model.

Sequenced before the scale fixtures on purpose: it changes what `Submit` accepts, so building
fixtures first would mean rebuilding them.

### Exit gate

A process test that is not a Mind organ is refused all nine reserved identities, the Journal count is
unchanged by the attempt, and a contribution under a non-reserved name still succeeds.

## P7.0-ADR — Freeze the epistemic owner and ObservationV1

**Status: drafted as [ADR-0027](adr/ADR-0027-local-epistemic-projection-owner.md), Proposed.** It is
an architectural decision, so it is written as a proposal and needs explicit acceptance before P7.1
starts. Every question it settles is one a perception adapter would otherwise answer by accident.

What it proposes: a separate `cybou-epistemicd` owning the derived projection, freshness,
contradiction and reconciliation — and owning neither the Journal, nor any perception source, nor
system-wide retention. Folding it into selfd or healthd was rejected because those project Mind's own
state and component health, while an epistemic projection is a claim about the world; merging them
would make one process the authority on two different kinds of assertion.

It separates `originOrgan` (who brought this into Mind, bound to the calling process since
P7.0-trust) from `sourceId` (what was observed). Conflating them would mean replacing an adapter
silently rewrote the provenance of everything it ever reported, and two adapters on one source would
look independent — the exact condition under which a contradiction check agrees with itself.

Its budgets come from [Scale Budgets](mind/SCALE_BUDGETS.md) rather than intuition, and one of them
has a direct consequence: an epistemic projection replaying the whole biography would exhaust the
5 s Presence budget near 560k contributions, so `epistemicd` must consume Event1 through the paged
`Replay` cursor with a persisted position. It is therefore the first real consumer of P7.0-replay,
and the first place a projection checkpoint will be justified by measurement rather than anticipated.

**Retention is deliberately not decided there**, and until a separate storage ADR covers expiry,
tombstones, derived-data propagation, backups and possibly per-record keys, no sensitive observation
may be ingested. The first source — NixOS system generation and build identity — is chosen so that
constraint costs nothing: local, non-sensitive, and naturally contradictory, since the generation
changes while an earlier observation still claims to be current.

## P7.0-registry — One capability and command declaration

**Status: implemented.** `CapabilityRegistry` in `cybou-protocol` is the single declaration of which
capabilities exist, which components each rests on, and which capabilities each Presence command
requires.

The same knowledge had been written down four times: the dependency graph in `HealthPolicy`, the
component list beside it, the command-to-capability map in the Presence projection, and again in the
capability gate of every Presence mutation. They agreed by hand. The fourth copy was added during
P6.8, which is the argument for doing this before M7 rather than after — perception, epistemic
projection and retention each add capabilities to all of them at once.

It is a policy declaration, not a state owner. It says nothing about whether anything is currently
healthy; healthd remains the sole owner of that, and reading the registry makes no other process a
second authority on capability health.

`interruptLifecycle` is deliberately absent from the command table: it is gated on lifecycled being
reachable rather than on a capability, because lifecycle mode is orthogonal to capability health and
a run must stay interruptible while other capabilities are degraded.

### Generated fault expectations

`everyComponentLossProducesTheDeclaredDeficits` walks every component, marks it unavailable, and
compares the resulting deficits against what the registry declares — in both directions, so a
capability that should *not* have degraded is caught too. This addresses the checkpoint's risk that
health tests sample representative failures rather than the whole graph, and adding a capability
extends the matrix without anyone editing the test.

`registryIsInternallyConsistent` rejects a capability resting on an unknown component, a command
requiring a capability that does not exist, and a capability that does not say what its loss costs.
Verified by deliberately breaking the registry: the check named the fault precisely, and healthd's
own integration tests failed alongside it, which confirms the registry drives production rather than
sitting beside it.

Still hand-written: the process-level matrix, where components are really stopped rather than
modelled. The policy-level matrix above covers the graph; the process level covers the wiring, and
they are not substitutes.

## P7.0-verify — Incremental verification

**Status: mechanism implemented and tested; not yet wired to selfd.** `Journal::verifyFrom(anchor)`
checks the anchor still describes the journal, then walks only the contributions after it.
`checkpointAtHead()` produces the anchor to persist after a successful check. The result is typed —
`FullyVerified`, `VerifiedThrough`, `InvalidAt`, `CheckpointMismatch` — so a partial check can never
be read as a whole-history guarantee.

`verify()` keeps its existing contract and is now expressed through the same chain walk, so there is
one implementation to be right about instead of two that can drift.

At 100k, checking 500 new contributions costs 4 ms against 1,060 ms for the full walk: the cost
follows the increment, not the history.

Three semantics worth keeping in view:

- **A checkpoint is an accelerator, never an authority.** The Journal remains the only source of
  truth about its own integrity. Losing a checkpoint costs a full verification, not correctness.
- **`CheckpointMismatch` is not corruption.** It says the checkpoint is unusable, not that the
  journal is bad; reporting it as damage would send someone looking for a fault that is not there.
- **The prefix is trusted, so corruption inside it is invisible.** A test tampers with an early
  contribution and confirms the incremental check still reports intact while the full walk catches
  it. That is why the result is typed, and why a periodic full verification stays as the heavy gate.

### Wiring — complete

`Event1.VerifyIncremental()` answers the typed result. eventd owns the checkpoint as
`verification-checkpoint.json` beside the journal, the same pattern as consumer offsets, so it stays
visibly derived: losing it costs one full verification and nothing else. A checkpoint that no longer
describes the journal is discarded and the full walk runs, because a stale accelerator says nothing
about the biography. The checkpoint only advances on a chain that held — advancing past a break
would make the next verification skip the very contribution that is wrong.

selfd now calls it on the self-assessment path and carries `verification` and `verifiedFrom` into
its report alongside `journalIntact`. The distinction is preserved rather than flattened: a
full rechain and a checkpoint-anchored check both leave `journalIntact` true, and only the status
says which evidence was actually gathered.

**The 460k cliff is closed.** `Reflect` no longer scales with the length of the biography.

## Scheduling flake — diagnosed and fixed

`scheduledOwnerTimeoutIsBoundedIdempotentAndRecoverable` intermittently saw `RunSchedulingCycle`
answer `failed` where it expected `started`. Established as a genuine flake before anything was
changed: the same Nix derivation hash produced one failing and two passing runs, so identical inputs
gave different outcomes.

**Cause.** The policy is evaluated twice. `RunSchedulingCycle` takes a health snapshot and decides,
then `ExecuteSchedulingDecision` re-fetches health and requires the snapshot identity to be
unchanged. healthd refreshes on a 30 s timer and on every bus owner change, so a refresh landing in
that window supersedes the evidence and the run is refused.

The refusal is correct — a decision reasoned about one snapshot should not execute against another.
What was wrong is that it was reported as `failed`, the same outcome as a scheduler that could not do
its job. An ordinary race was indistinguishable from a defect.

Isolated repeat runs never reproduced it; both real failures happened during full builds. The window
between the two evaluations widens under load, which is why it only appeared there.

**Fix.** A lost evidence race now answers `deferred` — which already means the run did not start and
a later attempt may succeed — with a reason naming the supersession. The tests retry on exactly that
condition and fail immediately on anything else.

**Separately, the assertions were unactionable.** They compared only the outcome and discarded the
`reason` the scheduler already returns, so every intermittent failure read as "expected started, got
failed" with nothing to act on. All three now print the reason. That is most of why this took two
sightings to diagnose.

## P7.0-replay — Paged Event1 replay

**Status: API implemented, one organ migrated.** `Event1.Replay(afterSequence, limit)` answers a
page as `{ok, from, to, head, hasMore, envelopes}`, capped server-side at 1000 so it cannot become
`Recent(0)` wearing a cursor. `EventStore` gained `after()` and a `replayAll()` helper; `Journal` and
`EventClient` both implement it. `Recent` is unchanged and stays what it should be — genuinely recent
activity for the UI, not a replay protocol.

The page reports `ok` separately from being empty. Without that, a replay whose transport died
halfway looks exactly like one that finished, and an organ rebuilds its state from a prefix while
believing it has the whole history.

**intentiond is migrated.** Two chronological passes, both paged, replacing one `recent(0)` that
was walked three times. Two hazards had to be handled, and both are the reason the remaining organs
are not done in the same change:

- **Order is inverted.** `recent(0)` yields newest first; `replayAll` yields oldest first. The old
  code ended with a `std::reverse` to correct for that, and keeping it would have inverted the order
  Presence shows commitments in.
- **Partial replay must fail closed.** If the first pass dies halfway, commitments whose closing
  Outcome was not read look open. Returning a partial open set is worse than returning none.

**Not yet migrated: predictord, selfd, workspaced.** Each has its own order dependence and must be
read before it is changed:

- `Predictor::samples` appends in newest-first order and downstream treats position as recency;
  reversing it silently changes which samples a prediction is built from.
- `Predictor::calibration` and `allCalibrations` sum and set-collect, so they are order-independent
  and are the safe ones.
- `SelfModel::subjects` already iterates `recent(0)` in reverse to get oldest-first, so it becomes
  simpler, not harder.
- `Workspace` uses `recent(m_capacity)` — bounded already, and not a replay at all. It may not need
  migrating.

**What this does not fix.** Paged replay is not faster: 728 ms against 762 ms at 100k. It bounds
memory and removes the single enormous D-Bus reply, both real failure modes, but the
cold-reconstruction budget is unmoved. See [Scale Budgets](mind/SCALE_BUDGETS.md).

## P7.0-scale — Journal fixtures and budgets

**Status: implemented for the Journal paths.** `journal-scale` builds a deterministic fixture and
measures append, full replay, `Verify`, backlog counting, indexed lookup and size. It defaults to
10k so it runs in the ordinary checks; 100k and 1m are the same code with
`CYBOU_SCALE_CONTRIBUTIONS` set. Results and the budgets derived from them are in
[Journal Scale Baseline and Budgets](mind/SCALE_BUDGETS.md).

Every growth-sensitive path is linear across two orders of magnitude and indexed lookup is flat, so
there is no hidden quadratic. The two thresholds that matter both land near half a million
contributions: `Verify` consumes the entire 5 s Presence command budget at ~460k, which makes
`Reflect` impossible, and organ cold reconstruction costs ~9 s per organ at 1m with three organs
each replaying the whole history. Those are the numbers the paged-replay and incremental-verification
packages exist to move.

Not yet measured, and the honest gaps: per-organ cold reconstruction end to end rather than just the
Journal read under it, RSS, Presence and consolidation behaviour under a large journal, concurrent
read/write pressure, and the growth rate of a real biography — without which the thresholds cannot be
turned into a date.

**A4 remainder — scalable biography replay and verification.** Every organ rebuild pulls the full
biography across D-Bus and every self-assessment rechains it. Closing this needs a replay API that
carries a cursor and an incremental `Verify` against a persisted checkpoint, so a partial result is
reported as partial. Both are wire-contract changes and belong with the M7 scale fixtures that give
them a budget to be measured against.

**A7 — fault-tier CI coverage.** None of the four KVM gates runs on a push. This is a maintainer
policy decision — KVM-capable runner, scheduled run, or a documented pre-tag manual gate with a named
owner — and not a code change.

### Exit gate

The fast Nix check set and both package builds pass from a clean tree, the Mind CTest suites pass
including new coverage for the bounded read paths and the derived presenced health answer, and the
four KVM gates pass on a KVM-capable host with no change to their continuity and recovery assertions.

Next, begin the first M7 vertical slice: one provenance-bearing local system perception adapter,
typed freshness/retention policy, an accepted Event1 observation, and a read-only Presence
projection. Keep contradiction handling explicit and do not add autonomous action authority.

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

## Historical P6 PR decomposition

| PR | Scope |
|---|---|
| 1 | P6 capability/health protocol, codec, transition tests, and accepted ownership ADR |
| 2 | explicit dependency graph and aggregate-health policy |
| 3 | `cybou-healthd`, `Health1`, systemd activation, and persistence/migration tests |
| 4 | Presence1 capability projection without UI policy ownership |
| 5 | bounded async RPC, idempotency metadata, backoff, and circuit breaker |
| 6 | typed homeostatic measurements and freshness semantics |
| 7 | capability-aware scheduling and degraded-mode UI |
| 8 | process fault-injection matrix and focused M6 KVM gate |
| 9 | synchronized docs and M6 evaluation evidence |

Keep protocol/schema work separate from UI polish. Do not combine raw measurement collection with
automatic scheduling policy, and do not begin M7 retention or epistemic ownership inside M6.

## Definition of done for every package

- explicit owner and non-owner list;
- typed success, failure, interruption, and degraded outcomes;
- persistence and migration statement;
- privacy/provenance/retention impact;
- unit plus process/VM test proportional to the boundary;
- updated contracts and `CURRENT_STATE.md` when behavior is implemented;
- reproducible Nix gate from a clean tree.
