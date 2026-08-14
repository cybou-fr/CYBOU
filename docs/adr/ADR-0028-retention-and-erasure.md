<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0028: Retention and Erasure in an Append-Only Journal

## Status

Accepted

This is the storage ADR that [ADR-0027](ADR-0027-local-epistemic-projection-owner.md) names as the
precondition for ingesting any sensitive observation.

Accepted before implementation, deliberately. The point of this decision is to freeze the contract
that Journal v3 will be written against; leaving it Proposed while the code is written would mean
the format decided the ADR rather than the reverse.

**Acceptance does not enable sensitive perception.** Sensitive observations remain prohibited until
the storage, key-management, erasure-propagation, projection-invalidation and recovery gates at the
end of this document are implemented and green. ADR-0027 requires both an accepted decision and
working retention semantics, and only the first of those exists today.

Being Accepted, this outranks [Current State](../CURRENT_STATE.md): where an implementation and this
document disagree, the implementation is wrong.

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

### Row hash v3 commits to an opaque value, not to a plaintext digest

Introduce `hash_version = 3`:

```
metadataDigest    = SHA256(canonicalNonErasableEnvelopeV3(envelope))
payloadCommitment = sensitive ? SHA256(nonce ‖ ciphertext ‖ tag)
                              : SHA256(payloadCbor)
commitment        = SHA256(metadataDigest ‖ payloadCommitment)

row_v3 = SHA256("CYBOU-JOURNAL-ROW-V3" ‖ u16(3) ‖ u64(seq) ‖ bytes(prev_hash) ‖ bytes(commitment))
```

with `commitment` stored in a new column. `canonicalEnvelopeV2` is unchanged, so the wire contract
and every existing signature over an envelope are unaffected.

**The commitment covers the metadata as well as the payload, and it must.** An earlier draft
committed to the payload alone. That would have left `messageId`, `causationId`, `originOrgan`,
`kind`, `wallTime`, `privacy`, `retention` and the evidence structure outside the chain, so a
contribution's *author* could be rewritten from `perceptiond` to anything at all without disturbing
a single hash. Provenance binding is the property P7.0 was built to obtain; a hash version that
quietly dropped it would have undone that work in the name of forgetting.

`canonicalNonErasableEnvelopeV3` is exactly the fields erasure never touches. That is not a
coincidence: what survives erasure is precisely what must stay verifiable afterwards.

**Both halves are stored, not only their combination.** A row keeps `commitment` *and*
`payload_commitment`, because after an erasure the payload commitment can never be recomputed — and
a verifier that had to recompute it in order to check the metadata would lose the ability to check
the metadata at exactly the moment forgetting made it unrecomputable. Storing it separately is what
makes an erased row's author still provable.

**The canonical field set is selected by envelope schema version, never extended in place.**
Retention will add `retentionClass`, `retainUntil` and retention dependencies to the envelope, and
appending them to the existing v3 encoding would change the digest of every row already written.
So envelope schema 2 keeps today's field set and schema 3 adds the retention fields, both under
journal `hash_version = 3`. A row is canonicalised by the schema it was written under.

The chain is then verifiable **without the payload**, because linkage is over stored commitments.

**What the payload commitment is depends on whether the payload is sensitive.** An earlier draft
used `SHA256(canonicalEnvelopeV2)` for everything and kept it after erasure. That is a permanent
guessing oracle: for the payloads that most need erasing — a diagnosis, a boolean, a small enum, a
name, one of a handful of known configurations — the search space is small enough to enumerate, so
anyone holding the digest can confirm the erased value. Erasure that leaves behind a verifier for
what was erased is not erasure.

So:

- **Non-sensitive payloads** commit to `SHA256(payloadCbor)`. The plaintext is not secret, so an
  oracle for it costs nothing, and content integrity is checkable by anyone.
- **Sensitive payloads** are encrypted with a randomized AEAD under a per-contribution key, and the
  row commits to the **ciphertext with its nonce and tag**. Destroying the key leaves a commitment
  to a value that cannot be recomputed from a guess, because the ciphertext depends on randomness
  the guesser does not have.

