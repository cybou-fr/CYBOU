<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0024: Cognitive Lifecycle and Consolidation

## Status

Accepted

Implemented in M5 and given its concrete owner and wire contract by the Accepted
[ADR-0026](ADR-0026-lifecycle-owner-and-wire-contract.md); leaving this Proposed while its
implementation ADR was Accepted was a contradiction in the record.

## Context

The M1–M4 substrate accepts durable events and projects a bounded current moment, but it has no
explicit lifecycle for turning accumulated experience into maintained cognitive state. Human sleep
is a useful product metaphor for this missing cycle, but copying a biological sleep process would
create the wrong software boundary.

A central `sleepd` that rewrites identity, intentions, predictions, Self, Workspace, or Journal
would become a hidden second owner of Mind. Conversely, leaving every organ to run unrelated
background jobs provides no coherent transition, interruption, or recovery semantics.

## Decision

Mind gains explicit lifecycle modes:

```text
Awake
Idle
Consolidating
Maintenance
Recovering
Degraded
Suspended
```

These names describe software availability and maintenance state. They do not claim biological
sleep or consciousness.

A future lifecycle coordinator MAY:

- propose a transition when policy conditions are met;
- obtain a causally bounded snapshot/high-water mark;
- request typed maintenance work from the owning organs;
- monitor progress, cancellation, and capability deficits;
- submit consolidation results through Event1;
- publish a lifecycle report after accepted completion.

The coordinator MUST NOT:

- write `journal.db` directly;
- mutate identity, intention, prediction, Self, or Workspace storage directly;
- silently delete or rewrite accepted contributions;
- treat a summary as replacement evidence for its sources;
- claim successful completion without an accepted terminal record;
- prevent urgent user work from interrupting non-critical consolidation.

Each organ remains the only owner of its projection. For example, `predictord` calibrates
predictions, `intentiond` reconciles commitments, and `workspaced` rebuilds or decays transient
attention. `eventd` remains the single canonical Journal writer.

## Consolidation protocol

A consolidation run has a durable identity and bounded input:

```text
ConsolidationRequested
→ snapshot/high-water mark accepted
→ typed work requested from owners
→ derived results cite causes/evidence
→ ConsolidationCompleted | ConsolidationInterrupted | ConsolidationFailed
```

Runs MUST be idempotent or detect prior completion. Restarting after interruption MUST reconcile
accepted partial results instead of duplicating them. New observations arriving after the input
high-water mark belong to a later run unless an explicit policy safely rebases the run.

Possible owned work includes:

- integrity verification and recovery preparation;
- prediction/outcome calibration;
- intention review and expiry proposals;
- contradiction detection and reconciliation proposals;
- episode closure and derived summaries;
- salience decay and Workspace reconstruction;
- retention review and privacy-preserving deletion proposals;
- schema migration and compaction under explicit policy.

### Learning work is scheduled, not owned, by lifecycle

Consolidation **may** schedule bounded learning work under [ADR-0032](ADR-0032-layered-lifelong-learning.md)
and [ADR-0033](ADR-0033-learned-artifact-governance.md): candidate extraction from accepted
experience, replay and evaluation of proposed generalizations, procedural skill induction, bounded
training or build snapshots, optional local adaptation, and promotion or rejection work.

`lifecycled` owns none of those learned domains. It owns transition and orchestration semantics
only, and any future mutable learning owner requires its own ownership and wire-contract decision
before it is implemented.

Candidate generation is ordinary interruptible work. **A completed training run is not a completed
promotion** — the two are separate events precisely because a process that finished is the easiest
thing to mistake for a decision that was made.

A consolidation run that produces a candidate records the source high-water mark it used. Accepted
observations beyond that mark belong to later learning work, not to a silent rebase of this one.

Consolidation produces derived state. It never changes what historically happened.

## Triggers and interruption

Policy may request shallow or deep consolidation after idle, screen lock, session transition,
resource pressure, an episode boundary, or explicit user action. Wall-clock night is not a required
trigger.

Shutdown/logout hooks may request a short checkpoint but MUST NOT assume unlimited completion time.
Critical integrity or migration work may be non-interruptible only inside a narrowly defined
transaction. Ordinary consolidation yields to user activity.

## Consequences

Continuity gains an active maintenance/recovery cycle rather than only persistent files.

The system can explain whether state is fresh, consolidating, interrupted, or awaiting
reconciliation. Presence can show lifecycle progress without owning it.

Implementation requires typed lifecycle records, scheduling policy, high-water marks, idempotency,
and tests across restart, reboot, interruption, and partial failure.

## Milestone direction

Lifecycle modes, checkpoints, and basic consolidation belong to M5. Capability-aware scheduling,
resource pressure, and degraded consolidation belong to M6. Epistemic reconciliation and retention
work mature with M7.

## Related documents

- `../MIND_MODEL.md`
- `../ARCHITECTURE.md`
- `../ROADMAP.md`
- `ADR-0011-single-writer-event-journal.md`
- `ADR-0016-identity-continuity.md`
- `ADR-0019-degraded-modes-and-capability-deficits.md`

