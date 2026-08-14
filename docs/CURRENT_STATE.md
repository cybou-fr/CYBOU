<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Current State

Status date: 2026-08-10.

This document is intentionally limited to implemented behavior and current limitations.

## Repository gate status

The P0 baseline is green: formatting, REUSE 3.3, package metadata, cognitive documentation, Mind
access, QML API, UI polish, `cybou-mind`, and `cybou-presence-applet` pass through pinned Nix checks.
The Mind package runs twenty-nine CTest suites, including Event1, lifecycle persistence/recovery,
Lifecycle1 process restart, and multi-process integration across the eleven Mind owners. Both counts
are checked against the build rather than trusted: the documentation validator derives them from the
package's daemon list and the tests CMakeLists, so a document that falls behind the code fails the
build instead of quietly misdescribing it. The process suite also proves a
simulated new login preserves identity and an accepted open intention while incrementing the
logical session count, and that compound Presence reads and mutations obey one bounded deadline.

The M5 lifecycle owner is present: lifecycle schema v1, legal mode transitions, atomic persistent
run state, `org.cybou.Mind.Lifecycle1`, D-Bus/systemd activation, D-Bus run requests, and restart
recovery of an active run into `Recovering`. Legacy v0 state is backed up and migrated to v1;
unknown future versions fail closed. The focused headless NixOS gate proves that a real reboot
preserves the exact persisted run and identity ID, enters `Recovering`, and increments the logical
session count.

The P3 transaction substrate now includes deterministic per-capability operation keys,
high-water-mark-bound idempotent acknowledgements, optional capability deficits, required-work
completion gates, and explicit resume of the same run after recovery. Lifecycle1 automatically
dispatches bounded `Consolidate` requests to Predictor1 and Workspace1 and validates their typed
receipts before persisting acknowledgements. Each owner resolves the exact accepted Event1 envelope
at the run high-water mark, commits an evidence-linked `Learning` contribution with a deterministic
UUIDv5 operation identity, and returns its contribution ID. Redelivery is a durable no-op, and the
integration suite proves two first-delivery contributions and zero duplicate contributions.
Lifecycle1 persists the capability-to-contribution mapping in the run, verifies every reference
against Event1, and refuses `Completed` until it has committed a deterministic terminal `Outcome`
caused by all owner results. The process integration suite verifies the extra terminal append and
exposes its ID through Lifecycle1 state. Process-level
fault injection now kills lifecycled immediately after an owner Event1 commit and immediately after
the terminal Event1 commit. In both cases restart enters `Recovering`, replay reuses deterministic
contributions, and Event1 count proves that no duplicate durable effect was created.
Lifecycle mutations roll back their in-memory candidate when persistence fails; unknown status
values fail protocol validation; optional deficit causes persist in the run; and the preferred
`RequestRunAtCurrentHead` API captures its accepted boundary directly from Event1.

Lifecycle1 emits `Changed` after each accepted state commit. Presenced subscribes through a typed
LifecycleClient and projects `lifecycleMode`, `lifecycleStatus`, the full lifecycle state,
lifecycled health, and a derived progress/freshness/deficit view through Presence1. The QML proxy
exposes these as read-only properties; Mind Header and Dashboard give lifecycle modes distinct
visual treatment while runtime availability remains a separate `awake` dimension. Projection age
does not claim that underlying evidence is epistemically fresh.

The focused headless NixOS gate now covers three boot cycles: baseline active-run/identity
continuity, reboot after an owner Event1 commit but before coordinator acknowledgement, and reboot
after terminal Event1 commit but before terminal run-state persistence. Both split-commit replays
reuse their deterministic contributions and leave Event1 count unchanged. This closes the P3
consolidation transaction exit gate.

The focused P4 Plasma VM gate restarts the shipped `plasma-plasmashell.service` around an active
run, observes a replacement PID and restored Plasma D-Bus surface, and proves that neither the
exact persisted lifecycle run nor Event1 count changes across UI recreation.

P6.1 introduced `CapabilitySnapshot`; the current schema v2 keeps component health, capability state, typed
deficit cause, recovery policy, observation/verification time, impact, and evidence/error reference
are encoded separately and validated fail closed. Focused tests cover round trip, unknown schema and
enum rejection, malformed/inconsistent state, dependency/uniqueness invariants, and component
transition legality. At the P6.1 boundary these types had no dependency graph, owner, or D-Bus service.

