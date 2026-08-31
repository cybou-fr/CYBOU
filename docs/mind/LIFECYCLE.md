<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Cognitive Lifecycle and Consolidation

## Scope

This document records the implemented lifecycle contract and future extensions from ADR-0024
and ADR-0026. `CURRENT_STATE.md` remains authoritative for the demonstrated implementation boundary.

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
workContributions{capability: contributionId}
missingWork[]
missingCauses{capability: cause}
terminalCause
terminalContributionId
```

The high-water mark makes input deterministic. Production requests use
`RequestRunAtCurrentHead`, so Lifecycle1 captures Event1's accepted count rather than trusting a
caller-supplied boundary. The explicit-mark `RequestRun` remains for compatibility and controlled
recovery/testing. Contributions accepted later remain valid but are not silently pulled into the
active run.

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
- Its non-empty cause is persisted and a conflicting replay cause fails closed.
- A missing required capability prevents successful completion.
- An interrupted run records accepted partial work.
- Recovery reads partial/terminal records before retrying.
- Repeated work uses stable operation keys or equivalent idempotency protection.
- Presence says `completed` only after the terminal contribution is accepted.

Lifecycle1 derives each current operation key as `runId:capability:inputHighWaterMark`. Owner
acknowledgements must return that key and the accepted high-water mark. Repeating an already
persisted acknowledgement is a successful no-op; a mismatched key or mark fails closed. Recovery
resumes the existing active run and therefore derives the same keys rather than creating new work.

`Dispatch` currently routes the `predictor` and `workspace` capabilities to their owning D-Bus
services. Each owner rejects a high-water mark beyond its accepted Event1 count and returns a typed
receipt containing the owner, operation key, mark, acceptance decision, and durable contribution
ID. The owner resolves the exact Event1 envelope at that mark and commits a deterministic UUIDv5
`Learning` contribution which cites it as causation. Lifecycle1 persists an acknowledgement only
after validating every receipt field and Event1 acceptance, atomically recording the capability to
contribution mapping. Repeating Dispatch skips already persisted work, while direct owner
redelivery returns the same contribution without another append. Completion commits one
deterministic Event1 `Outcome` caused by every recorded owner contribution, saves its ID in the run,
and fails closed if any reference is absent. Lifecycle1 state commits emit `Changed`; presenced
projects mode, run status/state, and lifecycled health through Presence1, while the desktop client keeps
runtime availability (`awake`) separate from lifecycle mode. The process suite covers crashes after owner and terminal Event1
commit but before the corresponding lifecycle state commit; deterministic replay creates no second
contribution. The automated test gate promotes both split-commit windows to real reboot recovery
and proves replay leaves Event1 count unchanged; the P3 transaction exit gate is complete.

Owner computation is also bounded by that mark: Predictor and Workspace reconstruct the data used
for their result only from Event1 sequences at or below it. Later accepted contributions cannot
change the first result, and replay reads the value from the already accepted owner contribution.
Every `missingWork` entry must have exactly one non-empty `missingCauses` entry. The terminal
Outcome carries completed capabilities, missing capabilities, and missing causes so degraded
completion remains durable evidence rather than only current lifecycle state.

Lifecycle state mutations use rollback-on-save-failure semantics: if atomic file replacement fails,
the service restores its previous in-memory run/mode instead of exposing state that was never
persisted. Unknown lifecycle status values also fail schema validation.

## Privacy and retention

Summaries inherit the most restrictive privacy of their causes/evidence. A summary does not allow
source deletion unless policy defines how derived material is erased, redacted, or recomputed.
Consolidation logs must not duplicate sensitive payloads unnecessarily.

## Acceptance evidence

Evaluation proves:

- restart during every transition produces one reconcilable result;
- concurrent runs cannot mutate one owner projection without explicit serialization;
- interruption cannot create a false successful terminal state;
- summaries/calibration cite accepted evidence;
- no coordinator path writes owner storage or Journal directly;
- logout/reboot recovery preserves identity and open commitments;
- lifecycle and freshness are visible without making Presence an owner.

Presence derives a read-only lifecycle projection from `Lifecycle1.State`: resolved/total work and
percentage, a semantic progress class, explicit `{ capability, cause }` deficits, and request age.
Freshness is `current` below five minutes, `aging` below one hour, `stale` thereafter, and `unknown`
when no run timestamp exists. These thresholds describe projection age, not evidence validity.
`awake` continues to mean runtime availability and is never inferred from lifecycle mode.

The desktop client sends user interruption asynchronously to Presence1 and exposes a read-only pending
flag. Presence1 validates that a run is active and asks Lifecycle1 to persist `Interrupted`; neither
the client nor presenced writes lifecycle state. The five-second transport timeout completes as an
`UnknownOutcome` error without blocking the client event loop. Inside Presence1, validation and
`FinishRun` share one monotonic server budget, and expiry before the terminal call prevents the
mutation from being sent.

Ordinary Presence commands also report user activity before their capability gate. Lifecycle1
persists the activity timestamp and a 60-second scheduler cooldown, wakes `Idle`, and interrupts
only automatically scheduled backlog work. A manual maintenance run is not implicitly cancelled.
The cooldown is evaluated again immediately before run creation and remains effective after a
lifecycled restart.

Automatically scheduled owner work is dispatched sequentially with asynchronous idempotent RPC.
Lifecycle1 stays available for activity while an owner is working. A callback is accepted only if
its captured run ID still names the active consolidating run; a reply arriving after interruption
is ignored for lifecycle progress and cannot produce a false successful terminal state.

## P2 continuity matrix

| Boundary | Current automated evidence | Remaining evidence |
|---|---|---|
| service object reconstruction | atomic reload and active-run recovery | none for P2 scope |
| daemon process restart | D-Bus restart preserves run identity and enters `Recovering` | none for P2 scope |
| duplicate daemon | second Lifecycle1 owner exits non-zero | none |
| corrupt lifecycle state | startup fails closed | backup/operator recovery policy |
| identity continuity | simulated login and booted reboot preserve UUID and increment logical session | none for P2 scope |
| open intentions | simulated login reconstructs accepted commitment | booted reboot proof |
| system reboot | automated test gate preserves identity and exact run blob, then enters `Recovering` | none for P2 scope |
| architecture upgrade | legacy v0/v1 backup+migration to v2; future schema fails closed | multi-version migrations |

## Related documents

- [Mind Model](../MIND_MODEL.md)
- [Continuity](CONTINUITY.md)
- [Failure Modes](FAILURE_MODES.md)
- [Data Ownership](DATA_OWNERSHIP.md)
- [ADR-0024](../adr/ADR-0024-cognitive-lifecycle-and-consolidation.md)
