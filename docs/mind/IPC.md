<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Local Cognitive IPC

## Interfaces

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

## Encoding

Event1 keeps the M3 versioned CognitiveEnvelope CBOR.

Other organ projections use fabric CBOR version 1:

```text
{
  version: 1,
  value: <typed QVariant/QCbor value>
}
```

This representation is not canonical Journal hashing.

## Baseline service contract

Every organ exposes at least:

```text
Ready() -> bool
Health() -> string
```

Health1 serves schema-v2 `CapabilitySnapshot` CBOR and accepts persisted schema v1 as a migration
input. Health1 exposes
`Ready`, aggregate `Health`, `LastError`, `HasSnapshot`, `Snapshot`, `HasMeasurements`,
`Measurements`, `Refresh`, and `Changed`.
The snapshot uses its own versioned protocol encoding rather than the generic fabric wrapper.
`Measurements` uses the separate schema-v2 homeostasis encoding. It carries policy-scoped
authorization and accepts schema v1 only as an observation-only migration input.

Event1 owns durable consumer progress outside the immutable Journal schema:
`EnsureConsumer(id, initialOffset)`, `AdvanceConsumer(id, offset)`, and `ConsumerBacklog(id)`.
Consumer IDs are bounded stable identifiers; offsets are monotonic, may not exceed Journal head,
and are atomically persisted. The `lifecycle.consolidation` backlog excludes contributions in its
own capability scope, preventing consolidation output from scheduling itself again.

Lifecycle1 additionally exposes `EvaluateScheduling`. It returns a fabric-CBOR dry-run decision:
`run`, `defer`, or `block`; policy/reason; observation time; hysteresis state; eligible workers;
typed causes for missing optional workers; and the exact capability/homeostasis snapshot IDs.
Evaluation reads Health1 but does not mutate the
Lifecycle1 run or mode. A `run` decision means the named policy is authorized and its typed gates
passed; it still does not create a lifecycle run.

`ExecuteSchedulingDecision(capabilitySnapshotId, homeostasisSnapshotId)` re-reads Health1 and
requires both IDs to remain current before it creates a bounded consolidation run. Its run UUID is
deterministic from policy plus both evidence IDs. Retrying after a timeout returns the same active
or completed run ID, including after a later run replaced the in-memory projection; the durable
terminal Event1 contribution closes that idempotency window.

`RunSchedulingCycle()` is the bounded orchestration entry point. It returns `blocked`, `deferred`,
`failed`, or `started` with a run ID and reason. For a new authorized decision it creates the
transaction and starts sequential asynchronous owner dispatch; terminal completion arrives through
`Changed`. If lifecycled restarts with an active scheduled run in `Recovering`, the same method
resumes and continues it rather than evaluating a second run.

`NotifyUserActivity(cause)` is the Presence-to-Lifecycle arbitration command. It durably records
activity and a scheduler cooldown, wakes `Idle`, and interrupts an active automatically scheduled
`event-backlog-v1:*` run. It deliberately leaves manual runs active. `State()` projects
`lastUserActivityAt`, `schedulerCooldownUntil`, and `schedulerCooldownActive`.

## Resilient asynchronous calls

Calls that must not block a shell or owner event loop use the typed policy in
[RPC Resilience](RPC_RESILIENCE.md). Retry requires explicit read-only or idempotent semantics;
non-idempotent timeout remains `UnknownOutcome` and is never automatically replayed.

## Presentation signal ordering

workspaced emits `Workspace1.Changed` after it admits an Event1 accepted contribution.
presenced converts that to `Presence1.Changed`. QML then refreshes its cached snapshot.