P6.2 adds `cybou-healthd` as the sole owner of the initial capability dependency graph and the
atomic persistent snapshot under `$XDG_STATE_HOME/cybou/health`. Health1 refreshes public organ
health boundaries, separates required core deficits from optional limitations, preserves the exact
last snapshot across owner restart, fails closed on corrupt state, and requires an explicit
`Recovering` snapshot before a formerly unavailable component becomes healthy. Process integration
proves loss of predictord leaves accepted biography, identity continuity, commitments, and bounded
workspace available; dependent optional capabilities become explicit deficits and recover without
replacing the health owner state.

P6.3 adds typed RPC outcomes and explicit read-only, idempotent-mutation, and non-idempotent-mutation
semantics. The shared async D-Bus client applies bounded deadlines, deterministic exponential
backoff, retry eligibility, and a closed/open/half-open circuit breaker. Plasma lifecycle
interruption is the first production consumer: its timeout is reported as `UnknownOutcome`, is not
retried, does not block the shell event loop, and does not move terminal ownership out of
lifecycled. Other legacy RPC paths remain synchronous until migrated explicitly.

P6.4 adds schema-v1 homeostatic measurements and an observation-only Health1 projection. Each
signal carries its source, kind, unit, observation and validity time, and explicit freshness or
availability status. Healthd collects capability-deficit count, per-owner probe latency, probe
failures, accepted Event1 count, and active lifecycle-run count. Event backlog, Journal size, and
prediction calibration pressure are `Unsupported`, not fabricated zeroes. Measurements are
ephemeral across healthd restart and schema v1 rejects scheduling authority; thresholds,
hysteresis, automatic lifecycle policy, and Presence projection remain P6.5 work.

The P6.4.1/M5-hardening bridge removes three pre-P6.5 ambiguity windows. Health1 now probes owners
in parallel through bounded read-only async RPC, maps timeout into typed deficit cause, rejects
overlapping collection, reacts to D-Bus owner changes with debounce, and performs slow periodic
verification. Predictor and Workspace consolidation derive their computed values only from Event1
sequences at or below the captured high-water mark; replay returns values from the accepted owner
contribution rather than live state. Lifecycle validation requires one non-empty cause for every
missing capability, and completed terminal Outcome payloads durably record completed capabilities,
missing capabilities, and their causes. CapabilitySnapshot schema v2 now emits one record per
unhealthy `(capability, dependency)` pair, so simultaneous owner loss remains fully explainable.
Persisted schema v1 is accepted as a strict compatible subset and normalized to v2; unknown future
versions fail closed.

P6.5 slice 1 connects presenced to Health1 and exports aggregate state, per-capability states,
typed deficits, and observation time through Presence1 and the QML proxy. `awake` remains a
compatibility alias for presentation endpoint reachability; it is no longer the authorization gate
for every command. Biography, commitments, prediction, self-assessment, and attention operations
are independently gated by their declared capabilities. This avoids a health probe cycle:
presenced readiness never depends on healthd readiness, while the capability projection may be
unknown until Health1 is available. Automatic lifecycle policy and complete degraded-mode visual
treatment remain P6.5 work.

P6.5 slice 2 adds an owner-correct scheduling dry run to Lifecycle1. The policy checks lifecycle
idleness, active-run exclusion, a 60-second capability freshness window,
accepted-biography availability, optional predictor/workspace
eligibility, measurement freshness, and a bounded Event1-backlog hysteresis (enter at 32, exit at
8). It returns `Run`, `Defer`, or `Block` with a stable policy ID and causal explanation, and is
projected through Presence1/QML as `lifecycleScheduling`. Evaluation cannot mutate lifecycle state.
At the slice-2 boundary homeostasis schema v1 forbade scheduling authority, so evaluation honestly
deferred instead of starting work.

P6.5 slice 3 makes Event1 the durable owner of consumer progress. Event1 stores versioned consumer
offsets separately from canonical Journal rows, rejects invalid/backward/ahead-of-head movement,
and derives exact backlog from Journal sequence plus the consumer offset. Lifecycled registers the
stable `lifecycle.consolidation` consumer and advances it to the accepted input high-water mark only
after terminal lifecycle state is durable; restart reconciles an already-completed run. Events in
the `lifecycle.consolidation` capability scope are excluded from that consumer's pressure, avoiding
a self-triggering output loop. Health1 now reports `event.backlog.count` as a current typed value
when the consumer is registered.

