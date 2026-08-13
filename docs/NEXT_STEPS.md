<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Next Engineering Steps

## Purpose

This document is the executable plan. The packages that produced the completed M5, M6 and P6.7
boundaries have moved to [Historical Execution](history/M5-M6.md), because a plan whose first thirty
pages are finished work stops being readable as a plan; the current work now comes first. [Roadmap](ROADMAP.md) remains the milestone definition; [Current State](CURRENT_STATE.md)
remains the implementation authority. The current M7 entry sequence and risk priorities are
recorded in the [2026-08-10 Project Checkpoint](PROJECT_CHECKPOINT_2026-08-10.md).

The immediate objective is still not language or autonomous action. It is:

> add one provenance-bearing local perception path with explicit freshness, epistemic status,
> retention behavior, bounded projection, and fault evidence.

M5 continuity, M6 capability honesty, and P6.7 bounded compound IPC are demonstrated. The current
architectural bottleneck is grounded knowledge governance: source provenance, freshness,
contradiction, retention, and erasure must become testable before replication or language.

## Current architecture assessment

### Strengths to preserve

- one canonical Event1/Journal acceptance boundary;
- explicit process and state owners;
- durable-before-visible ordering;
- lifecycle run persistence, high-water marks, deterministic operation keys, and split-commit recovery;
- Presence and Plasma as replaceable projections rather than cognitive owners;
- layered unit, process-integration, focused KVM, and full Plasma gates.

### Immediate gaps

Every entry that stood here described the substrate before M6 — no capability dependency owner,
untyped RPC outcomes, unbounded compound reads — and all of it has since been built. A gap list that
lists solved problems is worse than none: it makes the document unreadable as a statement of where
the work actually is. What follows is the current set.

- retention and erasure are decided but not built: [ADR-0028](adr/ADR-0028-retention-and-erasure.md)
  is Proposed, and until it is Accepted and implemented
  [ADR-0027](adr/ADR-0027-local-epistemic-projection-owner.md) forbids ingesting any sensitive
  observation;
- cold reconstruction still costs a full replay per organ, which the measured budgets put at roughly
  nine seconds each at a million contributions. Only epistemicd persists a checkpoint across
  restarts; predictord and intentiond rebuild theirs on start;
- the KVM gates run only locally, so the fault and recovery evidence — the substrate's most
  distinctive asset — is not exercised by any hosted check.

### Architectural direction

M6 introduces a dedicated capability/health owner, not a second cognitive monolith. It observes
typed service health and dependency policy, exposes an immutable projection, and records only
significant transitions through Event1. Existing organs retain their state and domain ownership;
Presence remains read-only; lifecycle mode remains orthogonal to capability health.

## Sequencing rules

1. A package starts only when its prerequisite gate is green.
2. Protocol/schema changes land before dependent UI behavior.
3. Every persistent change includes migration, interruption, and recovery tests.
4. `CURRENT_STATE.md` advances only with demonstrated implementation.
5. M8 language and M9 action work do not begin before the M6 exit gate.

## P6.8 — Close the substrate audit findings

**Status: complete.** The [Implementation Audit — 2026-08-10](CODE_AUDIT_2026-08-10.md) found
four places where the shipped implementation does not support a stated invariant, plus two hygiene
items. This package closes them before M7 raises event volume and adds a second projection, because
each finding becomes more expensive to fix once a perception adapter depends on it.

This is substrate repair on existing owners. It introduces no new process, no new persistent state
location, and no new wire contract.

### Work

- **A1 durability.** Raise the Journal to a commit mode that survives power loss, or restate the
  "durable before visible" invariant as durability to the operating system. Do not leave the
  invariant stated more strongly than the storage configuration supports.
- **A3 health honesty.** Derive presenced `Health()` from real aggregation outcomes and required
  downstream reachability, so `presence-presentation` can enter a deficit. Keep "the process is
  running" separate from "the process can present".
- **A4 bounded reads.** Replace the per-row `ConsumerBacklog` scan with one aggregate counting
  query. Do not cap `Recent` or bound `Verify`: `recent(0)` is how intentiond, predictord, and selfd
  replay their entire state, and selfd calls `Verify` on the ordinary self-assessment path, so a cap
  would silently truncate organ reconstruction and a partial verification would be reported as an
  integrity result. Those two need a cursor-carrying replay API and incremental verification against
  a persisted checkpoint, which is a contract change and is deferred below.
- **A5 failpoints — withdrawn, no change.** The hooks stay in the shipped binaries. `qFatal` grants
  no capability a same-user process lacks, since that process can already signal any daemon, and the
  reboot gate sets `CYBOU_LIFECYCLE_FAILPOINT` against the installed package — gating them out would
  move the split-commit evidence onto a binary that is not the shipped one. Recorded as an accepted
  property in the threat model.
- **A6 unit hardening.** Add justified systemd hardening to the Mind units, limited to directives a
  user manager can actually enforce — seccomp, rlimits, and no-new-privileges. Namespace-based
  options are omitted because an unprivileged user manager cannot apply them reliably, and
  `ProtectHome` would hide the Journal. Record the result as reducing the blast radius of a
  compromised Mind process, not as progress on the same-user D-Bus authorization boundary, which it
  does not address.

### Deferred within this package

**A2 — Presence projection migrated; healthd and the mutations are not.** `Snapshot` is now a
delayed-reply method that gathers every capability-gated owner concurrently through
`AsyncRpcClient`, so presenced serves other callers while aggregating and pays the slowest owner
rather than the sum. The projection clients use a single attempt and no circuit latching:
a latched circuit outlives the request that opened it and would render a transient stall as a
permanently empty section for five seconds.

**A2 is now closed.** The mutations, `Activity` and `DetailedObligations` are asynchronous
continuations with delayed replies, so no call on the Presence surface blocks. Mutations remain
ordered — gate, Event1 preflight, activity, durable Observation, domain mutation — because their
steps are causally dependent; asynchronous is not the same property as concurrent. Safety comes from
the operation semantics, not the retry policy: a non-idempotent step is never retried, and its
timeout surfaces as `unknown-outcome`.

A claim made when this package was opened has been withdrawn. healthd does **not** probe with the
synchronous client — `Refresh` already issues every probe concurrently through `AsyncRpcClient`
under a bounded deadline. No healthd work was needed.

**P6.8 is complete.** Do not open a P6.9 for further polish: the substrate findings are closed or
explicitly deferred with reasons, and the remaining risks are M7's to carry.

## P7.0-trust — Bind Event1 origin to the calling process

