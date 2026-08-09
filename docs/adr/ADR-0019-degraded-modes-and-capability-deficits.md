<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0019: Degraded Modes and Capability Deficits

## Status

Accepted

## Context

A process-isolated Mind must remain honest when an organ is unavailable, slow, recovering, or
returning an uncertain result. M4 created independent failure domains and M5 added lifecycle-level
deficits, but the current `Ready()/Health()` aggregation still treats most dependencies as a single
boolean. This can preserve processes while still hiding which useful capabilities remain.

## Decision

Represent component health separately from capability availability.

Component health uses `Starting`, `Healthy`, `Degraded`, `Unavailable`, `Recovering`, and
`Conflicted`. Capability state uses `Available`, `Limited`, `Unavailable`, `Unknown`, `Stale`, and
`Recovering`.

Every capability deficit identifies:

- the affected capability and dependency;
- a typed cause and detection time;
- the last verified successful state where known;
- operational impact;
- retry or recovery policy;
- causal evidence or an error reference where available.

An explicit dependency graph determines aggregate Mind state. Optional-organ failure removes or
limits only dependent capabilities; it does not make independent identity, biography, intention,
or presentation capabilities unavailable. Lifecycle mode and capability health remain orthogonal.

A dedicated health/capability owner is preferred over placing durable health policy in Presence.
Presence consumes a read-only projection and remains a presentation aggregator. Significant health
transitions cross Event1; routine probes do not become biography noise.

Retries are bounded and limited to operations whose idempotency semantics permit them. Timeout,
rejection, unavailability, and unknown outcome remain distinct. Automatic retry must never turn an
unknown mutation result into an invented success.

## Consequences

Partial failure no longer becomes fictional success or automatic death of the whole Mind. The
design adds a versioned protocol, dependency policy, recovery rules, projection work, and
fault-injection tests before it can be claimed as implemented M6 behavior.

## Alternatives Considered

A generic awake flag and a single aggregate health string were rejected as sufficient reporting.
Making Presence the durable health owner was rejected because UI/presentation recreation must not
change cognitive availability state. Unbounded retry was rejected because it obscures latency and
unknown outcomes and can duplicate non-idempotent effects.