P6.5 slice 4 introduces Homeostasis schema v2 policy-scoped authorization. The codec accepts schema
v1 only when its legacy boolean is false and normalizes it to no authorized policies. Health1 adds
`event-backlog-v1` only when the durable consumer backlog is current. Lifecycle1 can consequently
return an authorized `Run` decision after its independent capability, freshness, idle-state,
worker, and 32/8 hysteresis checks. Evaluation remains read-only and process integration proves its
state bytes are unchanged.

P6.5 slice 5 adds the explicit idempotent `ExecuteSchedulingDecision` mutation to Lifecycle1.
Evaluation carries exact capability and homeostasis snapshot IDs; execution re-reads Health1 and
rejects either superseded ID before constructing a run. The run UUID is deterministic from the
policy and both evidence IDs, so retry after an unknown reply returns the same active or completed
run. After a newer run replaces the lifecycle projection, the deterministic terminal Event1 ID
still prevents recreation of the old run. Process integration proves stale-evidence rejection,
active/terminal retry, later-run retry, owner dispatch, completion, and consumer-offset advancement.

P6.5 slice 6 adds the bounded production scheduler trigger inside lifecycled. Health1 `Changed` is
debounced by 100 ms and a 30-second timer supplies slow verification. Each trigger returns without
mutation for `Block`/`Defer`; an authorized `Run` executes, dispatches, and completes through the
existing idempotent transaction. An active scheduled run takes precedence over new evaluation:
after restart it resumes from `Recovering` and continues the same run. Fault injection immediately
after durable scheduled-run creation proves restart recovery, owner dispatch, terminal completion,
and zero residual consumer backlog without a duplicate run.

P6.5 slice 7 adds durable user-activity arbitration. Presence commands call
`Lifecycle1.NotifyUserActivity`; lifecycle state schema v2 records `lastUserActivityAt` and
`schedulerCooldownUntil`. Activity wakes an idle lifecycle, defers automatic scheduling for the
configured window (60 seconds by default), and interrupts only active automatic backlog runs.
Cooldown survives restart, while manual maintenance runs retain their active state.

P6.5 slice 8 moves production scheduled owner dispatch onto the shared asynchronous resilient RPC
client. A cycle returns `started`, processes eligible owners sequentially, and completes through
callbacks. Callback acceptance is fenced by run identity and active lifecycle state. Process fault
injection holds predictord inside consolidation while Lifecycle1 accepts activity immediately;
the late contribution cannot turn the interrupted run into a completed run.

P6.5 slice 9 completes the capability explanation boundary in Presence1. Alongside raw deficit
diagnostics, `capabilityDetails` provides one grouped record per known capability with state,
availability, causes, impact, dependencies, verification time, recovery policy, and recovery
progress. Predictor-loss process coverage proves unrelated commands remain usable and the same
record moves from unavailable/waiting through recovering/verifying to available/ready.

P6.5 slice 10 adds the command-side presentation contract. `commandAvailability` and
`canCommand(id)` expose required/missing capabilities without weakening backend enforcement.
Process coverage proves useful commands survive optional predictor loss and explicitly observes
`Awake + Limited` and `Recovering + Limited`; lifecycle mode is not an alias for health. P6.5 is
therefore complete, and implementation focus moves to the P6.6 failure/recovery matrix.

P6.6 slice 1 expands optional-organ fault injection across predictord, selfd, and workspaced.
Process tests assert exact capability/command loss, zero Event1 mutation for rejected operations,
continued independent behavior, typed recovery progress, and restored command availability.

P6.6 slice 2 covers the lifecycle and presentation boundaries. Lifecycled loss disables only its
mapped consolidation/control surface and rejected control creates no Event1 effect. Presenced loss
marks an existing QML proxy unreachable instead of leaving a stale reachable flag; reconnecting
the same proxy preserves identity/session/Event1 state and every cognitive owner process.

P6.6 slice 3 proves the scheduled-owner timeout boundary with a real delayed predictord. Three
bounded idempotent attempts fail a required run closed without advancing consumer progress; late
replies converge on one deterministic contribution and cannot rewrite terminal state. Restoring
predictord allows a new evidence-bound run to consume the preserved backlog once.