**Status: implemented.** `originOrgan` is provenance, and until now the caller simply asserted it.
eventd resolves the calling connection to its executable, caches that per connection, and refuses
any contribution claiming one of the reserved organ identities unless the caller is that organ.

The binding is to the executable, not to D-Bus name ownership. identityd records its session to
Event1 from its constructor, before `ServiceHost` publishes its name, so requiring ownership would
reject that write and break identity continuity at startup — a worse failure than the forgery it
prevents. A same-user process cannot fake `/proc/<pid>/exe` without actually being that binary, and
the answer does not depend on startup ordering.

This is scoped deliberately. It closes impersonation, not authorship: a caller that is not an organ
may still contribute under a name of its own, which is how tools and the test suite write. Method-
level authorization and capability tokens remain outstanding and are recorded as such in the threat
model.

Sequenced before the scale fixtures on purpose: it changes what `Submit` accepts, so building
fixtures first would mean rebuilding them.

### Exit gate

A process test that is not a Mind organ is refused every reserved identity, the Journal count is
unchanged by the attempt, and a contribution under a non-reserved name still succeeds.

## P7.0-scale — Journal fixtures and budgets

**Status: implemented for the Journal paths.** `journal-scale` builds a deterministic fixture and
measures append, full replay, `Verify`, backlog counting, indexed lookup and size. It defaults to
10k so it runs in the ordinary checks; 100k and 1m are the same code with
`CYBOU_SCALE_CONTRIBUTIONS` set. Results and the budgets derived from them are in
[Journal Scale Baseline and Budgets](mind/SCALE_BUDGETS.md).

Every growth-sensitive path is linear across two orders of magnitude and indexed lookup is flat, so
there is no hidden quadratic. The two thresholds that matter both land near half a million
contributions: `Verify` consumes the entire 5 s Presence command budget at ~460k, which makes
`Reflect` impossible, and organ cold reconstruction costs ~9 s per organ at 1m with three organs
each replaying the whole history. Those are the numbers the paged-replay and incremental-verification
packages exist to move.

Not yet measured, and the honest gaps: per-organ cold reconstruction end to end rather than just the
Journal read under it, RSS, Presence and consolidation behaviour under a large journal, concurrent
read/write pressure, and the growth rate of a real biography — without which the thresholds cannot be
turned into a date.

**A4 remainder — scalable biography replay and verification.** Every organ rebuild pulls the full
biography across D-Bus and every self-assessment rechains it. Closing this needs a replay API that
carries a cursor and an incremental `Verify` against a persisted checkpoint, so a partial result is
reported as partial. Both are wire-contract changes and belong with the M7 scale fixtures that give
them a budget to be measured against.

**A7 — fault-tier CI coverage.** None of the four KVM gates runs on a push. This is a maintainer
policy decision — KVM-capable runner, scheduled run, or a documented pre-tag manual gate with a named
owner — and not a code change.

### Exit gate

The fast Nix check set and both package builds pass from a clean tree, the Mind CTest suites pass
including new coverage for the bounded read paths and the derived presenced health answer, and the
four KVM gates pass on a KVM-capable host with no change to their continuity and recovery assertions.

Next, begin the first M7 vertical slice: one provenance-bearing local system perception adapter,
typed freshness/retention policy, an accepted Event1 observation, and a read-only Presence
projection. Keep contradiction handling explicit and do not add autonomous action authority.

## P7.0-replay — Paged Event1 replay

**Status: API implemented, one organ migrated.** `Event1.Replay(afterSequence, limit)` answers a
page as `{ok, from, to, head, hasMore, envelopes}`, capped server-side at 1000 so it cannot become
`Recent(0)` wearing a cursor. `EventStore` gained `after()` and a `replayAll()` helper; `Journal` and
`EventClient` both implement it. `Recent` is unchanged and stays what it should be — genuinely recent
activity for the UI, not a replay protocol.

The page reports `ok` separately from being empty. Without that, a replay whose transport died
halfway looks exactly like one that finished, and an organ rebuilds its state from a prefix while
believing it has the whole history.

**intentiond is migrated.** Two chronological passes, both paged, replacing one `recent(0)` that
was walked three times. Two hazards had to be handled, and both are the reason the remaining organs
are not done in the same change:

- **Order is inverted.** `recent(0)` yields newest first; `replayAll` yields oldest first. The old
  code ended with a `std::reverse` to correct for that, and keeping it would have inverted the order
  Presence shows commitments in.
- **Partial replay must fail closed.** If the first pass dies halfway, commitments whose closing
  Outcome was not read look open. Returning a partial open set is worse than returning none.

**Not yet migrated: predictord, selfd, workspaced.** Each has its own order dependence and must be
read before it is changed:

- `Predictor::samples` appends in newest-first order and downstream treats position as recency;
  reversing it silently changes which samples a prediction is built from.
- `Predictor::calibration` and `allCalibrations` sum and set-collect, so they are order-independent
  and are the safe ones.
- `SelfModel::subjects` already iterates `recent(0)` in reverse to get oldest-first, so it becomes
  simpler, not harder.
- `Workspace` uses `recent(m_capacity)` — bounded already, and not a replay at all. It may not need
  migrating.

**What this does not fix.** Paged replay is not faster: 728 ms against 762 ms at 100k. It bounds
memory and removes the single enormous D-Bus reply, both real failure modes, but the
cold-reconstruction budget is unmoved. See [Scale Budgets](mind/SCALE_BUDGETS.md).

## P7.0-verify — Incremental verification

**Status: mechanism implemented and tested; not yet wired to selfd.** `Journal::verifyFrom(anchor)`
checks the anchor still describes the journal, then walks only the contributions after it.
`checkpointAtHead()` produces the anchor to persist after a successful check. The result is typed —
`FullyVerified`, `VerifiedThrough`, `InvalidAt`, `CheckpointMismatch` — so a partial check can never
be read as a whole-history guarantee.

`verify()` keeps its existing contract and is now expressed through the same chain walk, so there is
one implementation to be right about instead of two that can drift.

At 100k, checking 500 new contributions costs 4 ms against 1,060 ms for the full walk: the cost
follows the increment, not the history.

Three semantics worth keeping in view:

- **A checkpoint is an accelerator, never an authority.** The Journal remains the only source of
  truth about its own integrity. Losing a checkpoint costs a full verification, not correctness.
- **`CheckpointMismatch` is not corruption.** It says the checkpoint is unusable, not that the
  journal is bad; reporting it as damage would send someone looking for a fault that is not there.
