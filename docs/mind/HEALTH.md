<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Capability and Health Contract

## Scope

A versioned wire contract carries component observations and capability deficits. It
implements the initial dependency graph, `cybou-healthd`, persistent snapshot ownership, and
`Health1`. Health1 probing uses bounded async transport. Schema v2 preserves every
unhealthy dependency of a capability. Presence1 now projects the snapshot, UI-ready capability
details, command availability, and recovery progress without becoming a second health owner.

Component health and capability availability are deliberately separate. A stopped optional organ
can remove one capability without making identity, biography, or commitments unavailable.
Lifecycle mode is also orthogonal: a Mind may be `Awake` while a capability is `Limited`.

## Schema v2

`CapabilitySnapshot` contains:

```text
schemaVersion
snapshotId
observedAt
aggregateState
components[]
deficits[]
```

A component record contains its stable ID, component-health state, observation time, optional last
verified time, and diagnostic detail. A deficit contains the capability, dependency, capability
state, typed cause, detection time, optional last verified time, operational impact, recovery
policy, and optional Event1 evidence ID or error reference.

Schema v2 changes deficit identity from `capabilityId` to the pair
`(capabilityId, dependencyId)`. One capability may therefore report several simultaneous causes.
The decoder accepts persisted schema v1 as the compatible one-deficit-per-capability subset,
normalizes it to v2 in memory, and the next successful refresh atomically rewrites v2. Unknown
future versions fail closed.

## State vocabularies

Component health:

```text
Starting | Healthy | Degraded | Unavailable | Recovering | Conflicted
```

Capability availability:

```text
Available | Limited | Unavailable | Unknown | Stale | Recovering
```

Deficit cause:

```text
DependencyUnavailable | DependencyDegraded | TimedOut | Rejected
UnknownOutcome | StaleEvidence | ConflictingState
```

Recovery policy:

```text
None | Observe | RetryIdempotent | Reconcile | OperatorRequired
```

`UnknownOutcome` is not interchangeable with failure: a mutation may have committed even when its
reply was lost. Only operations with an explicit idempotency contract may use `RetryIdempotent`.

## Validation invariants

- unknown schema versions and enum values fail closed;
- snapshot, component, and capability identifiers are non-empty; components are unique by ID and
  deficits are unique by `(capabilityId, dependencyId)`;
- deficit dependencies name a component present in the same snapshot;
- observation and verification times cannot claim knowledge from the future;
- `Available` has no deficit; a snapshot with deficits cannot aggregate to `Available`;
- malformed CBOR, missing required fields, and structurally inconsistent records are rejected;
- the codec does not infer dependency or retry policy from UI strings.

## Component transition baseline

Startup may resolve to a verified operational/failure state. A component cannot jump directly from
`Unavailable` to `Healthy`; it must enter `Recovering`. `Conflicted` similarly requires recovery or
explicit unavailability before health can be asserted. Re-entering the same state is not a
transition and should not create an Event1 record.

## Ownership boundary

The protocol library owns encoding and validation only. `cybou-healthd` owns the dependency graph
and current capability projection at `$XDG_STATE_HOME/cybou/health/snapshot.cbor`. Organs retain
domain state, lifecycle retains run state, Event1 retains accepted history, and Presence remains a
read-only aggregator. Corrupt persisted health state fails closed; atomic replacement prevents a
partially written snapshot from becoming current.

## Initial dependency policy

The graph defines required identity-continuity, accepted-biography, and commitment-access
capabilities, plus optional prediction, self-assessment, attention/workspace, consolidation, and
Presence presentation. Aggregate state is `Unavailable` or `Unknown` when a required capability
cannot be supported and `Limited` when only optional capability deficits remain.

`Health1.Refresh` observes public D-Bus `Ready/Health` boundaries and never opens owner storage.
Event1 predates the common `Health()` method, so its successful typed `Ready()` is the compatibility
health boundary. A component returning from `Unavailable` or `Conflicted` enters `Recovering` for a
verified snapshot before it may become `Healthy`/`Available`.

Refresh fans out through parallel read-only `AsyncRpcClient` probes. Each probe has a
750-millisecond deadline and no retry inside one sample; the collection has a two-second common
deadline. Timeout maps to a typed deficit instead of blocking the observer. D-Bus owner changes
trigger a debounced refresh, a slow verification timer runs every 30 seconds, explicit `Refresh`
remains available, and overlapping collection is rejected.

## Automated evidence

`health-protocol` covers schema-v2 round trip, v1 migration, pair uniqueness, and validation.
`health-service` covers graph policy,
required/optional aggregation, atomic persistence/reload, and corrupt-state rejection.
`healthd-integration` exercises Health1 ownership, optional-owner loss, preserved independent core
capabilities, exact snapshot recovery after healthd restart, explicit recovery, and duplicate-owner
rejection under a real D-Bus session.