P6.6 slice 4 crashes lifecycled at two transport boundaries: after a retryable timeout and after
the circuit opens. Persistent lifecycle state retains the same active run; restart resumes that run
from `Recovering`, reuses deterministic owner effects, commits one terminal outcome, and drains the
backlog without duplication.

P6.6 slice 5 covers required Event1 loss. Presence remains a responsive presentation boundary but
projects biography, identity continuity, and commitments as unavailable. Rejected Promise creates
no Journal effect; restart preserves exact count, identity/session continuity, and existing
commitments, excludes the rejected description, and restores commands after verification. The
process fault matrix is complete.

P6.6 slice 6 adds the focused KVM exit gate. A real Plasma session proves Presence D-Bus/systemd
activation without shell replacement, durable lifecycle state remains unchanged across a timed-out
interruption and changes only after recovery, and an unresponsive Event1 owner rejects Promise
without Journal growth. Resuming the same owner preserves its PID, count, and the plasmashell PID.
Together with slices 1–5, this completes P6.6 and satisfies the M6 exit gate.

P6.7 slice 1 hardens the compound Promise path discovered by that gate. EventClient waits on an
explicitly timed asynchronous pending call, and Promise probes required Event1 durability before
notifying auxiliary owners. The unresponsive-owner KVM case now rejects in under one second under
an 8-second client deadline instead of accumulating roughly 24 seconds of internal RPC budgets.
A shared remaining-budget context across different owners remains the next post-M6 hardening step.

P6.7 slice 2 implements that context for Promise. The command owns one five-second monotonic
deadline; Health1, Event1, Lifecycle1, and Intention1 receive only its remaining budget, and no
later call is sent after exhaustion. Existing client APIs retain their five-second default. KVM
coverage lowers the server budget to one second, uses a three-second external deadline, observes a
server-side rejection in about 0.22 seconds, and then proves Journal and Plasma continuity.
Reflect, prediction, and commitment commands are the remaining rollout surface.

P6.7 slice 3 completes that interactive-command rollout. A small shared deadline helper propagates
one monotonic remaining budget through Reflect, Observe, Predict, Fulfill, and Abandon. Every path
uses one Health1 snapshot; durable mutation paths preflight Event1, while read-only Predict remains
independent of Journal availability. Self1, Predictor1, and Intention1 clients now accept the same
bounded per-call budget without changing their five-second standalone default. Full process and KVM
coverage pass with continuity intact. `InterruptLifecycle` is the remaining compound Presence path
to align server-side with this model; its asynchronous shell caller already retains the explicit
non-idempotent unknown-outcome policy.

P6.7 slice 4 aligns `InterruptLifecycle` as well. Presence validates active Lifecycle1 state and
submits `FinishRun(interrupted)` under one server-side monotonic budget, refusing to send the
mutation after expiry. The Plasma caller remains asynchronous and non-idempotent: losing the reply
still yields `UnknownOutcome` and never triggers a retry. Process and KVM fault coverage preserve
the active run byte-for-byte after timeout and prove an explicit later interruption succeeds.
Read-only snapshot aggregation is now the remaining P6.7 sequential multi-owner latency surface.

P6.7 slice 5 closes that surface and completes P6.7. Presence `Snapshot`, `Activity`, and
`DetailedObligations` share one deadline per request. Snapshot returns the complete stable key
shape even when an owner exhausts the budget; fields not collected are typed empty/default values,
and no later owner RPC is sent. Its organ-health projection now reuses the canonical Health1
component records. A real process test suspends selfd while it remains registered, observes the
bounded partial snapshot, resumes it, and leaves subsequent recovery tests clean. All compound
Presence reads and mutations are now protected from sequential timeout multiplication.

P6.8 closes the substrate findings recorded in the [Implementation Audit](CODE_AUDIT_2026-08-10.md).
The Journal commits at `synchronous=FULL` and verifies both commit pragmas at open, refusing to
start rather than let a silent fallback leave durable-before-visible stated more strongly than
storage supports; an in-memory Journal remains exempt because it makes no durability claim.
presenced derives `Health()` from the outcome of its last projection, so Health1 can report a
`presence-presentation` deficit when capabilities are unreachable or the shared budget expires,
while readiness stays a separate dimension. The consolidation consumer backlog is answered by one
aggregate query instead of decoding every envelope after the offset. Mind user units carry seccomp,
rlimit, and no-new-privileges hardening; namespace-based directives are omitted because an
unprivileged user manager cannot enforce them and `ProtectHome` would hide the Journal.