- **The prefix is trusted, so corruption inside it is invisible.** A test tampers with an early
  contribution and confirms the incremental check still reports intact while the full walk catches
  it. That is why the result is typed, and why a periodic full verification stays as the heavy gate.

### Wiring — complete

`Event1.VerifyIncremental()` answers the typed result. eventd owns the checkpoint as
`verification-checkpoint.json` beside the journal, the same pattern as consumer offsets, so it stays
visibly derived: losing it costs one full verification and nothing else. A checkpoint that no longer
describes the journal is discarded and the full walk runs, because a stale accelerator says nothing
about the biography. The checkpoint only advances on a chain that held — advancing past a break
would make the next verification skip the very contribution that is wrong.

selfd now calls it on the self-assessment path and carries `verification` and `verifiedFrom` into
its report alongside `journalIntact`. The distinction is preserved rather than flattened: a
full rechain and a checkpoint-anchored check both leave `journalIntact` true, and only the status
says which evidence was actually gathered.

**The 460k cliff is closed.** `Reflect` no longer scales with the length of the biography.

## P7.0-registry — One capability and command declaration

**Status: implemented.** `CapabilityRegistry` in `cybou-protocol` is the single declaration of which
capabilities exist, which components each rests on, and which capabilities each Presence command
requires.

The same knowledge had been written down four times: the dependency graph in `HealthPolicy`, the
component list beside it, the command-to-capability map in the Presence projection, and again in the
capability gate of every Presence mutation. They agreed by hand. The fourth copy was added during
P6.8, which is the argument for doing this before M7 rather than after — perception, epistemic
projection and retention each add capabilities to all of them at once.

It is a policy declaration, not a state owner. It says nothing about whether anything is currently
healthy; healthd remains the sole owner of that, and reading the registry makes no other process a
second authority on capability health.

`interruptLifecycle` is deliberately absent from the command table: it is gated on lifecycled being
reachable rather than on a capability, because lifecycle mode is orthogonal to capability health and
a run must stay interruptible while other capabilities are degraded.

### Generated fault expectations

`everyComponentLossProducesTheDeclaredDeficits` walks every component, marks it unavailable, and
compares the resulting deficits against what the registry declares — in both directions, so a
capability that should *not* have degraded is caught too. This addresses the checkpoint's risk that
health tests sample representative failures rather than the whole graph, and adding a capability
extends the matrix without anyone editing the test.

`registryIsInternallyConsistent` rejects a capability resting on an unknown component, a command
requiring a capability that does not exist, and a capability that does not say what its loss costs.
Verified by deliberately breaking the registry: the check named the fault precisely, and healthd's
own integration tests failed alongside it, which confirms the registry drives production rather than
sitting beside it.

Still hand-written: the process-level matrix, where components are really stopped rather than
modelled. The policy-level matrix above covers the graph; the process level covers the wiring, and
they are not substitutes.

## P7.0-ADR — Freeze the epistemic owner and ObservationV1

**Status: complete. [ADR-0027](adr/ADR-0027-local-epistemic-projection-owner.md) is Accepted.**
Every question it settles is one a perception adapter would otherwise have answered by accident.

Being Accepted, it outranks Current State: where an implementation and that ADR disagree, the
implementation is wrong. **P7.1 is unblocked.**

What it proposes: a separate `cybou-epistemicd` owning the derived projection, freshness,
contradiction and reconciliation — and owning neither the Journal, nor any perception source, nor
system-wide retention. Folding it into selfd or healthd was rejected because those project Mind's own
state and component health, while an epistemic projection is a claim about the world; merging them
would make one process the authority on two different kinds of assertion.

It separates `originOrgan` (who brought this into Mind, bound to the calling process since
P7.0-trust) from `sourceId` (what was observed). Conflating them would mean replacing an adapter
silently rewrote the provenance of everything it ever reported, and two adapters on one source would
look independent — the exact condition under which a contradiction check agrees with itself.

Its budgets come from [Scale Budgets](mind/SCALE_BUDGETS.md) rather than intuition, and one of them
has a direct consequence: an epistemic projection replaying the whole biography would exhaust the
5 s Presence budget near 560k contributions, so `epistemicd` must consume Event1 through the paged
`Replay` cursor with a persisted position. It is therefore the first real consumer of P7.0-replay,
and the first place a projection checkpoint will be justified by measurement rather than anticipated.

**Retention is deliberately not decided there**, and until a separate storage ADR covers expiry,
tombstones, derived-data propagation, backups and possibly per-record keys, no sensitive observation
may be ingested. The first source — NixOS system generation and build identity — is chosen so that
constraint costs nothing: local, non-sensitive, and naturally contradictory, since the generation
changes while an earlier observation still claims to be current.

## P7.2 — Reconstructible epistemic projection

**Status: the projection is implemented as a library; the owning process is not.** The reasoning is
separable from the plumbing that will carry it, and landing it first means the distinctions can be
argued about on their own.

`EpistemicProjection` derives, and decides nothing else. It holds no Journal, no transport and no
clock: status is evaluated against an instant the caller supplies, so the same admitted history
always yields the same answer for the same moment. A projection whose output depended on when it was
asked could not be tested, and could not be compared against itself after a rebuild.

The vocabulary is carried honestly rather than collapsed:

- **`unknown`** — never observed. Answered for any subject rather than failing for an unfamiliar
  one, because not knowing is a normal state of a mind, not an error.
- **`stale`** — observed, horizon lapsed, *value kept*. Discarding it would lose evidence that was
  actually gathered, and "was this, last checked then" is a more useful answer than silence.
- **`superseded`** — attached to history, never to what is currently claimed. A source restating an
  unchanged value is re-affirmation, not replacement; filing each restatement as a supersession
  would make a still world look busy.
- **`disputed`** — two sources currently claiming different things, left unresolved. Picking a
  winner by recency or by source would be inventing knowledge the projection does not have. A lapsed
  claim differing from a fresh one is not a dispute; it is the past, and the past does not argue
  with the present.

Ordering is by acquisition, not arrival: replay and restart deliver contributions in Journal order,
and an older reading arriving late must not unseat a newer one.

Anything that is not an `ObservationV1` passes through untouched — including the acquisition-state
records the adapter itself writes. A source that went unreadable leaves what it last said unchanged,
because no new evidence is not counter-evidence.

### The projection checkpoint

`snapshot()` and `restore()` make the derived state persistable, which is what ADR-0027 requires of
`epistemicd`: it must not replay from zero on every start, because at ~8.9 µs per contribution a
full replay exhausts the Presence budget near 560k.

