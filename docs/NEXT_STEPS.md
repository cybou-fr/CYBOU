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

- observations are produced but nothing consumes them: no epistemic projection owner exists, so
  freshness, contradiction and supersession are recorded and unread;
- retention and erasure remain undecided, and [ADR-0027](adr/ADR-0027-local-epistemic-projection-owner.md)
  forbids ingesting any sensitive observation until a storage ADR covers expiry, tombstones,
  derived-data propagation and backups;
- organs rebuild derived state on demand rather than maintaining it: `Intentions::open()`
  reconstructs on every call and predictord recomputes calibrations per query, both linear in the
  biography;
- cold reconstruction still costs a full replay per organ, which the measured budgets put at roughly
  nine seconds each at a million contributions;
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
any contribution claiming one of the nine organ identities unless the caller is that organ.

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

A process test that is not a Mind organ is refused all nine reserved identities, the Journal count is
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

- **Incremental projections.** `open()` reconstructs on every call and predictord recomputes per
  query. Both want an organ to hold derived state updated on `Accepted` rather than rebuilding on
  demand — one piece of work, not two.
- ~~**ADR-0027 lists `privacy` on `ObservationV1`.**~~ Amended: privacy travels on the envelope,
  where Event1 already enforces inheritance, and the ADR no longer lists it as a payload field.
- **The scale fixture is all root Observations.** A realistic mix with causation and evidence links
  will cost more, because evidence is fetched per contribution.
- ~~**Typed acquisition failure durability undecided.**~~ Decided in
  [ADR-0027](adr/ADR-0027-local-epistemic-projection-owner.md): ephemeral health state, except that
  a change between readable and unreadable is durable. Repeating an unchanged failure would write
  thousands of contributions recording that nothing happened; the transition is the fact. A
  transition record carries its own payload type and is never an `ObservationV1`.

## P7.1 — One typed local perception envelope

**Status: ObservationV1 frozen; the adapter is next.**

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

### Remaining for P7.1

A process-level test that the adapter's contribution survives the real D-Bus path with its
provenance intact. The behaviour is proven against a real Journal in-process, and the origin binding
is proven separately by the impostor test, but the two have not been proven together.

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