Presence `Snapshot` is now a delayed-reply D-Bus method. Health1 remains the single sequential step
because its answer decides which reads are legitimate; every gated read then issues concurrently
through the async RPC client, and the reply is sent when the last one lands or the shared guard
expires. presenced no longer blocks its own event loop during a projection, and a process test
proves a second caller is answered while a gather is in flight. The projection clients use one
attempt and no circuit latching, so a transient owner stall cannot blank a section of the UI beyond
the request that observed it.

The compound mutations, `Activity` and `DetailedObligations` are asynchronous continuations with
delayed replies as well, so no call on the Presence surface blocks. Mutations stay ordered — gate,
Event1 preflight, activity notification, durable Observation, domain mutation — because those steps
are causally dependent; only the waiting is removed. A non-idempotent step is never retried and its
timeout surfaces as `unknown-outcome`, which is the contract the shell relies on for lifecycle
interruption. A process test suspends intentiond mid-command and observes a second caller answered
while the mutation is still in flight. healthd already probed concurrently through the async client
and was not altered.

Event1 binds a contribution's claimed `originOrgan` to the process that submitted it. eventd resolves
the calling connection to its executable, caches that per connection, and refuses any contribution
claiming one of the reserved organ identities unless the caller is that organ. The binding is to the
executable rather than to D-Bus name ownership because identityd records its session to Event1 from
its constructor, before it publishes its name; requiring ownership would reject that and break
identity continuity at startup. A process test proves every reserved identity is refused to a non-organ
caller, that nothing forged reaches the Journal, and that a caller contributing under its own name
still succeeds. This closes forged provenance; it is not a general authorization model, and what a
non-organ caller may contribute under its own name is unchanged.

A Journal scale baseline exists. `journal-scale` builds a deterministic fixture — 10,000
contributions by default, larger through `CYBOU_SCALE_CONTRIBUTIONS` — and measures append under
production durability, full replay, `Verify`, backlog counting, indexed lookup and size. `Journal`
gained `appendBatch`, which shares one transaction across many contributions so a large fixture can
be built without one fsync per row; it is deliberately not reachable from Event1, where acceptance
must remain per-contribution. Measurements and the thresholds derived from them are recorded in
[Journal Scale Baseline and Budgets](mind/SCALE_BUDGETS.md): every growth-sensitive path is linear,
`Verify` exhausts the Presence command budget near 460,000 contributions, and organ cold
reconstruction costs roughly nine seconds per organ at a million.

Journal verification is incremental. `Journal::verifyFrom(anchor)` confirms the anchor still
describes the journal and then walks only what follows it; eventd owns the checkpoint as
`verification-checkpoint.json` beside the journal, discards one that no longer matches, and advances
it only on a chain that held. `Event1.VerifyIncremental()` carries a typed result — `FullyVerified`,
`VerifiedThrough`, `InvalidAt`, `CheckpointMismatch` — and selfd reports it alongside
`journalIntact`, so a check that trusted a prefix is never presented as a whole-history guarantee.
At 100k, checking 500 new contributions costs 4 ms against 1,060 ms for the full walk. This removes
verification as a limit on `Reflect`.

`Reflect` is now independent of the biography as well. `SelfModel::measure` once built its subject
list with `recent(0)` and then replayed the whole history again inside `Predictor::calibration` for
every subject, which was roughly O(contributions x subjects); a single pass removed the
multiplication, and a cursor-carrying projection in `Predictor` removed the remaining factor. At
10k contributions the first read costs 94 ms and the second costs 0 ms, because a read now pays for
what arrived since the last one rather than for the length of a life.

A periodic full verification remains the heavy integrity gate, because corruption inside a trusted
prefix is by construction invisible to the incremental check.

Automatic scheduling distinguishes a lost race from a failure. Lifecycled evaluates its policy once
to decide and again to execute, refusing to start a run whose health evidence was replaced in
between; healthd refreshes on a timer and on bus owner changes, so that supersession is ordinary
rather than exceptional. It is now reported as `deferred` with a reason naming it, not as `failed`.