After erasure what survives is proof of who recorded it, when, of what kind, under what causality,
privacy and retention — never a means of testing a hypothesis about its content.

Two distinct checks replace today's single one:

- **Chain integrity** — every row's `hash` follows from `prev_hash` and `commitment`. Always
  checkable, erased or not.
- **Content integrity** — the surviving payload matches its stored commitment. Checkable only where
  that payload survives; **reported as skipped, never as passed**, where it does not.

These are separate results, not one verdict. A payload that disagrees with its commitment is a
content failure at a known sequence while the chain stays intact; folding it into a chain failure
would say the biography's structure is damaged when one record's contents are, and after erasure
would make every legitimately forgotten row indistinguishable from a corrupted one.

`VerificationResult` gains that distinction. A verification that silently counted erased rows as
verified would be the same defect as a replay that treats a failed page as the end of history.

Existing rows keep `hash_version` 1 and 2 and remain verifiable as they are. Erasure is available
only for rows written at v3, because a v1 or v2 row's hash covers its payload by value and cannot be
recomputed without it. Nothing is rewritten in place: a hash chain that can be migrated
retroactively is not a hash chain.

### An erasure is itself a contribution, and it is a state machine

Erasure spans two systems that cannot be committed together: the SQLite transaction that redacts a
payload, and the key store that destroys a DEK. A single "do both" step therefore has two crash
windows, and each one produces a lie. Destroying the key first and crashing leaves data
irrecoverably gone with no record of why. Committing the redaction first and crashing leaves Mind
claiming it forgot something whose ciphertext is still decryptable.

So erasure follows the discipline lifecycle already uses — **durable intent before irreversible side
effect** — as three steps:

```
1. ErasureRequested          durable Event1 contribution; names target and typed reason
2. destroy DEK + wrappings   idempotent, repeatable after a crash
3. transaction:              redact payload, set erased_at,
                             bump erasure_epoch, append ErasureApplied
```

Recovery is then a question the Journal can answer by itself: an `ErasureRequested` with no matching
`ErasureApplied` is resumed from step 2, and step 2 is idempotent precisely so that resumption is
always safe. No state in that sequence claims more than has happened.

**The reason is a closed set, never free text**: `UserRequested`, `RetentionExpired`,
`ConsentWithdrawn`, `PolicyChange`, `SourceRevoked`. An erasure record is permanent, so a free-text
reason would let the thing being forgotten be restated in the one place that can never be erased —
"remove the record of diagnosis X" defeats the erasure it requests. A typed reason says why without
saying what.

An erasure record is never itself erasable. A forgetting that could be forgotten would make the
audit trail a suggestion.

### Submitting a contribution never authorizes an erasure

`Event1.Submit` must **not** accept an `ErasureRequested` kind from an arbitrary caller. Erasure is
a destructive storage operation, not a cognitive proposal, and the two travel different paths:
`Event1.RequestErasure` exists so that destroying biography is never reachable by the same call that
records a thought about it.

This is the invariant the rest of the substrate already runs on — *a proposal is not permission to
execute* — applied one step earlier than M9's authorization boundary. Stated here because the
alternative is discovering later that a critical security policy became an implementation detail of
`Submit()`, which is the kind of thing that is only ever found by someone exploiting it.

### Erasure propagates through durable retention dependencies

Invalidating caches is not enough, and treating it as sufficient was the largest hole in the earlier
draft. Consider:

```
Observation A   diagnosis = X
      ↓ evidence
Learning B      "because X, expect Y"
      ↓ evidence
Conclusion C    derived judgement about X
```

Erasing A's payload leaves B and C intact — and B and C are not caches to be rebuilt, they are
biography. Mind would have destroyed the record it was asked to forget and kept the reasoning that
restates it.

