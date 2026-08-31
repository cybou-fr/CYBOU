<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0027: Local Epistemic Projection Owner

## Status

Accepted

This is an implementation decision, narrower than [ADR-0025](ADR-0025-grounding-epistemics-and-cognitive-governance.md),
which deliberately left the owner open. It had to be accepted before P7.1 began, because every
question below is one that a perception adapter would otherwise answer by accident.

Being Accepted, this outranks [Current State](../CURRENT_STATE.md): where an implementation and this
document disagree, the implementation is wrong. In particular the retention constraint below is
binding, not advisory.

## Context

M7 adds a grounded observation to Mind for the first time. M7 entry requires that the owner, state
locations, freshness vocabulary, contradiction handling and budgets be frozen before implementation,
and its exit gate is that no dual ownership remains and no perception source is treated as
truth merely because it is available.

Three things have changed since that gate was set that this decision can now rest on rather than
guess at:

- **Provenance is enforceable.** Event1 binds a contribution's `originOrgan` to the calling
  process's executable and refuses organ impersonation. Before that, any statement about who
  observed something was unverifiable, which would have made an epistemic projection a record of
  claims rather than of observations.
- **The scale costs are measured**, in [Scale Budgets](../mind/SCALE_BUDGETS.md). Sustained
  ingestion is capped near 800 contributions per second by fsync; full replay costs ~8.9 µs per
  contribution and each organ pays it separately on start.
- **Capabilities have one declaration.** A new capability is declared once and both healthd and
  Presence derive from it, so adding epistemic capabilities does not mean editing four tables.

## Decision

### A separate owner: `cybou-epistemicd`

The epistemic projection gets its own process, the tenth. It owns:

- the derived epistemic projection over accepted Observations;
- freshness state and the transition of an observation to stale;
- contradiction detection and reconciliation state;
- the epistemic status vocabulary below.

It explicitly does **not** own the Journal, any perception source, or system-wide retention policy.

The alternative — folding this into selfd or healthd — was rejected. selfd derives a projection of
Mind's own state and healthd projects component health; an epistemic projection is a claim about the
*world*, and merging them would make one process the authority on two different kinds of assertion.
Adding a tenth process boundary is a real cost, and it is accepted here because the boundary is
semantic rather than administrative.

### Producer identity and observation source are different fields

`originOrgan` answers **who brought this into Mind**. It is bound to the calling process and must not
be reused to mean where the knowledge came from.

`sourceId` answers **what was observed**. A perception adapter carries both: it is the producer, and
it names its source separately.

Conflating them is the failure this prevents. If `originOrgan` were the source, then replacing an
adapter would silently rewrite the provenance of everything it had ever reported, and two adapters
reading the same source would look like two independent sources — which is exactly the conditions
under which a contradiction check would agree with itself.

### `ObservationV1`

An Observation payload carries, at minimum:

| Field | Meaning |
|---|---|
| `sourceId` | What was observed. Stable across adapter replacement. |
| `subject` | The key the observation is about, so successive observations are comparable. |
| `value` | Typed. Not a string that a later reader has to guess the shape of. |
| `acquiredAt` | When the source was read, which is not when the contribution was accepted. |
| `freshnessUntil` | When the observation stops being current *by declaration*, not by inference. |
| `provenance` | How the value was obtained, sufficient to re-derive or challenge it. |

**Amended.** This table originally listed `privacy` as an `ObservationV1` field. It is not one, and
should not be: privacy travels on the surrounding `CognitiveEnvelope`, where Event1 already enforces
inheritance by refusing a contribution whose class is weaker than its references. Duplicating it in
the payload would create two copies free to disagree, and nothing would say which one governs. The
implementation was right and this document was wrong; the ADR is corrected rather than the code.

`acquiredAt` and the envelope's `wallTime` are deliberately distinct. Acceptance time is a fact
about Mind; acquisition time is a fact about the world, and a slow adapter must not be able to make
a stale reading look recent.

### A failed acquisition is ephemeral; only a change in it is durable

An adapter that cannot read its source reports a typed failure rather than an observation — a
valueless observation is structurally invalid precisely so that a failure cannot be smuggled in as a
fact about the world. What remained undecided was whether that failure is ephemeral state or durable
evidence, and an adapter written before the answer would have settled it by accident.

**Decided: ephemeral, except at a transition.** The adapter's current ability to read its source is
health state, observable through Health1 like any other component's. Only a *change* — readable to
unreadable, or back — is written to Event1.

Two reasons, and they point the same way:

- **Volume.** At one acquisition per source per 10 s, a source unavailable for a day would write
  more than eight thousand contributions recording that nothing happened, against a measured
  ingestion ceiling near 800 per second shared by everything else. Repeating an unchanged failure is
  not evidence; it is noise that makes the biography more expensive to replay for no gain.