Capability and command policy has one declaration. `CapabilityRegistry` in `cybou-protocol` states
which capabilities exist, which components each rests on, and which capabilities each Presence
command requires; healthd derives its dependency graph from it and presenced derives both the
command projection and the capability gate of every command. It declares policy and owns no state,
so healthd remains the sole owner of capability health. A generated matrix marks each component
unavailable in turn and compares the resulting deficits against the declaration in both directions,
replacing the representative sample the checkpoint recorded as insufficient.

`ObservationV1` is frozen as the typed perception payload: source, subject, typed value, acquisition
time, declared freshness horizon and provenance. Unknown schema versions fail closed in both
directions, a valueless observation is structurally invalid so a failure to observe cannot be
contributed as one, and acquisition identity is deterministic over source, subject, acquisition time
**and value**, so a repeated identical report is a durable no-op while two different values for one
instant keep distinct identities and are both recorded. That last part is what lets a source be
caught contradicting itself; an identity that excluded the value would have silently kept whichever
arrived first.

`cybou-perceptiond` produces these in the running system, and `cybou-epistemicd` consumes them.

Health1 coalesces refreshes rather than refusing them. A caller arriving while a collection is
running receives a delayed reply; when that collection finishes one further refresh is scheduled and
every waiter is answered from it, so one extra collection serves any number of waiters and each gets
an answer derived from a run that began after it asked. The overlapping-collection guard is
unchanged — two probes of one owner at once remains forbidden — but a busy owner no longer returns a
bare false that a caller cannot distinguish from failure.

`cybou-perceptiond` is the tenth process and the first grounded perception adapter. It reads the
identity of the running NixOS system and proposes it as an `ObservationV1` under its own reserved
origin, which Event1 binds to its executable. It owns no state, mutates no configuration, and makes
no judgement about whether what it reported remains true. An unreadable source yields a typed
failure and no observation; only a change between readable and unreadable is durable. An unchanged
reading is re-affirmed at most once per declared freshness horizon rather than once per poll.
`local-perception` is declared in the capability registry, so healthd graphs it like any other owner.
A process test runs the real
adapter against real eventd over a real bus and confirms the contribution arrives with provenance
eventd verified rather than accepted; a restart re-reads and records once, then falls silent. The epistemic projection exists as a
library: it derives `unknown`, `observed`, `stale`, `disputed` and `superseded` from accepted
observations against a caller-supplied instant, keeps a stale value rather than discarding it,
treats an unchanged restatement as re-affirmation rather than replacement, refuses to resolve a
disagreement between sources, and orders by acquisition so a late-arriving older reading cannot
unseat a newer one. The projection is also persistable: a restored checkpoint answers exactly as a
replay would, at any instant, because status is derived rather than stored, and a corrupt or
unrecognised checkpoint is refused whole rather than partly applied.

`cybou-epistemicd` is the eleventh process and the owner ADR-0027 requires. It reads accepted
observations and answers what is known; it never writes to Event1, owns no perception source, and
owns no retention policy. It is the first consumer of the cursor-carrying paged `Replay`: it catches
up from its persisted cursor on start, and a live announcement whose sequence leaves a gap triggers a
read of that gap rather than a skip over it, so the projection stays a function of the whole
biography rather than of what happened to be delivered. The cursor and the projection are written as
one value, because a cursor ahead of its checkpoint would claim history had been admitted that had
not, and nothing downstream could ever discover it. Losing or corrupting the checkpoint costs a
replay and nothing else — it is a cache of the Journal, never a rival to it — which the tests assert
by comparing the cold answer against the warm one byte for byte. `epistemic-projection` is declared
in the capability registry, so healthd graphs it and the generated fault matrix covers it. The
projection is answered over D-Bus; no Presence view reads it yet.

Two audit items are deliberately not closed. `Recent` is not capped and `Verify` is not bounded per
call: `recent(0)` is how intentiond, predictord, and selfd replay their whole state, and selfd calls
`Verify` on the ordinary self-assessment path, so both need a cursor-carrying replay API and
incremental verification against a persisted checkpoint rather than a truncating limit. Unit
hardening reduces the blast radius of a compromised Mind process and is not progress on the
same-user D-Bus authorization boundary, which remains open.

The larger cognitive model and future agency architecture are described in `MIND_MODEL.md`.
M1–M6 form the implemented process-isolated, continuity-preserving and degraded-mode substrate of
that model. P6.7 is post-M6 latency hardening; the tree does not yet contain the planned M8 language faculty
or M9 authorized executor.

