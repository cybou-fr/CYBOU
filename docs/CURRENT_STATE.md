<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Current State

Status date: 2026-08-09.

This document is intentionally limited to implemented behavior and current limitations.

## Repository gate status

The P0 baseline is green: formatting, REUSE 3.3, package metadata, cognitive documentation, Mind
access, QML API, UI polish, `cybou-mind`, and `cybou-presence-applet` pass through pinned Nix checks.
The Mind package runs nineteen CTest suites, including Event1, lifecycle persistence/recovery,
Lifecycle1 process restart, and seven-process M4 integration. The process suite also proves a
simulated new login preserves identity and an accepted open intention while incrementing the
logical session count.

The M5 lifecycle owner is present: lifecycle schema v1, legal mode transitions, atomic persistent
run state, `org.cybou.Mind.Lifecycle1`, D-Bus/systemd activation, D-Bus run requests, and restart
recovery of an active run into `Recovering`. Legacy v0 state is backed up and migrated to v1;
unknown future versions fail closed. The focused headless NixOS gate proves that a real reboot
preserves the exact persisted run and identity ID, enters `Recovering`, and increments the logical
session count.

The P3 transaction substrate now includes deterministic per-capability operation keys,
high-water-mark-bound idempotent acknowledgements, optional capability deficits, required-work
completion gates, and explicit resume of the same run after recovery. Lifecycle1 automatically
dispatches bounded `Consolidate` requests to Predictor1 and Workspace1 and validates their typed
receipts before persisting acknowledgements. Each owner resolves the exact accepted Event1 envelope
at the run high-water mark, commits an evidence-linked `Learning` contribution with a deterministic
UUIDv5 operation identity, and returns its contribution ID. Redelivery is a durable no-op, and the
integration suite proves two first-delivery contributions and zero duplicate contributions.
Lifecycle1 persists the capability-to-contribution mapping in the run, verifies every reference
against Event1, and refuses `Completed` until it has committed a deterministic terminal `Outcome`
caused by all owner results. The process integration suite verifies the extra terminal append and
exposes its ID through Lifecycle1 state. Process-level
fault injection now kills lifecycled immediately after an owner Event1 commit and immediately after
the terminal Event1 commit. In both cases restart enters `Recovering`, replay reuses deterministic
contributions, and Event1 count proves that no duplicate durable effect was created.
Lifecycle mutations roll back their in-memory candidate when persistence fails; unknown status
values fail protocol validation; optional deficit causes persist in the run; and the preferred
`RequestRunAtCurrentHead` API captures its accepted boundary directly from Event1.

Lifecycle1 emits `Changed` after each accepted state commit. Presenced subscribes through a typed
LifecycleClient and projects `lifecycleMode`, `lifecycleStatus`, the full lifecycle state,
lifecycled health, and a derived progress/freshness/deficit view through Presence1. The QML proxy
exposes these as read-only properties; Mind Header and Dashboard give lifecycle modes distinct
visual treatment while runtime availability remains a separate `awake` dimension. Projection age
does not claim that underlying evidence is epistemically fresh.

The focused headless NixOS gate now covers three boot cycles: baseline active-run/identity
continuity, reboot after an owner Event1 commit but before coordinator acknowledgement, and reboot
after terminal Event1 commit but before terminal run-state persistence. Both split-commit replays
reuse their deterministic contributions and leave Event1 count unchanged. This closes the P3
consolidation transaction exit gate.

The focused P4 Plasma VM gate restarts the shipped `plasma-plasmashell.service` around an active
run, observes a replacement PID and restored Plasma D-Bus surface, and proves that neither the
exact persisted lifecycle run nor Event1 count changes across UI recreation.

P6.1 introduced `CapabilitySnapshot`; the current schema v2 keeps component health, capability state, typed
deficit cause, recovery policy, observation/verification time, impact, and evidence/error reference
are encoded separately and validated fail closed. Focused tests cover round trip, unknown schema and
enum rejection, malformed/inconsistent state, dependency/uniqueness invariants, and component
transition legality. At the P6.1 boundary these types had no dependency graph, owner, or D-Bus service.