**A checkpoint must be indistinguishable from the replay it stands in for**, or it is not a cache of
the Journal but a second biography, with nothing to say which is right. The test compares a restored
projection against a replayed one at two instants — one where a reading is still fresh and one where
it has aged out — because status is derived rather than stored, and a checkpoint that froze a status
would agree at the first and disagree at the second. Both the dispute and the supersession survive
the round trip, which is what keeps it from being a comparison of two empty projections.

Status is deliberately not serialised. It is an answer about a moment, and persisting it would hand
back something that was only ever true once as though it were still current.

A bad checkpoint is refused whole rather than partly applied. Rebuilding from the Journal is always
available and always correct, so a projection half-built from a corrupt cache buys nothing and risks
being quietly wrong; every refusal leaves what was already known untouched.

### Remaining for P7.2

`cybou-epistemicd` itself: the process, the paged `Replay` cursor persisted alongside the checkpoint,
the `Epistemic1` interface, and the capability declaration. The cursor and the checkpoint must be
written together — a checkpoint ahead of its cursor would re-admit contributions, which is harmless
because admission is idempotent, while a cursor ahead of its checkpoint would leave a gap, which is
not.

## P7.1 blockers — contract defects found in review

An external review found seven defects, five of them in work landed during this sequence. Each was
verified against the code before being recorded. Two corrected claims this repository was making.

**This section was lost once and is reconstructed.** The commit that claimed to record it anchored a
scripted edit on a heading that did not yet exist; the edit silently did nothing and the commit
message described content that was never written. Every later "closed" edit anchored on the same
missing text and also did nothing. The code changes below are real and committed — only the record
was absent. The lesson is narrow and worth keeping: a scripted documentation edit that cannot fail
loudly will eventually claim work it did not do.

### B1 — Origin binding defeated by a basename (P0) — CLOSED

An organ identity was granted on the executable's basename, so a user ELF named `cybou-predictord`
in `/tmp` could attribute contributions to the prediction organ. The caller's executable must now
also sit in eventd's own directory, derived from `/proc/self/exe` rather than configured — a
configured path would be settable by anyone able to restart the service. Proven by running the
genuine predictord binary from elsewhere, and by deleting the check and watching that test fail.

### B2 — A contradiction could not be recorded (P0) — CLOSED

Acquisition identity excluded the value, justified by a comment claiming disagreement would surface
as a contradiction. It could not: both values mapped to one `messageId` and Event1 rejected the
second as a duplicate, so the contradicting evidence never arrived. The value now participates.

### B3 — Identity key had a separator collision (P0) — CLOSED

Fields were joined with a byte nothing forbade inside them, so `("a", "b<sep>c")` and
`("a<sep>b", "c")` collided. The original collision test checked a pair that never collided. Identity
is now a canonical CBOR array, and the test checks the genuinely ambiguous pair.

### B4 — Freshness had no lower bound (P0) — CLOSED

`isFreshAt` checked only the upper bound, so a reading taken at 09:00 reported fresh at 04:00.

### B5 — `Observation` meant several things (P0) — CLOSED

`ContributionKind::Observation` already carried unrelated payloads from predictord and presenced.
Every `ObservationV1` now carries `@type: cybou.observation.v1`, checked before anything else is
read. Landed before any adapter writes, because afterwards the ambiguity is in durable history.

### B6 — `Reflect` scaled with biography times subjects (P0/P1) — CLOSED

`SelfModel::measure` replayed to find subjects, then replayed again per subject through
`Predictor::calibration`. `allCalibrations` now accumulates every subject in one pass and selfd
consumes that projection. `allCalibrations` had no test coverage at all, which is how it kept the
defect. The multiplication is gone; the remaining single replay is not.

### B7 — `Intentions::open()` replayed twice (P1) — CLOSED

Justified by a claim that a single chronological pass would meet some Outcomes before their
Intentions. Causation makes that impossible, and the sentence stating so sat directly beneath the
justification it contradicts. Now one pass.

### Still open

- ~~**Incremental projections.**~~ Done in P7.3 and P7.6: predictord and intentiond both carry
  cursors, and a read costs what arrived since the last one. Measured at 10k: 94 ms then 0 ms for
  calibrations, 86 ms then 0 ms for open commitments.
- ~~**ADR-0027 lists `privacy` on `ObservationV1`.**~~ Amended: privacy travels on the envelope,
  where Event1 already enforces inheritance, and the ADR no longer lists it as a payload field.
- ~~**The scale fixture is all root Observations.**~~ Done in P7.6. A connected fixture costs 2.3x
  more to build and only 10–15% more to read: causality is paid on the way in.
- ~~**Typed acquisition failure durability undecided.**~~ Decided in
  [ADR-0027](adr/ADR-0027-local-epistemic-projection-owner.md): ephemeral health state, except that
  a change between readable and unreadable is durable. Repeating an unchanged failure would write
  thousands of contributions recording that nothing happened; the transition is the fact. A
  transition record carries its own payload type and is never an `ObservationV1`.

## P7.1 — One typed local perception envelope

**Status: complete. `ObservationV1` is frozen and `cybou-perceptiond` is the adapter that uses it.**

`ObservationV1` carries `sourceId`, `subject`, a typed `value`, `acquiredAt`, `freshnessUntil` and
`provenance`, exactly as [ADR-0027](adr/ADR-0027-local-epistemic-projection-owner.md) requires. The
schema landed before the adapter deliberately: an adapter written first would have defined the
envelope by accident, in whatever shape its own source happened to need.

Three properties are load-bearing and tested:

- **Unknown schemas fail closed**, older and newer alike. There is one schema so far, so any other
  number means the payload was written by something this build cannot interpret, and evidence read
  under guessed rules is worse than no evidence.
- **A valueless observation is invalid.** A failure to observe is not an observation of nothing, so
  an adapter that cannot read its source must report a typed failure rather than contribute an empty
  value. This is what makes the checkpoint's "source unavailability has a typed result" enforceable
  rather than merely intended.
- **Acquisition identity is deterministic** over source, subject and acquisition time, so
  re-reporting one reading after a restart or retry resolves to the same contribution and Event1's
  existing duplicate rejection makes it a durable no-op. The value is excluded on purpose: two
  different values for one subject at one instant is a contradiction for the projection to surface,
  not two contributions to record. Timezone does not affect identity, or every observation would
  duplicate across a DST change.

