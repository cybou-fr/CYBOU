<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Capability and Health Contract

## Scope

P6.1 defines a versioned wire contract for component observations and capability deficits. It does
not yet implement the dependency graph, `cybou-healthd`, `Health1`, retry policy, or Presence UI.
Those remain P6.2 and later work.

Component health and capability availability are deliberately separate. A stopped optional organ
can remove one capability without making identity, biography, or commitments unavailable.
Lifecycle mode is also orthogonal: a Mind may be `Awake` while a capability is `Limited`.

## Schema v1

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
- snapshot, component, and capability identifiers are non-empty and unique in their scope;
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

The protocol library owns encoding and validation only. The planned M6 health owner will own the
dependency graph and current capability projection. Organs retain domain state, lifecycle retains
run state, Event1 retains accepted history, and Presence remains a read-only aggregator.

## Automated evidence

`health-protocol` covers schema-v1 round trip, typed causes and recovery policy, unknown schema and
enum rejection, malformed/inconsistent payload rejection, uniqueness/dependency invariants, and
component transition legality.