P6.2 adds `cybou-healthd` as the sole owner of the initial capability dependency graph and the
atomic persistent snapshot under `$XDG_STATE_HOME/cybou/health`. Health1 refreshes public organ
health boundaries, separates required core deficits from optional limitations, preserves the exact
last snapshot across owner restart, fails closed on corrupt state, and requires an explicit
`Recovering` snapshot before a formerly unavailable component becomes healthy. Process integration
proves loss of predictord leaves accepted biography, identity continuity, commitments, and bounded
workspace available; dependent optional capabilities become explicit deficits and recover without
replacing the health owner state.

P6.3 adds typed RPC outcomes and explicit read-only, idempotent-mutation, and non-idempotent-mutation
semantics. The shared async D-Bus client applies bounded deadlines, deterministic exponential
backoff, retry eligibility, and a closed/open/half-open circuit breaker. Plasma lifecycle
interruption is the first production consumer: its timeout is reported as `UnknownOutcome`, is not
retried, does not block the shell event loop, and does not move terminal ownership out of
lifecycled. Other legacy RPC paths remain synchronous until migrated explicitly.

P6.4 adds schema-v1 homeostatic measurements and an observation-only Health1 projection. Each
signal carries its source, kind, unit, observation and validity time, and explicit freshness or
availability status. Healthd collects capability-deficit count, per-owner probe latency, probe
failures, accepted Event1 count, and active lifecycle-run count. Event backlog, Journal size, and
prediction calibration pressure are `Unsupported`, not fabricated zeroes. Measurements are
ephemeral across healthd restart and schema v1 rejects scheduling authority; thresholds,
hysteresis, automatic lifecycle policy, and Presence projection remain P6.5 work.

The P6.4.1/M5-hardening bridge removes three pre-P6.5 ambiguity windows. Health1 now probes owners
in parallel through bounded read-only async RPC, maps timeout into typed deficit cause, rejects
overlapping collection, reacts to D-Bus owner changes with debounce, and performs slow periodic
verification. Predictor and Workspace consolidation derive their computed values only from Event1
sequences at or below the captured high-water mark; replay returns values from the accepted owner
contribution rather than live state. Lifecycle validation requires one non-empty cause for every
missing capability, and completed terminal Outcome payloads durably record completed capabilities,
missing capabilities, and their causes. CapabilitySnapshot schema v2 now emits one record per
unhealthy `(capability, dependency)` pair, so simultaneous owner loss remains fully explainable.
Persisted schema v1 is accepted as a strict compatible subset and normalized to v2; unknown future
versions fail closed.

P6.5 slice 1 connects presenced to Health1 and exports aggregate state, per-capability states,
typed deficits, and observation time through Presence1 and the QML proxy. `awake` remains a
compatibility alias for presentation endpoint reachability; it is no longer the authorization gate
for every command. Biography, commitments, prediction, self-assessment, and attention operations
are independently gated by their declared capabilities. This avoids a health probe cycle:
presenced readiness never depends on healthd readiness, while the capability projection may be
unknown until Health1 is available. Automatic lifecycle policy and complete degraded-mode visual
treatment remain P6.5 work.

P6.5 slice 2 adds an owner-correct scheduling dry run to Lifecycle1. The policy checks lifecycle
idleness, active-run exclusion, a 60-second capability freshness window,
accepted-biography availability, optional predictor/workspace
eligibility, measurement freshness, and a bounded Event1-backlog hysteresis (enter at 32, exit at
8). It returns `Run`, `Defer`, or `Block` with a stable policy ID and causal explanation, and is
projected through Presence1/QML as `lifecycleScheduling`. Evaluation cannot mutate lifecycle state.
At the slice-2 boundary homeostasis schema v1 forbade scheduling authority, so evaluation honestly
deferred instead of starting work.

P6.5 slice 3 makes Event1 the durable owner of consumer progress. Event1 stores versioned consumer
offsets separately from canonical Journal rows, rejects invalid/backward/ahead-of-head movement,
and derives exact backlog from Journal sequence plus the consumer offset. Lifecycled registers the
stable `lifecycle.consolidation` consumer and advances it to the accepted input high-water mark only
after terminal lifecycle state is durable; restart reconciles an already-completed run. Events in
the `lifecycle.consolidation` capability scope are excluded from that consumer's pressure, avoiding
a self-triggering output loop. Health1 now reports `event.backlog.count` as a current typed value
when the consumer is registered.

P6.5 slice 4 introduces Homeostasis schema v2 policy-scoped authorization. The codec accepts schema
v1 only when its legacy boolean is false and normalizes it to no authorized policies. Health1 adds
`event-backlog-v1` only when the durable consumer backlog is current. Lifecycle1 can consequently
return an authorized `Run` decision after its independent capability, freshness, idle-state,
worker, and 32/8 hysteresis checks. Evaluation remains read-only and process integration proves its
state bytes are unchanged.

