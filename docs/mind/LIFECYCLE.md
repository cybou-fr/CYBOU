<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Cognitive Lifecycle and Consolidation

## Scope

This document refines the M5 target from ADR-0024. It is a design contract, not current M4
behavior. `CURRENT_STATE.md` remains authoritative for implemented functionality.

## Modes

| Mode | Meaning | Normal entry | Normal exit |
|---|---|---|---|
| `Awake` | interactive work is available | session activation, completed recovery | idle, maintenance, suspension |
| `Idle` | no foreground demand; shallow maintenance is allowed | inactivity policy | user activity or consolidation |
| `Consolidating` | bounded owner-specific cognitive maintenance | accepted policy request | completed, interrupted, failed |
| `Maintenance` | integrity, migration, backup, or compaction work | explicit/system policy | awake, suspended, recovering |
| `Recovering` | reconciliation after incomplete or failed transition | restart or detected inconsistency | awake or degraded |
| `Degraded` | lifecycle continues with declared capability deficits | component/capability loss | recovering or prior mode |
| `Suspended` | activity checkpointed across session/system boundary | logout, suspend, shutdown | recovering or awake |

`Degraded` may qualify another operational mode in implementation. The wire representation must
avoid impossible combinations rather than relying on UI strings.

## Coordinator ownership

The coordinator owns only lifecycle mode/reason, run identity, accepted input high-water mark,
requested work, progress, cancellation, terminal result, and recovery metadata. It never owns
identity, intentions, predictions, Self, Workspace, epistemic state, or Journal.

## Run contract

Every run has at least:

```text
runId
kind
policyId
requestedAt
inputHighWaterMark
requiredCapabilities[]
optionalCapabilities[]
status
completedWork[]
missingWork[]
terminalCause
```

The high-water mark makes input deterministic. Contributions accepted later remain valid but are
not silently pulled into the active run.

## Candidate typed events

Names are provisional until protocol/schema work is accepted:

```text
LifecycleTransitionRequested
LifecycleTransitionAccepted
ConsolidationRequested
ConsolidationWorkCompleted
ConsolidationInterrupted
ConsolidationFailed
ConsolidationCompleted
RecoveryRequired
RecoveryReconciled
```

Terminal events cite the request and accepted work supporting the result.

## Owner-specific work

| Owner | Permitted consolidation responsibility |
|---|---|
| `eventd` | integrity verification, backup boundary, accepted run records |
| `identityd` | continuity checkpoint and transition evidence |
| `intentiond` | overdue/expired review proposals and terminal consistency |
| `predictord` | prediction/outcome matching and calibration |
| `selfd` | rebuild self projection from supported evidence |
| `workspaced` | salience decay, episode boundary, bounded reconstruction |
| future epistemic owner | contradiction, freshness, supersession, reconciliation projection |

No row authorizes direct mutation of another owner's storage.

## Scheduling, failure, and recovery

Triggers can include inactivity, screen lock, episode completion, state pressure, explicit user
request, session transition, or recovery. Night-time scheduling is optional policy, not the
architectural definition of sleep.

Deep work normally requires idle time and suitable resource conditions. User activity interrupts
ordinary consolidation. Narrow non-interruptible regions are limited to explicit transactions or
migrations and expose their reason.

- A missing optional capability yields a degraded report when policy permits.
- A missing required capability prevents successful completion.
- An interrupted run records accepted partial work.
- Recovery reads partial/terminal records before retrying.
- Repeated work uses stable operation keys or equivalent idempotency protection.
- Presence says `completed` only after the terminal contribution is accepted.

## Privacy and retention

Summaries inherit the most restrictive privacy of their causes/evidence. A summary does not allow
source deletion unless policy defines how derived material is erased, redacted, or recomputed.
Consolidation logs must not duplicate sensitive payloads unnecessarily.

## M5 acceptance direction

M5 should prove:

- restart during every transition produces one reconcilable result;
- concurrent runs cannot mutate one owner projection without explicit serialization;
- interruption cannot create a false successful terminal state;
- summaries/calibration cite accepted evidence;
- no coordinator path writes owner storage or Journal directly;
- logout/reboot recovery preserves identity and open commitments;
- lifecycle and freshness are visible without making Presence an owner.

## Related documents

- [Mind Model](../MIND_MODEL.md)
- [Continuity](CONTINUITY.md)
- [Failure Modes](FAILURE_MODES.md)
- [Data Ownership](DATA_OWNERSHIP.md)
- [ADR-0024](../adr/ADR-0024-cognitive-lifecycle-and-consolidation.md)