`freshnessUntil` is declared by the adapter, which knows how fast its source changes, rather than
inferred by whoever reads the observation later. `acquiredAt` stays distinct from the envelope's
acceptance time throughout.

### The adapter — implemented

`cybou-perceptiond` is the tenth process. It reads the identity of the running system, proposes it
as an `ObservationV1` through Event1 under its own reserved identity, and does nothing else: it owns
no state, mutates no configuration, and does not decide whether what it reported is still true.

`perceptiond` was added to the reserved identities *before* it existed. An identity that only becomes
protected once something claims it leaves a window in which anything may claim it first, and
provenance is the whole point of this organ.

**Reading often and contributing every time are different things**, and the first version conflated
them. Acquisition identity includes the instant, so an unchanged system polled every ten seconds
produced one contribution per poll — over eight thousand restatements of one fact a day, exactly the
noise the transition rule forbids for failures. The test written alongside caught it.

Contributing only on change is the opposite error: within its declared horizon the previous
observation speaks for the present, but once that lapses nothing does, and a projection would have to
call the fact stale forever while the adapter sat watching it be true. So an unchanged value is
re-affirmed at most once per freshness horizon — thirty times fewer contributions than per-poll, and
a fact that stays true keeps saying so.

`local-perception` is declared in the capability registry, so healthd graphs it and the generated
fault matrix covers the new owner without a new test.

### Proven across the process boundary

`perceptiond-integration` runs the real adapter against real eventd over a real bus. Two things had
been proven separately and never together — that a reading becomes an `ObservationV1`, and that
Event1 binds a claimed origin to the calling executable — and a contribution carrying correct
provenance only when nothing checks it would be worth very little.

It also settled a question the in-process tests could not ask. **A restart records once and then
falls silent.** The test first asserted it should record nothing, reasoning that identity comes from
the reading rather than from who read it; that is wrong, because identity includes the acquisition
instant and a fresh instance has no memory of what it last contributed.

The behaviour is right as it stands. Re-affirmation exists so a fact that stays true keeps saying
so, and a session boundary is exactly when a fresh reading is worth having: "the system was still
this at startup" is evidence, where "it was still this one poll later" is not. The rate is bounded
by how often the process starts, and a crash loop is bounded further by systemd's restart limit.

**P7.1 is complete.**

## P7.2 — the epistemic owner

`cybou-epistemicd` is the eleventh process, and the owner ADR-0027 named before it existed. The
projection had been a library since P7.1: it could already say `unknown`, `observed`, `stale`,
`disputed` and `superseded`, and could already be checkpointed and restored. What was missing was
someone whose job it is to hold it.

**What it owns is narrower than what it knows.** It owns the derived projection — freshness,
contradiction, reconciliation — and owns neither the Journal, nor any perception source, nor
system-wide retention. It never writes to Event1. That asymmetry is the whole point: a component
that both decides what is true and writes what is true has no one to check it.

### The first real consumer of the cursor

Paged `Replay(afterSequence, limit)` and the persisted verification checkpoint were built in P7.0
against benchmarks. This is the first thing that needs them for its own reasons. epistemicd catches
up from its persisted cursor on start and pages through in units of a thousand, so a long biography
never sits in memory at once and never crosses the bus in one reply.

A page that fails to read is not the end of history. Treating it as the end would leave a gap that
nothing downstream could ever discover, so the cursor keeps whatever it reached and the failure is
reported instead.

The same reasoning covers live announcements. One whose sequence is behind the cursor is dropped —
re-admitting is harmless because admission is idempotent, but moving the cursor backwards would
claim history had not been read when it had. One whose sequence leaves a gap triggers a read of that
gap rather than a skip over it.

### The checkpoint is a cache, and is tested as one

The cursor and the projection are written as **one** value. Half of that pair is worse than neither:
a checkpoint ahead of its cursor merely re-admits contributions, but a cursor ahead of its
checkpoint leaves a hole in what was admitted and nothing would ever notice.

Three tests fix what the checkpoint may be. A restart resumes with the same answer *and* the
supersession intact — which is what makes it a resumption rather than a fresh projection that
happens to look similar. Deleting it costs a replay and nothing else, asserted by comparing the cold
answer to the warm one byte for byte. Corrupting it discards the whole thing rather than trusting
part, because a damaged cache quietly becoming what Mind believes is precisely the failure the
Journal exists to prevent.

Status is never serialised, only derived, which is why a restored projection answers correctly at an
instant it was never asked about before.

### What the eleventh owner cost elsewhere

Adding `epistemic-projection` to the capability registry changed what *healthy* means, and four
process tests that assert a fully-healthy Mind began failing — they were still starting nine or ten
owners. That is the registry working as intended: one declaration, and every consumer of it moves
together. Both process suites now start all eleven.

**A test written alongside caught a defect in the test, not the code.** An early version dated the
later of two acquisitions into the future and then reported `stale`. The projection was right: an
observation says nothing about a time before it was acquired, so a future-dated reading does not
speak for the present.

The projection answers over D-Bus. No Presence view reads it yet — surfacing epistemic status to a
person is P7.4, and it is deliberately after retention.

### A gate that was measuring the wrong thing

`p4-plasma-lifecycle` failed once in three runs after the eleventh owner arrived: `count 1 -> 3`.
Its final assertion is that restarting plasmashell contributes nothing to the Journal, taken as a
Journal count before and after.

That is only a claim about the restart if the Journal was quiet when the baseline was read, and it
was not. healthd records a capability transition the first time it observes each owner, and an owner
still starting is observed later — so a baseline read early enough missed two transitions, which
then landed during the restart window and were attributed to the restart. Two extra contributions,
two newly registered owners.

The bug was latent long before this change; adding an owner only widened the window enough to hit
it. The fix is to establish the precondition the assertion depends on rather than to loosen the
assertion: wait for a completed health refresh, then for two consecutive counts to agree, and only
then take the baseline. A restart that genuinely contributed would still fail, because the
quiescence only runs beforehand. Four consecutive runs green, then a full `nix flake check`.

**Adding to the registry is how this was found.** One declaration means a new owner perturbs every
consumer at once, and a test that was passing for a reason it did not state stops passing.

### Proven across the process boundary

The projection's reasoning was covered in process, and the owner's cursor and checkpoint were
covered against a Journal in the same process. Neither said anything about the parts only a real
boundary shows, so `epistemicd-integration` runs eventd, perceptiond and epistemicd as three
processes over a real bus.