So a contribution carries **retention dependencies**, ordinarily derived from its causation and
evidence references, and an erasure applies to the *dependency closure* of its target rather than to
one row. `RetentionExpired` is already covered by inheriting the earliest expiry among references;
this covers the reasons that arrive before any expiry — `UserRequested`, `ConsentWithdrawn`,
`SourceRevoked` — which are exactly the ones a person cares about.

The closure is over retention dependencies, not over the whole causal graph. A contribution that
merely happened afterwards is not a descendant of what was erased.

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

Erasure by nulling reaches the live database and nothing else. For sensitive payloads that is not
enough, so **sensitive payloads are encrypted per contribution** and erasure destroys the key as
well as the ciphertext. A backup then retains ciphertext whose key no longer exists anywhere. This
is the only mechanism here that reaches copies Mind does not control.

**Keys are wrapped, not loose, and the wrapping is what makes backups recoverable.**

An earlier draft said only that the key store must never be backed up. That is a coherent position,
but it has a consequence the draft never stated: after a disk failure, restoring the Journal would
recover every sensitive ciphertext and no key at all. The entire sensitive biography would be lost
without anyone ever requesting an erasure — data loss by design, arrived at silently.

So the decision is a key hierarchy rather than a flat store:

```
per-contribution data key (DEK)   — one per sensitive payload, destroyed on erasure
        ↓ optionally wrapped by
key domain (KEK)                  — identified by an opaque keyDomainId and keyEpoch
        ↓ wrapped by
recovery root                     — held by the user, backed up deliberately and separately
```

**The intermediate layer is identified opaquely, never by what it protects.** Naming a key domain
after its subject would put `medical`, `sexuality`, `politics` or `location` into key metadata that
survives erasure — leaking the category of the thing being forgotten, which for many subjects is
most of what there was to hide. `keyDomainId` is a UUID and `keyEpoch` an integer; neither says
anything about the payload.

Erasure destroys the DEK and every wrapping of it. Because a DEK is per contribution, destroying it
reaches exactly one payload; because the wrappings are stored beside the ciphertext, a restored
backup is decryptable **only** for records whose DEK survived.

The recovery root is the one secret the user is responsible for, and it is deliberately not stored
with the Journal or its backups: a backup that contains both the ciphertext and the means to unwrap
everything is a plaintext backup with extra steps.

The remaining honest limit: a backup taken *before* an erasure, together with a recovery root that
still unwraps the DEK captured in that backup, defeats the erasure for that record. Erasure destroys
keys going forward; it cannot reach into a snapshot of the past that already contains them. Backup
rotation is therefore part of the retention guarantee rather than an operational detail, and a
deployment that keeps backups indefinitely has weakened erasure to the age of its oldest backup.

That is stated here so it is a decision, not a surprise — and it is why erasure reports a state
rather than a boolean.

### Erasure reports what it actually achieved

"Erased" is too binary to be honest. Destroying a key and redacting a payload reaches the live
database and every future backup; it does not reach a backup already taken. So an erasure carries a
typed completion state:

```
ErasureStatus {
    target
    liveState:        Complete        payload redacted, DEK destroyed
    projectionsState: Complete        epoch bumped, derived state rebuilt
    backupState:      PendingRotation older backups may still hold a wrapped DEK
}
```

A person asking whether something was forgotten must not be told "yes, completely" while a backup
containing a recoverable copy is still in rotation. This is the substrate's own invariant applied to
its most consequential operation: **partial is not the same as complete, and the reassuring reading
is the one that must be justified.**

### Retention is its own axis, propagated like privacy but not merged into it

A contribution carries a `RetentionClass` alongside its `PrivacyClass`. They answer different
questions — privacy asks *who may see this*, retention asks *how long may this exist at all* — and
the answers do not correlate: an identity fact may be highly private and needed for years, while
public telemetry may be worthless after ten minutes. An earlier draft attached lifetime to
`PrivacyClass`, which would have forced those two into one ordering and made every future
classification argument a fight about the wrong axis.

Both propagate, by the same discipline and through the same code path:

