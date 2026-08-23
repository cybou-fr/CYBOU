<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Homeostatic Measurements

## Boundary

Observation comes before autonomous policy. `cybou-healthd` owns an immutable in-memory
`HomeostasisSnapshot` refreshed together with its capability snapshot. It reads public D-Bus
contracts and never opens Journal or another organ's storage. Schema v2 can authorize a named,
reviewed scheduling policy but still cannot request or interrupt a lifecycle run itself.

Homeostatic snapshots are intentionally not persisted. A recreated health owner must collect new
evidence; it must not present an old runtime measurement as current.

## Schema v2

Each snapshot carries a schema version, UUID, UTC observation time, a unique list of authorized
policy IDs, and a non-empty unique list of measurements. Authorization is policy-scoped rather
than a global scheduling boolean. Schema v1 is accepted only as an observation-only migration
input; its `schedulingAuthorized` value must be false and normalizes to an empty policy list.
Each measurement carries:

- stable metric and source identifiers;
- kind (`Gauge`, `Counter`, `Duration`, or `Bytes`) and unit;
- status (`Current`, `Stale`, `Unknown`, or `Unsupported`);
- observation and validity times for value-bearing states;
- a finite numeric value only when a value is known;
- a reason instead of a fabricated value for `Unknown` and `Unsupported`.

`Current` expires at `validUntil`. `Stale` retains its last observed value but its validity time is
before the snapshot observation. Unknown schema versions, enum values, duplicate identifiers,
wrong CBOR types, non-finite values, and inconsistent freshness fail closed.

## Health1 projection

Health1 exposes `HasMeasurements()` and `Measurements()`. `Refresh()` collects a new measurement
snapshot only after the durable capability snapshot can be committed. `Changed` then announces
both projections. Restart reloads the durable capability snapshot but reports no measurements
until the next refresh.

| Metric | Source | Kind/unit | Current support |
|---|---|---|---|
| `health.capability-deficit.count` | healthd | counter / `{deficit}` | yes |
| `rpc.probe.latency.<component>.ms` | healthd | duration / `ms` | yes |
| `rpc.probe-failure.count` | healthd | counter / `{probe}` | yes |
| `event.accepted.count` | eventd | counter / `{event}` | yes, otherwise `Unknown` |
| `lifecycle.active-run.count` | lifecycled | counter / `{run}` | yes, otherwise `Unknown` |
| `event.backlog.count` | eventd | counter / `{event}` | yes for registered consumer, otherwise `Unknown` |
| `journal.storage.bytes` | eventd | bytes | `Unsupported`: no public owner metric |
| `prediction.calibration-pressure` | predictord | gauge | `Unsupported`: policy is undefined |

Event backlog is derived from Event1's durable `lifecycle.consolidation` consumer offset; events in
that same capability scope are excluded to prevent self-triggering. Unsupported storage and
calibration pressure are not represented as zero. Health1 authorizes `event-backlog-v1` only when
the consumer backlog is current. Lifecycle1 separately validates capability state, freshness,
idleness, worker eligibility, and hysteresis before returning `Run`.

## Evidence

Protocol tests cover round-trip and malformed input rejection. Service tests prove unsupported
signals have no value and measurements are not recovered as current after restart. Process
integration proves Event1 count and consumer backlog are observed without appending an event,
authorization is scoped to `event-backlog-v1`, and evaluation alone does not create a run.