It proves four things that were previously assumed. A live acceptance announced by Event1 reaches
the projection with no restart and no polling — the two organs are actually connected, and the
subject and source survive two boundaries without collapsing into the carrying organ's name. The
checkpoint under the real state root survives a restart, with perceptiond stopped so a non-zero
cursor immediately after start can only have come from disk. What was accepted while the owner was
down is taken in on the next start, so an organ that only learned from live announcements would be
caught. And **the projection never contributes**: reading everything it can answer leaves the
Journal count unchanged, which is the ADR-0027 boundary tested rather than asserted in prose.

The suite passed in 1.4 seconds, which is fast enough for five process tests to be worth doubting.
Sabotaging the live-path expectation failed exactly that test and nothing else.

**P7.2 is complete.**

## P7.3 — a read costs what changed, not what happened

`Reflect` was the last budget line that grew with the length of a life. `Intentions::open()` had
already moved to paged `replayAll`; the remaining full scans were all in `Predictor`, three of them,
one per query — `history`, `calibration`, `allCalibrations`.

An earlier pass had already removed a multiplication there: `allCalibrations` used to replay the
biography to find the subjects and then replay it again for each one, so the cost was the length of
a life times the number of subjects. That fixed the multiplication and left the length.

Predictor now keeps the same shape epistemicd introduced: a cursor, a paged catch-up, and derived
state that is never authoritative over the Journal. Per subject it holds the samples a forecast is
built from and running error accumulators, so a calibration is arithmetic on numbers rather than a
traversal. Catch-up **fails closed** — a projection built from part of the history is not a smaller
answer but a wrong one, because an unread Outcome makes a subject look better calibrated than it is
and nothing downstream could tell.

### Measured, not asserted

At ten thousand contributions, the first read costs 94 ms and the second costs 0 ms. The suite
reports both, and asserts only the absolute claim — a read that answers from a cursor does no work
proportional to history, so it cannot take a meaningful number of milliseconds on any machine.
Extrapolated at the measured ~9.4 µs per contribution, the old per-query cost at 560k was on the
order of five seconds, against a five second `Reflect` budget.

### The defect this makes possible

Every existing predictor test passed immediately, and would have passed just as well against a cache
that never advanced its cursor. That is the failure this change introduces, so two tests were added
where a **second** writer appends between two reads of the same instance — a forecast whose sample
count must grow, and a calibration that must see an Outcome settled elsewhere. A stale cursor fails
both.

`Predictor::m_bySubject` is memory that grows with the number of distinct subjects and their
samples, which is the cost of not re-reading. It is the same data `history()` used to materialise on
every call and then throw away; what is new is that it is retained. Only epistemicd persists a
checkpoint, because only epistemicd has been measured to need one.

### The gate's other timeout

Quiescing the baseline uncovered a second, unrelated flake in the same gate: plasmashell failing to
become active again within thirty seconds of a restart, roughly one run in four, while every count
assertion passed.

Thirty seconds was a timeout measuring the host. What the gate claims is that Plasma comes back and
that Mind neither lost its run nor recorded anything for the restart — not how quickly a
software-rendered compositor restarts inside a VM. The four post-restart waits are now 120 seconds
and every assertion is unchanged: the unit must still become active, the MainPID must still differ,
Plasma must still re-expose `evaluateScript`, and the applet must still be there. Four consecutive
runs green.

### The same defect in m6, and three sabotages that lied

`m6-recovery-boundary` had the defect `p4` had. Its
`unresponsive Event1 rejects Promise and preserves count` subtest asserted an unchanged Journal count
across a window in which eventd is deliberately SIGSTOPped. healthd observes the frozen owner and
cannot record that transition until eventd resumes, so the write lands inside the window and is
charged to the Promise — about one failure in four, while the rejection itself always behaved.

The count was a stand-in for the real claim: a rejected Promise leaves no commitment behind.
intentiond's open set says that precisely and is unmoved by what other owners record about the
outage, so the gate now compares it across the window, guarded by an assertion that the set is
non-empty to begin with.

**Getting there took three sabotages that all reported success for three different wrong reasons**,
and on that evidence the change was briefly reverted as unprovable:

- The first was a Nix syntax error — `''` inside a `''`-string — so Nix never built the VM. It
  produced no output, and no output through a `grep` filter reads exactly like a passing run.
- The second called `Intention1.Form` with an empty `causeId`. `Intentions::form` requires a cause
  that already exists in the Journal, so it formed nothing; the sabotage was a no-op.
- The third let the Promise run with eventd unfrozen but read its result through a flipped `grep`,
  and its exit code was over-read as "the assertion did not fire".

What settled it was measuring instead of inferring: a probe that printed the values showed a
successful Promise takes the open set from **70 bytes to 604**, reproduced at 600 on a second run.
The assertion then failed exactly as it should when the Promise was allowed to succeed — and failed
at that assertion specifically, identifiable because it is the only one there without a message.
Four consecutive runs green afterwards.

Also worth recording, because it invalidated several verification runs in this session:
`nix build --rebuild` **errors** on a derivation it has never built rather than running it. Repeat
runs used to characterise a flake must use a plain `nix build` first, and must be judged on the exit
code — not on the absence of a `grep` match.

**P7.3 is complete.**

## P7.5 — Presence shows what Mind knows

Everything in P7 up to here produced knowledge or derived it. This is the first place a person could
see it: the Snapshot carries a `knowledge` section, gated on `epistemic-projection` like every other
section and gathered concurrently with them, and the QML proxy exposes it as a bindable property.

**Wiring it up found a defect that nothing else could have.** epistemicd answered `Knowledge` and
`KnowledgeOf` in **bare CBOR**, while every other organ answers in the versioned fabric envelope.
Presence could not have decoded it. Nothing caught it in P7.2 because nothing consumed it — a
projection nobody reads can be encoded any way at all and still look correct. It now speaks the
shared wire, and both test suites decode through `FabricCodec` so a future drift off it fails rather
than passing quietly. This was the cheapest moment the wire could have been corrected: no consumer
existed to break.

**The end-to-end test found a second gap.** It waits for real content rather than asserting the key
exists, and it failed — because the process suite had no perception source at all. `/run/current-system`
does not exist in a build sandbox, so perceptiond reported the source unavailable and observed
nothing. Eleven organs were running and one of them could never do its job. The suite now stages a
system-generation fixture, and the test walks the whole chain: perceptiond observes, Event1 accepts,
epistemicd derives, presenced projects, across five processes.

It checks the vocabulary a reader would act on rather than the shape of the reply — which subject,
what status, and on whose authority. The source stays `nixos.system` rather than collapsing into the
organ that carried it, two process boundaries from the adapter.

