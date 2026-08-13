<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0028: Retention and Erasure in an Append-Only Journal

## Status

Proposed

This is the storage ADR that [ADR-0027](ADR-0027-local-epistemic-projection-owner.md) names as the
precondition for ingesting any sensitive observation. That constraint is binding, so until this is
Accepted and implemented, perception stays on sources chosen to make the constraint costless.

## Context

Mind's memory is an append-only hash-chained Journal with a single writer. That is what makes its
biography auditable: any contribution can be shown to have been recorded when it claims, in the
order it claims, unaltered since.

[ADR-0025](ADR-0025-grounding-epistemics-and-cognitive-governance.md) also requires that forgetting
actually stop storing sensitive content while keeping auditable semantics. Those two requirements
are in direct tension, and the tension is structural rather than incidental:

- `canonicalEnvelopeV2` includes `payloadCbor` **by value**, and the row hash covers that canonical
  encoding. Erasing a payload therefore breaks the chain from that row onward.
- Deleting a row is worse. Sequence numbers and `prev_hash` linkage would show a hole, and
  `contribution_evidence` holds `ON DELETE RESTRICT` foreign keys precisely so that evidence cannot
  be silently orphaned.
- Derived state holds copies. epistemicd persists a projection checkpoint; predictord keeps
  per-subject samples in memory; intentiond reconstructs from replay. Erasing the Journal alone
  leaves the content alive in caches that outrank nothing but are read by everything.
- Backups taken before an erasure still contain the content, and a backup is not reachable by a
  transaction against the live database.

An erasure design that addresses only the first of these is not erasure. It is a redaction of one
copy, and the honest thing is to say which copies a mechanism actually reaches.

## Decision

### The unit of erasure is the payload, never the record

A contribution's **identity, causality, and position** — `message_id`, `correlation_id`,
`causation_id`, `origin_organ`, `kind`, `sequence`, timestamps, `privacy`, evidence edges — are
never erased. Only `payloadCbor` is. What was thought, by whom, in what order, and on the evidence
of what, remains provable; what was *said* becomes unavailable.

This keeps the biography's shape intact. A Mind that could forget that it once concluded something
could not be audited, and could not later explain why it changed its mind.

### Row hash v3 chains an envelope digest, not an envelope

Introduce `hash_version = 3`:

```
row_v3 = SHA256("CYBOU-JOURNAL-ROW-V3" ‖ u16(3) ‖ u64(seq) ‖ bytes(prev_hash) ‖ bytes(envelope_hash))
envelope_hash = SHA256(canonicalEnvelopeV2(envelope))
```

with `envelope_hash` stored in a new column. `canonicalEnvelopeV2` is unchanged, so the wire
contract and every existing signature over an envelope are unaffected.

The chain is then verifiable **without the payload**, because linkage is over stored digests. Two
distinct checks replace today's single one:

- **Chain integrity** — every row's `hash` follows from `prev_hash` and `envelope_hash`. Always
  checkable, erased or not.
- **Content integrity** — `envelope_hash == SHA256(canonicalEnvelopeV2(envelope))`. Checkable only
  where the payload is present; **reported as skipped, never as passed**, where it is not.

`VerificationResult` gains that distinction. A verification that silently counted erased rows as
verified would be the same defect as a replay that treats a failed page as the end of history.

Existing rows keep `hash_version` 1 and 2 and remain verifiable as they are. Erasure is available
only for rows written at v3, because a v1 or v2 row's hash cannot be recomputed without its payload.
Nothing is rewritten in place: a hash chain that can be migrated retroactively is not a hash chain.

### An erasure is itself a contribution

Erasure is requested by appending an `Erasure` contribution naming the target and the reason. The
single writer then, **in one transaction**, nulls the target payload, sets its `erased_at`, and
appends the record. Either both happen or neither does.

So the fact of forgetting is itself remembered, and is auditable on the same terms as everything
else. There is no side channel that mutates the Journal without leaving a trace in it — which is the
property that makes the single-writer rule worth having.