P6.5 slice 5 adds the explicit idempotent `ExecuteSchedulingDecision` mutation to Lifecycle1.
Evaluation carries exact capability and homeostasis snapshot IDs; execution re-reads Health1 and
rejects either superseded ID before constructing a run. The run UUID is deterministic from the
policy and both evidence IDs, so retry after an unknown reply returns the same active or completed
run. After a newer run replaces the lifecycle projection, the deterministic terminal Event1 ID
still prevents recreation of the old run. Process integration proves stale-evidence rejection,
active/terminal retry, later-run retry, owner dispatch, completion, and consumer-offset advancement.

P6.5 slice 6 adds the bounded production scheduler trigger inside lifecycled. Health1 `Changed` is
debounced by 100 ms and a 30-second timer supplies slow verification. Each trigger returns without
mutation for `Block`/`Defer`; an authorized `Run` executes, dispatches, and completes through the
existing idempotent transaction. An active scheduled run takes precedence over new evaluation:
after restart it resumes from `Recovering` and continues the same run. Fault injection immediately
after durable scheduled-run creation proves restart recovery, owner dispatch, terminal completion,
and zero residual consumer backlog without a duplicate run.

P6.5 slice 7 adds durable user-activity arbitration. Presence commands call
`Lifecycle1.NotifyUserActivity`; lifecycle state schema v2 records `lastUserActivityAt` and
`schedulerCooldownUntil`. Activity wakes an idle lifecycle, defers automatic scheduling for the
configured window (60 seconds by default), and interrupts only active automatic backlog runs.
Cooldown survives restart, while manual maintenance runs retain their active state.

P6.5 slice 8 moves production scheduled owner dispatch onto the shared asynchronous resilient RPC
client. A cycle returns `started`, processes eligible owners sequentially, and completes through
callbacks. Callback acceptance is fenced by run identity and active lifecycle state. Process fault
injection holds predictord inside consolidation while Lifecycle1 accepts activity immediately;
the late contribution cannot turn the interrupted run into a completed run.

P6.5 slice 9 completes the capability explanation boundary in Presence1. Alongside raw deficit
diagnostics, `capabilityDetails` provides one grouped record per known capability with state,
availability, causes, impact, dependencies, verification time, recovery policy, and recovery
progress. Predictor-loss process coverage proves unrelated commands remain usable and the same
record moves from unavailable/waiting through recovering/verifying to available/ready.

P6.5 slice 10 adds the command-side presentation contract. `commandAvailability` and
`canCommand(id)` expose required/missing capabilities without weakening backend enforcement.
Process coverage proves useful commands survive optional predictor loss and explicitly observes
`Awake + Limited` and `Recovering + Limited`; lifecycle mode is not an alias for health. P6.5 is
therefore complete, and implementation focus moves to the P6.6 failure/recovery matrix.

The larger cognitive model and future agency architecture are described in `MIND_MODEL.md`.
M1–M5 form the implemented process-isolated, continuity-preserving substrate of that model. M6 is
the current engineering milestone; the tree does not yet contain the planned M8 language faculty
or M9 authorized executor.

## Process topology

Mind now has nine real user-session processes:

```text
cybou-eventd
cybou-healthd
cybou-lifecycled
cybou-identityd
cybou-intentiond
cybou-predictord
cybou-selfd
cybou-workspaced
cybou-presenced
```

`plasmashell` no longer constructs Identity, Intentions, Predictor, SelfModel, Workspace, Journal,
or EventClient. It loads a lightweight `Presence` QObject whose runtime job is Presence1 IPC and
QML property caching.

## Ownership

| Resource / responsibility | Owner |
|---|---|
| `journal.db` | `cybou-eventd` |
| capability dependency graph and current health snapshot | `cybou-healthd` |
| lifecycle mode and run state | `cybou-lifecycled` under `$XDG_STATE_HOME/cybou/lifecycle` |
| `identity.json` | `cybou-identityd` |
| identity login marker | `cybou-identityd` under `$XDG_RUNTIME_DIR/cybou` |
| intention commands/projection | `cybou-intentiond` |
| prediction/calibration | `cybou-predictord` |
| self projection/assessment | `cybou-selfd` |
| bounded attention | `cybou-workspaced` |
| presentation aggregation | `cybou-presenced` |
| visual cache | Plasma Presence proxy |