- **What is actually worth remembering.** "The source became unreadable at T" is a fact about the
  world's availability. "It was still unreadable one poll later" is not a second fact. A projection
  can derive "unreadable since T" from the transition, which is the same reasoning by which healthd
  already records only significant transitions rather than every observation of unchanged health.

**A transition record is not an `ObservationV1` and must not claim to be one.** It states something
about the adapter's ability to observe, not about the subject it observes. It therefore carries its
own payload type under the discriminator introduced with `ObservationV1`, and the epistemic
projection reads it to distinguish `unknown` — never looked — from an observation that has aged out
while the source was unreachable. Conflating the two would let a failure to look present itself as
something looked at, which is the failure this ADR's separation of producer and source exists to
prevent.

The exact schema belongs to the adapter package. What is frozen here is that the failure is not an
observation, that unchanged failure is not rewritten, and that the transition is.

### Epistemic status vocabulary

`observed`, `stale`, `disputed`, `superseded`, `unknown`.

`unknown` is the default for anything never observed, and is distinct from `stale`. The distinction
is load-bearing for the UI: never having looked is not the same as having looked and the answer
having aged, and presenting either as the other is the failure ADR-0025 calls perception being
treated as truth.

### The projection is reconstructible

The projection is derived from accepted Observations and may be rebuilt from them. It is a cache,
never a second biography. Losing it costs a replay. If it ever disagrees with the Journal, the
Journal wins.

This is the same rule the verification checkpoint follows, and for the same reason.

### Budgets

Measured, not guessed:

- **Acquisition**: an adapter must not exceed one observation per source per 10 s in the first
  slice. At ~1.2 ms per accepted contribution, sustained ingestion is capped near 800/s overall, and
  a perception source has no claim on a meaningful share of that.
- **Projection**: reconstruction must stay within the Presence command budget of 5 s. Given ~8.9 µs
  per contribution for replay, an epistemic projection that replays the whole biography exhausts
  that near 560k contributions — so `epistemicd` must consume Event1 through the paged `Replay`
  cursor and persist its own position, not replay from zero.
- **Presence**: the epistemic section joins the existing single monotonic budget. It adds no second
  deadline.

### Frontend remains read-only for epistemic authority

The UI shows `unknown`, `stale` and `disputed` distinctly, and cannot resolve a contradiction.

## Consequences

- A tenth process, with the activation, health-graph entry and capability declaration that implies.
- The capability registry gains epistemic entries; healthd and Presence derive from them
  automatically, and the generated fault matrix extends to cover the new owner without new tests.
- `epistemicd` becomes the first consumer of paged `Replay` with a persisted cursor. That is the
  first real user of the P7.0-replay work, and it is where a projection checkpoint will first be
  justified by measurement rather than anticipated.
- Retention is **not** decided here. ADR-0025 requires that forgetting actually stop storing
  sensitive content while keeping auditable semantics, and the Journal is an append-only hash chain.
  Until a separate storage ADR covers expiry, tombstones, derived-data propagation, backups and
  possibly per-record keys, **no sensitive observation may be ingested**. The first adapter is
  chosen to make that constraint costless.

  **Satisfied.** [ADR-0028](ADR-0028-retention-and-erasure.md) is Accepted and implemented — payload
  erasure with a split commitment, a crash-safe protocol, propagation to durable descendants, key
  destruction, and retention recorded on the envelope. This constraint no longer blocks. It is left
  in place rather than deleted because it explains why the first adapter was chosen the way it was,
  and a constraint that vanishes once met leaves the record looking as though it never applied.

### Amendment: this owner does not own semantic association

`contextd` may reference `SubjectKnowledge` and contribution ids, and **must preserve epistemic
status** when it does:

```
Disputed → retrieved as Disputed
Stale    → retrieved as Stale
Unknown  → never becomes Observed by being retrieved
```

Retrieval is not evidence. Without this rule, `related ≈ true` becomes an architectural fact: a
claim would gain standing by being associated with something relevant, which is precisely the
"perception is not truth" error moved one layer up.

## First source

Current NixOS system generation and build identity. Local, non-sensitive, cheaply verifiable, and
naturally contradictory — the generation changes while an earlier observation still claims to be
current, which exercises staleness and supersession without any privacy question to answer first.

Explicitly not first: anything from sensors, network, browsing, or user files.

## Related documents

- [ADR-0025: Grounding, Epistemics, and Cognitive Governance](ADR-0025-grounding-epistemics-and-cognitive-governance.md)
- [ADR-0018: Privacy Classification and Replication](ADR-0018-privacy-classification-and-replication.md)
- [ADR-0011: Single Writer Event Journal](ADR-0011-single-writer-event-journal.md)
- [Journal Scale Baseline and Budgets](../mind/SCALE_BUDGETS.md)
- [Epistemic Governance](../mind/EPISTEMIC_GOVERNANCE.md)