## Process topology

Mind now has eleven real user-session processes:

```text
cybou-eventd
cybou-healthd
cybou-lifecycled
cybou-identityd
cybou-intentiond
cybou-predictord
cybou-selfd
cybou-workspaced
cybou-presenced
cybou-perceptiond
cybou-epistemicd
```

`plasmashell` no longer constructs Identity, Intentions, Predictor, SelfModel, Workspace, Journal,
or EventClient. It loads a lightweight `Presence` QObject whose runtime job is Presence1 IPC and
QML property caching.

## Ownership

| Resource / responsibility | Owner |
|---|---|
| `journal.db` | `cybou-eventd` |
| capability dependency graph and current health snapshot | `cybou-healthd` |
| lifecycle mode and run state | `cybou-lifecycled` under `$XDG_STATE_HOME/cybou/lifecycle` |
| `identity.json` | `cybou-identityd` |
| identity login marker | `cybou-identityd` under `$XDG_RUNTIME_DIR/cybou` |
| intention commands/projection | `cybou-intentiond` |
| prediction/calibration | `cybou-predictord` |
| self projection/assessment | `cybou-selfd` |
| bounded attention | `cybou-workspaced` |
| presentation aggregation | `cybou-presenced` |
| visual cache | Plasma Presence proxy |

There is currently no language-model process and no privileged action-executor process in this
ownership table.

## IPC

Versioned Qt D-Bus interfaces:

```text
org.cybou.Mind.Event1
org.cybou.Mind.Health1
org.cybou.Mind.Lifecycle1
org.cybou.Mind.Identity1
org.cybou.Mind.Intention1
org.cybou.Mind.Predictor1
org.cybou.Mind.Self1
org.cybou.Mind.Workspace1
org.cybou.Mind.Presence1
```

Complex organ projections use fabric CBOR version 1. Event1 CognitiveEnvelope encoding remains
separate from generic projection encoding and from canonical Journal hashing.

## Lifecycle

The NixOS module installs each organ as a `systemd --user` `Type=dbus` service. The services are
D-Bus activated.

They are intentionally not eagerly wanted by the graphical target: the Plasma-hosted proxy first
performs the one-time pre-M1 state-location migration, then its first Presence1 request can
activate the Mind graph.

Identity uses a volatile runtime-session marker. Restarting `identityd` inside the same user login
reloads the current identity without incrementing `sessionCount`.

The process integration suite additionally simulates a new login by removing only the volatile
session marker and restarting the process graph. Focused booted NixOS gates prove identity, exact
active-run continuity, both split-commit recovery windows, Plasma recreation, required-owner
failure, and capability-specific recovery across real system transitions. Stronger in-place
upgrade reconciliation remains an explicit hardening track.

## Durable-to-visible ordering

```text
command
→ owning organ process
→ Event1
→ eventd
→ Journal COMMIT
→ Event1 Accepted
→ workspaced admission
→ Workspace1 Changed
→ presenced Changed
→ QML proxy refresh
```

This is the implemented form of the `durable before visible` invariant.

## Forgetting

The Journal writes rows at `hash_version = 3`, whose chain covers a **split commitment**:

```text
metadataDigest    = SHA256(canonicalNonErasableEnvelopeV3)
payloadCommitment = SHA256(payload)          ciphertext once sensitive payloads exist
commitment        = SHA256(metadataDigest ‖ payloadCommitment)
```

Both halves are stored, not only their combination. After an erasure the payload commitment can
never be recomputed, so a verifier that had to recompute it in order to check the metadata would
lose the ability to check the metadata at exactly the moment forgetting made it unrecomputable. An
erased row still proves its author, causality, kind and privacy.

Verification therefore answers on two axes. `status` and `brokenAt` describe the chain and stay
checkable forever; `contentVerified`, `contentSkipped` and `contentBrokenAt` describe payloads. A
payload that disagrees with its commitment is a content failure at a known sequence with the chain
intact, and an erased payload is **skipped, never counted as verified**. Rows at hash versions 1 and
2 verify exactly as before and are not erasable, because their hash covers the payload by value.

Erasure is a three-step protocol, because a database transaction and a key store cannot commit
together:

```text
1. ErasureRequested        durable contribution, target and typed reason
2. destroy DEK             idempotent, safe to repeat after any crash
3. transaction:            redact payload, set erased_at, bump erasure_epoch,
                           append ErasureApplied
```