There is currently no language-model process and no privileged action-executor process in this
ownership table.

## IPC

Versioned Qt D-Bus interfaces:

```text
org.cybou.Mind.Event1
org.cybou.Mind.Health1
org.cybou.Mind.Lifecycle1
org.cybou.Mind.Identity1
org.cybou.Mind.Intention1
org.cybou.Mind.Predictor1
org.cybou.Mind.Self1
org.cybou.Mind.Workspace1
org.cybou.Mind.Presence1
```

Complex organ projections use fabric CBOR version 1. Event1 CognitiveEnvelope encoding remains
separate from generic projection encoding and from canonical Journal hashing.

## Lifecycle

The NixOS module installs each organ as a `systemd --user` `Type=dbus` service. The services are
D-Bus activated.

They are intentionally not eagerly wanted by the graphical target: the Plasma-hosted proxy first
performs the one-time pre-M1 state-location migration, then its first Presence1 request can
activate the Mind graph.

Identity uses a volatile runtime-session marker. Restarting `identityd` inside the same user login
reloads the current identity without incrementing `sessionCount`.

The process integration suite additionally simulates a new login by removing only the volatile
session marker and restarting the seven M4 processes. Focused booted NixOS gates prove identity,
exact active-run continuity, both split-commit recovery windows, and Plasma recreation across real
system transitions. In-place upgrade reconciliation remains an explicit hardening track. The next
fault-injection matrix belongs to M6 and tests capability-specific partial availability.

## Durable-to-visible ordering

```text
command
→ owning organ process
→ Event1
→ eventd
→ Journal COMMIT
→ Event1 Accepted
→ workspaced admission
→ Workspace1 Changed
→ presenced Changed
→ QML proxy refresh
```

This is the implemented form of the `durable before visible` invariant.

## Current cognitive substrate

The present tree has implementation boundaries for:

- canonical durable causal history;
- identity ownership;
- intention state;
- prediction/calibration state;
- self projection/assessment;
- bounded Workspace attention;
- presentation aggregation;
- process-level health/failure isolation.

These components are intentionally useful without any language model.

## Not implemented yet

The current tree does **not** yet implement:

- in-place upgrade reconciliation beyond the tested schema-v0-to-v1 migration;
- background consolidation, retention, forgetting, or temporal freshness policy;
- M6 Presence health projection and homeostatic scheduling policy beyond the P6.1–P6.4 substrate;
- owner contracts for Event1 backlog, Journal growth, and calibration pressure, and general
  metacognitive uncertainty/evidence-freshness projection;
- M7 inter-node transport, replication, or partition handling;
- typed perception adapters, epistemic claims, contradiction reconciliation, or value constraints;
- M8 optional language faculty;
- M9 planning/authorization/executor pipeline for privileged external actions.

A UI or current organ method should not be described as providing those future capabilities unless
the corresponding milestone is implemented and gated.

## Current limitations

- Health1 probes and most legacy organ RPC remain synchronous; Health1 is not yet projected through Presence;
- most local RPC is synchronous;
- same-user IPC authorization is not yet a capability security boundary;
- stronger in-place upgrade/reconciliation guarantees remain a hardening track;
- Journal history is not yet consolidated into a governed epistemic projection;
- privacy classification exists, but retention and erasure propagation are not implemented;
- no inter-node transport exists;
- no model-selection/context policy for M8 exists;
- no authorization policy or typed privileged executor for M9 exists.

## Milestones

- M1: complete.
- M2: complete.
- M3: complete after the M3 compile repair included by M4.
- M4: implementation present; repository gates remain the acceptance authority.
- M5: evaluation milestone complete; lifecycle, continuity, consolidation transaction, Presence
  projection, process/Plasma/reboot fault injection, and clean VM/ISO evidence are implemented.
- M6: P6.1–P6.4 protocol, dependency graph, health owner, persistent Health1 snapshot, recovery,
  process fault injection, bounded async transport, and policy-scoped homeostatic measurements
  are implemented. P6.5 capability-aware Presence behavior and read-only scheduling policy are
  implemented; authorized automatic scheduling, broader RPC migration, complete degraded UI, and
  KVM gates remain.

See `ROADMAP.md` for the capability meaning of M5–M9.