A projection that cannot be read leaves an empty list, which reads exactly as knowing nothing. That
is the honest projection of an owner that could not be asked, and the proxy test pins it, including
through the meta-object — QML binds that way, and a `Q_PROPERTY` that was declared but never
registered would still pass a direct call.

**P7.5 is complete.**

## P7.6 — the last per-query replay, and a fixture with a shape

`Intentions::open()` was the last derived read that paid for the whole biography every time it was
asked, and Presence asks on every Snapshot.

It now carries a cursor, like epistemicd and predictord. **The reason it is safe is the same
guarantee the old two-pass version misread**: an Outcome names the Intention it closes and is always
accepted after it, so state accumulated up to a sequence can never be invalidated by a later page.
That property was already written down; the earlier code contradicted it in the comment directly
above the guarantee.

Measured at 10k: first read 86 ms, second read 0 ms.

Every existing test would have passed against a cache that never advanced its cursor, so two were
added where a second instance forms or closes a commitment between two reads of the first. Freezing
the cursor fails both — and also fails two process tests that already asserted an obligation formed
in one process is visible from another, which is worth knowing: the D-Bus boundary was covering this
by accident, and the library level is where the defect actually lives.

### The fixture had no shape

Every scale measurement to date used root observations: no causation, no evidence, nothing to look
up on the way in. Mind never writes that, so every budget derived from it described a Journal that
does not exist. A second fixture builds five-contribution episodes, so each append pays the
reference lookups and privacy inheritance Event1 actually performs.

Measured in the same run at 10k, which is what makes the comparison worth anything:

| Measure | Flat | Connected |
|---|---:|---:|
| Fixture build (batched) | 435 ms | **1,017 ms** |
| Full replay | 92 ms | 101 ms |
| Full `Verify` | 122 ms | 140 ms |

**Connection is paid on the way in.** Building costs 2.3x more, because each derived contribution
resolves its cause and every evidence id and then checks its privacy against all of them. Reading
costs 10–15% more.

Deliberately not reported as a finding: per-contribution append measured *faster* connected than
flat in the same run (0.935 ms against 1.790 ms). That contradicts the build number, and both are
fsync-bound where variance exceeds the gap. It is noise, and dressing it up as a result would be
inventing one.

Two mistakes of mine are recorded in the test itself. An `Observation` is a root kind and may carry
no references at all, and evidence may never repeat the `causationId` — the first fixture broke both
and was refused as invalid. And `verify()` answers *where the chain broke*, so zero means intact
rather than nothing verified; asserting it like a count made a healthy journal look wholly corrupt.

**P7.6 is complete.**

## P7.7 — three invariants, held on purpose

A second external review found a P0 I had introduced myself, and it was the same class of mistake
twice over.

**A dispute did not survive a checkpoint.** Self-contradiction was carried in two side tables added
when the rule was written, and `snapshot()` was never taught to write them. So a source that said
two different things about one instant read as `Disputed` until the next restart and `Observed`
afterwards. There were tests for the dispute and tests for the checkpoint, and none for the two
together; `evidenceSurvivesACheckpoint` gave false comfort by proving something else survived.

Fixed structurally rather than by serialising two more fields: a source's current state is one to
many **co-current claims**. The contradiction becomes a property of the data instead of an
annotation beside it, so it persists for free and both side tables are gone. That also fixed a
freshness bug found in the same review — two readings of one instant may declare different horizons,
and the lapsed one used to keep disputing forever. Each claim is aged by its own `freshUntil` now.

**Presence had acquired a new unbounded read.** `Knowledge()` returned every superseded claim
inline, and supersession grows for the life of the Journal, so P7.3's removal of the full-Journal
scan had been traded for an ever-growing reply — the same cost moved somewhere less visible, and
invisible entirely while there is only one source. The current projection now carries
`supersededCount`, and history is paged through `KnowledgeHistory(subject, after, limit)`, capped
like Event1's `Replay`.

**`Intentions::open()` was still O(all formed).** The cursor stopped it re-reading the Journal but
it kept every commitment ever made and filtered on each call — the same unbounded shape one level
up. Closing removes now, so a read is proportional to what Mind currently carries.

### The three invariants

These three defects are one family, and the review named it better than the fixes do:

- **checkpoint == replay** — a cache may never answer differently from the history it stands in for;
- **partial or unavailable != empty truth** — a read that failed must not look like a fact;
- **the current projection stays bounded** — what is true now cannot cost what has ever been true.

Every P7 organ now holds derived state, so these are the acceptance conditions for anything added
next, not observations about what happened to be fixed.

### Also here

ADR-0028 gained the disaster-recovery decision it was missing. Saying only that the key store must
never be backed up has a consequence the draft never stated: a disk failure would lose the entire
sensitive biography with no erasure ever requested. Keys are wrapped in a hierarchy now — per
contribution, per retention class, under a recovery root the user holds and backs up separately —
so a restored backup decrypts exactly the records whose keys survived. The remaining limit is stated
rather than glossed: a backup predating an erasure, with a root that still unwraps it, defeats that
erasure, so backup rotation is part of the retention guarantee.

The documentation validator now derives the process count *and* the suite count from the build
rather than matching remembered phrases. That is why the drift kept returning: a validator that
pins the wrong number turns fixing the docs into a build failure.

**P7.7 is complete.**

## P7.8 — a failed read stops looking like a fact

The three invariants were written down as acceptance conditions, and the second one — *partial or
unavailable is not empty truth* — turned out not to hold in three places I had written myself.

- `Intentions::open()` returned an empty list both when nothing was outstanding and when the
  Journal could not be read.
- `Predictor::calibration()` returned a zeroed Calibration for both "never settled" and "could not
  tell".
- `allCalibrations()` returned nothing for both "no subjects" and "no answer".

Each collapse chose the most reassuring reading. A Mind that cannot read its own history would have
reported no outstanding obligations, no prediction errors and a perfectly unbiased record — and
Presence would have shown that as a complete projection.

**The fix is a type, not a flag.** Both reads return `std::optional`, so the compiler enumerated
every place the distinction had been discarded rather than leaving it to a search. The organs then
send a D-Bus error instead of an empty success, which needs no new machinery: every caller already
treats a failed call as a section it could not measure and leaves a typed default.

It propagates the whole way. `IntentionClient::open` reports whether the call succeeded — a fabric
reply always carries its version envelope, so empty bytes are unambiguously a failure — and
`SelfReport` carries `obligationsKnown` and `calibrationsKnown` across the wire. A self-assessment
can no longer report "nothing outstanding" when it never found out.