A request with no application is the only state a crash can leave, and `incompleteErasures()` makes
recovery a question the Journal answers by itself. The epoch lives in a table row rather than a
pragma so it is bumped inside the redaction transaction: a projection must never see a redacted
payload while believing its cached view is current.

Erasure reaches what was derived from its target. `retentionDependents()` takes the transitive
closure over causation and evidence edges, so a `Learning` that says "because X" is redacted with
X — it is biography rather than a cache, and leaving it would destroy the record while keeping the
reasoning that restates it. Erasure records are excluded from their own closure, and a contribution
that merely happened afterwards is not a descendant.

`Event1.Submit` refuses erasure kinds. Destroying biography is not reachable through the call that
records a thought about it.

`cybou-crypto` provides the primitive the sensitive path will use: randomized XChaCha20-Poly1305
through libsodium, per-contribution data keys, key wrapping under the same primitive, and key
domains identified by an opaque UUID and epoch rather than by what they protect. Sealing one
plaintext twice yields two different commitments, and a guesser holding the plaintext *and* the key
still cannot reproduce a surviving commitment. **No payload is encrypted yet and no perception
source is sensitive.**

## Current cognitive substrate

The present tree has implementation boundaries for:

- canonical durable causal history;
- identity ownership;
- intention state;
- prediction/calibration state;
- self projection/assessment;
- bounded Workspace attention;
- presentation aggregation;
- process-level health/failure isolation;
- persistent capability health and typed deficits;
- persistent lifecycle runs, consolidation, recovery, and evidence-bound scheduling;
- capability-aware Presence projection and bounded compound RPC.

These components are intentionally useful without any language model.

## Not implemented yet

The current tree does **not** yet implement:

- in-place upgrade reconciliation beyond the tested schema migrations;
- **sensitive payload storage**: the AEAD primitive, key store and erasure protocol exist and are
  tested, but no payload is encrypted and no perception source is sensitive. ADR-0027 still forbids
  ingesting one until the remaining ADR-0028 gates are green;
- automatic retention expiry: `retainUntil` is decided in ADR-0028 and not yet carried on the
  envelope, so nothing acts on a lifetime;
- associative memory: ADR-0029 and ADR-0030 are Accepted and `cybou-contextd` does not exist;
- M7 inter-node transport, replication, or partition handling;
- M8 optional language faculty;
- M9 planning/authorization/executor pipeline for privileged external actions.

A UI or current organ method should not be described as providing those future capabilities unless
the corresponding milestone is implemented and gated.

## Current limitations

- owner-internal and standalone legacy reads remain synchronous, but every compound Presence path
  has one bounded monotonic request budget;
- same-user IPC authorization is not yet a capability security boundary;
- stronger in-place upgrade/reconciliation guarantees remain a hardening track;
- erasure is implemented for the live database and reaches derived state through the epoch, but the
  backup story is decided and unbuilt: a backup taken before an erasure, with a recovery root that
  still unwraps it, defeats that erasure;
- no inter-node transport exists;
- no model-selection/context policy for M8 exists;
- no authorization policy or typed privileged executor for M9 exists.

## Milestones

- M1: complete.
- M2: complete.
- M3: complete after the M3 compile repair included by M4.
- M4: implementation present; repository gates remain the acceptance authority.
- M5: evaluation milestone complete; lifecycle, continuity, consolidation transaction, Presence
  projection, process/Plasma/reboot fault injection, and clean VM/ISO evidence are implemented.
- M6: complete. P6.1–P6.6 implement the health graph, persistent snapshots, typed homeostasis,
  capability-aware Presence, authorized evidence-bound automatic scheduling, degraded behavior,
  recovery fault matrix, and focused KVM gate.
- P6.7: complete post-M6 latency hardening. Compound Presence mutations and reads share monotonic
  budgets and cannot multiply per-owner transport deadlines.
- P6.8: complete. Substrate audit repair — durable commit mode enforced at open, presenced health
  derived from real projection outcomes, consolidation backlog counted by aggregate query, user-unit
  hardening limited to directives a user manager can enforce, and the whole Presence surface moved
  to non-blocking asynchronous transport. Scalable biography replay and incremental verification
  remain open and are carried into M7 with the scale budgets that give them a target.

See `ROADMAP.md` for the capability meaning of M5–M9.
