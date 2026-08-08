<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0026: Lifecycle Owner and Wire Contract

## Status

Accepted

## Context

ADR-0024 requires lifecycle coordination without creating a second owner of organ state. M5 needs a
concrete process, persistence boundary, transition protocol, and failure semantics.

## Decision

`cybou-lifecycled` will be a separate user-session D-Bus service. It owns only lifecycle mode,
transition/run identity, accepted input high-water mark, work acknowledgements, interruption, and
terminal/recovery metadata. It never opens `journal.db` or organ-owned files.

Persistent run state is rooted under `$XDG_STATE_HOME/cybou/lifecycle`; runtime serialization/lock
state is under `$XDG_RUNTIME_DIR/cybou/lifecycle`. Event1 remains the only durable-biography path.

Wire schema version 1 is represented by `cybou::LifecycleRun`. One active mutating run is allowed.
Every run has a stable UUID and operation identity. Terminal status requires a cause; successful
completion cannot contain missing work. Unknown schema/status and malformed capability sets fail
closed.

Legal mode transitions are defined in `canTransition()`. Direct `Awake → Consolidating` is illegal:
policy first establishes `Idle`. Failure may enter `Degraded` or `Recovering`; suspension resumes
through recovery when verification is required.

## Consequences

The service can fail independently and be added without changing existing organ ownership.
P2/P3 must implement durable reconciliation, D-Bus API, systemd activation, and fault-injection
tests. This ADR accepts the contract, not those later implementation claims.

## Related documents

- `ADR-0024-cognitive-lifecycle-and-consolidation.md`
- `../mind/LIFECYCLE.md`
- `../mind/DATA_OWNERSHIP.md`