Both new tests use a Journal whose paged reads can be made to fail, because the case only exists
when reading history does not work and a healthy Journal will not produce it.

### On the gates

`p4-plasma-lifecycle` failed again inside a full `nix flake check`, waiting for the Presence applet
to reappear in `appletsrc` after a plasmashell restart, and passed alone immediately afterwards.
`nix flake check` runs four VM gates against one machine; run sequentially, all four pass. The
timeout was **not** raised again — it was already raised once, and raising a limit that has failed
at its new value twice would be tuning the measurement rather than the thing measured.

**P7.8 is complete.**

## P7.4 — the retention decision, written down

ADR-0027 made one constraint binding: no sensitive observation may be ingested until a storage ADR
covers expiry, tombstones, derived-data propagation, backups and possibly per-record keys. That is
the only thing standing between the current tree and perception sources worth having, so
[ADR-0028](adr/ADR-0028-retention-and-erasure.md) is now **Proposed**. It is not Accepted:
ADR-0027 was accepted explicitly, and this one should be too.

The tension is structural, not incidental. `canonicalEnvelopeV2` includes `payloadCbor` **by
value**, so erasing a payload breaks the chain from that row onward; deleting the row is worse,
because `contribution_evidence` holds `ON DELETE RESTRICT` edges precisely so evidence cannot be
orphaned. And even a perfect fix to the live database reaches neither the projections that cached
the content nor the backups already taken.

What it decides:

- **The unit of erasure is the payload, never the record.** Identity, causality, position and
  evidence edges survive. What was thought, by whom, on the evidence of what, stays provable; what
  was *said* becomes unavailable. A Mind that could forget having concluded something could not
  explain why it changed its mind.
- **Row hash v3 chains an envelope digest rather than an envelope**, so the chain verifies without
  the payload. Verification splits into chain integrity, always checkable, and content integrity,
  checkable only where the payload survives — and reported as **skipped**, never as passed, where it
  does not. Counting an erased row as verified would be the same defect as treating a failed replay
  page as the end of history. v1 and v2 rows stay as they are and are not erasable, because a hash
  chain that can be migrated retroactively is not a hash chain.
- **An erasure is itself a contribution**, written in the same transaction that nulls the payload.
  There is no side channel that mutates the Journal without leaving a trace in it.
- **Derived state is invalidated by an erasure epoch**, bluntly. Working out which derived value
  depended on an erased payload needs exactly the payload that is gone, so precise invalidation is
  not available and rebuilding is. epistemicd and predictord already have the machinery.
- **Backups are addressed by key destruction**, because nulling reaches the live database and
  nothing else. The limits are stated rather than glossed: it is cryptographic erasure, the key
  store becomes as sensitive as the payloads, and a backup of the key store taken beforehand defeats
  it — so excluding it from backup is part of the decision, not an operational footnote.
- **Retention rides on `PrivacyClass`**, inheriting the shortest lifetime among evidence exactly as
  privacy inherits the most restrictive class. Two classification schemes over the same records
  would disagree, and the disagreement would be discovered by a leak.

The consequence worth accepting deliberately: encrypted payloads are opaque to the Journal's own
indices, so anything searchable must live in non-payload fields — which are never erased and must
therefore never be sensitive. That constrains adapter design, and is the reason to settle this
before writing the adapters rather than after.

## Scheduling and refresh flakiness — fixed

Two separate defects made the process suite intermittently red. Both are closed.

### A lost scheduling race was reported as failure

The policy is evaluated twice: `RunSchedulingCycle` takes a health snapshot and decides, then
`ExecuteSchedulingDecision` re-fetches and requires the snapshot identity to be unchanged. healthd
refreshes on a timer and on every bus owner change, so a refresh landing in that window supersedes
the evidence and the run is refused.

The refusal is correct — a decision reasoned about one snapshot must not execute against another.
Reporting it as `failed` was not: that is the same answer as a scheduler which could not do its job.
It now answers `deferred`, and the tests retry on exactly that condition.

Established as a genuine flake before anything changed: the same Nix derivation hash produced one
failing and two passing runs.

### healthd refused refreshes instead of coalescing them

`Refresh` returned false whenever one was already collecting. Under process churn — a suite starting
and stopping nine organs generates owner changes continuously, each refresh running to its deadline —
an explicit caller could be locked out for seconds together, and a bare false gave it no way to tell
"I am busy" from "I could not".

A caller arriving mid-refresh now gets a delayed reply. When the running collection finishes, one
further refresh is scheduled and every waiter is answered from it: one extra collection serves any
number of waiters, and each gets an answer from a run that began after they asked — which matters,
because they may have changed something the in-flight run had already passed.

**Three earlier attempts failed, each on a premise checked only after being built on.** That the
assertions were at fault — no, `QTRY_VERIFY(Refresh)` is a barrier whose wait is load-bearing, and
removing it made the suite worse. That automatic refresh could be disabled for the suite — no,
several tests exercise healthd re-observing by itself. That two concurrent D-Bus calls would overlap
— no, instrumentation showed they never did, because a refresh with few organs running finishes
first.

What unblocked it was building the missing instrument rather than another fix:
`CYBOU_HEALTH_REFRESH_HOLD_MS` holds a refresh open for a known duration, so overlap is constructed
instead of hoped for. With it the test failed for the right reason before the change and passed
after, and the process suite ran green four times consecutively.

The knob is deliberate test scaffolding in a shipped binary, on the same terms as the failpoints the
threat model already accepts: it can only make healthd slower, it grants no capability, and without
it this defect cannot be reproduced on demand or shown to be fixed.

## Deferred behind gates

### M7

Start with one provenance-bearing system perception adapter and one epistemic contradiction slice.
Do not begin multi-node transport until retention, privacy, freshness, and erasure semantics are
testable locally.

### M8

Attach a language faculty only to selected typed context carrying provenance, epistemic status,
freshness, privacy, and deficits. Model absence/replacement must be an acceptance test.

### M9

No privileged executor work begins until proposal, criticism, value constraints, authorization,
typed capability, observation, outcome, and rollback boundaries are independently represented.

## Definition of done for every package

- explicit owner and non-owner list;
- typed success, failure, interruption, and degraded outcomes;
- persistence and migration statement;
- privacy/provenance/retention impact;
- unit plus process/VM test proportional to the boundary;
- updated contracts and `CURRENT_STATE.md` when behavior is implemented;
- reproducible Nix gate from a clean tree.