An erasure record is never itself erasable. It names a target and a reason and carries no
observation content, and a forgetting that could be forgotten would make the audit trail a
suggestion.

### Derived state is invalidated by construction

The Journal carries an **erasure epoch**, incremented on every erasure and readable over Event1.
Every persisted projection records the epoch it was built under. A projection whose stored epoch is
behind the Journal's is discarded and rebuilt, not repaired.

This is deliberately blunt. Working out which derived value depended on an erased payload requires
exactly the payload that is gone, so precise invalidation is not available; rebuilding is, and it
costs a replay that the measured budgets say is affordable. epistemicd's checkpoint is already a
cache the Journal outranks, and predictord's projection already rebuilds from a cursor, so both
already have the machinery this needs.

In-memory projections rebuild on the same signal rather than on restart. An organ that only noticed
an erasure when it happened to restart would serve erased content for an unbounded time.

### Backups are addressed by key, not by deletion

Erasure by nulling reaches the live database and nothing else. For sensitive payloads, that is not
enough, so **sensitive payloads are encrypted per contribution** with a key held in a local key
store, and erasure destroys the key as well as the ciphertext.

A backup then retains ciphertext whose key no longer exists anywhere. This is the only mechanism
here that reaches copies Mind does not control, and it is the reason it is worth its cost.

The limits are stated rather than glossed:

- It is **cryptographic** erasure. It assumes the cipher holds and the key was not separately
  captured. It is not the same claim as overwriting a disk.
- The key store becomes as sensitive as the payloads and must not itself be backed up alongside
  them.
- A backup of the key store taken before an erasure defeats it. The key store is therefore excluded
  from ordinary backup, and that exclusion is part of this decision rather than an operational
  detail left to whoever configures it.

### Retention classes ride on privacy, not beside it

`PrivacyClass` already propagates to the most restrictive class across evidence. Retention attaches
there rather than in a parallel table: a class carries a default lifetime, and a derived
contribution inherits the shortest lifetime among its evidence for the same reason it inherits the
most restrictive privacy. Two independent classification schemes over the same records would
disagree, and the disagreement would be discovered by a leak.

## Consequences

- The Journal gains a column, a hash version, an epoch, and an erasure record kind. All are
  additive; no existing row changes.
- Verification answers a third thing — how many rows could not be content-checked — and every caller
  of `VerifyIncremental` has to decide what that means to it. That is the intended cost.
- Encrypting sensitive payloads makes them opaque to the Journal's own indices. Anything that needs
  to be searchable must live in the envelope's non-payload fields, which are never erased and
  therefore must never be sensitive. This constrains adapter design and is the main reason to accept
  this ADR *before* writing the adapters rather than after.
- Erasure is bounded by what a single writer can do transactionally, so it is not free at scale: a
  request naming a large set is many transactions, and the epoch bump forces every projection to
  rebuild once regardless of how many payloads were erased.

## What this does not decide

- **Automatic expiry.** Lifetimes are declared here; a policy engine that acts on them is separate
  work, and acting on a lifetime is an erasure like any other.
- **Which sources are sensitive.** That is per-adapter and belongs with each adapter's ADR.
- **Remote or replicated Journals.** ADR-0018 governs replication; erasure across nodes needs the
  distributed prototype to exist first, and the key-destruction mechanism is what will make it
  tractable when it does.

## Related documents

- [ADR-0011: Single Writer Event Journal](ADR-0011-single-writer-event-journal.md)
- [ADR-0018: Privacy Classification and Replication](ADR-0018-privacy-classification-and-replication.md)
- [ADR-0025: Grounding, Epistemics, and Cognitive Governance](ADR-0025-grounding-epistemics-and-cognitive-governance.md)
- [ADR-0027: Local Epistemic Projection Owner](ADR-0027-local-epistemic-projection-owner.md)
- [Journal Scale Baseline and Budgets](../mind/SCALE_BUDGETS.md)