- derived privacy is the **most restrictive** class among a contribution's references;
- derived expiry is the **earliest** among them.

One mechanism, two axes. That is what keeps them checkable without pretending they are the same
thing.

**A contribution stores an absolute `retainUntil`, not only a class.** A class alone is a pointer
into a policy that will change: if `Short` means seven days today and twenty-four hours next year,
every record written under the old meaning silently acquires the new one, and a contribution's
retention would stop being a fact about that contribution. So a row carries `retentionClass`,
`retentionPolicyVersion` and the resolved `retainUntil` instant, and the instant is what governs.

Derived contributions resolve it the same way as privacy:

```
effectiveRetainUntil = min(own retainUntil, every referenced retainUntil)
```

A later policy change applies to what is written after it, which is the only direction a policy can
honestly reach.

## Consequences

- The Journal gains a column, a hash version, an epoch, and an erasure record kind. All are
  additive; no existing row changes.
- Verification answers a third thing — how many rows could not be content-checked — and every caller
  of `VerifyIncremental` has to decide what that means to it. That is the intended cost.
- Sensitive payloads are opaque to the Journal's own indices, and now also to anyone verifying
  content integrity without the key. Anything that needs
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

### Amendment: associative projections are derived state

Associative indices, embeddings, graph edges and cached activation state obey the erasure epoch like
every other projection, and **no surviving association, embedding or index key may reveal erased
sensitive content**. An index that still answered "these two were related" about a redacted payload
would be a smaller oracle, not an absent one.

The distinction that decides which mechanism applies:

```
learned durable association   →  retention dependency closure (E7)
derived association or index  →  epoch invalidation and rebuild (E8)
```

If Mind genuinely learned that the person prefers lemon with honey, that is biography: a durable
typed contribution, with evidence, privacy and retention, erased transitively with what it rests on.
If `contextd` merely computed a similarity to make retrieval fast, that is a cache and is rebuilt.

## Acceptance gates

Implementation is complete when these pass. They are listed here rather than in a test plan because
several of them are the reason a paragraph above says what it says, and a gate kept beside its
decision is harder to quietly drop than one kept elsewhere.

| | Gate |
|---|---|
| **E1** | An untouched v3 row verifies both chain and content |
| **E2** | An erased v3 row verifies its chain and reports content as **skipped** — never as verified |
| **E3** | A guessed low-entropy plaintext cannot be tested against a surviving commitment |
| **E4** | A crash after `ErasureRequested` recovers and completes |
| **E5** | A crash after DEK destruction recovers and completes, with no silent-loss state |
| **E6** | A crash before the terminal redaction never leaves a false "erased" claim |
| **E7** | A durable descendant carrying derived sensitive content is erased with its ancestor |
| **E8** | A projection whose epoch is behind the Journal's is refused and rebuilt |
| **E9** | During a rebuild, readers see `known = false` rather than an empty success |
| **E10** | A restored backup decrypts sensitive data that was never erased |
| **E11** | A restored current or rotated backup cannot decrypt a record whose DEK was destroyed |
| **E12** | A pre-erasure backup still in rotation is reported as outside the erasure guarantee |
| **E13** | `checkpoint == replay` continues to hold across erasures |
| **E14** | Every current projection stays bounded as erasures accumulate |

E9 and E13 are the existing P7 invariants applied to this feature rather than new requirements, and
E13 is checked by the property test P7.9 introduced rather than by a case written for erasure.

## Related documents

- [ADR-0011: Single Writer Event Journal](ADR-0011-single-writer-event-journal.md)
- [ADR-0018: Privacy Classification and Replication](ADR-0018-privacy-classification-and-replication.md)
- [ADR-0025: Grounding, Epistemics, and Cognitive Governance](ADR-0025-grounding-epistemics-and-cognitive-governance.md)
- [ADR-0027: Local Epistemic Projection Owner](ADR-0027-local-epistemic-projection-owner.md)
- [Journal Scale Baseline and Budgets](../mind/SCALE_BUDGETS.md)
