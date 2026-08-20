<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Current State

> Debian 13 and Rust are the active production target. The entire Mind suite has been ported to pure Rust
> within the unified Cargo workspace: `cybou-crypto`, `cybou-storage`, `cybou-eventd`, `cybou-identityd`,
> `cybou-healthd`, `cybou-selfd`, `cybou-predictord`, `cybou-intentiond`, `cybou-perceptiond`,
> `cybou-epistemicd`, `cybou-contextd`, `cybou-workspaced`, `cybou-lifecycled`, and `cybou-presenced`,
> complete with systemd user units and zbus D-Bus service implementations. The C++/Qt/NixOS
> implementation has been removed: nothing installed its packages and no Journal it wrote exists,
> so the compatibility it proved had no data to protect. The canonical byte fixtures it produced
> remain under `fixtures/` and the Rust tests still verify against them — which is weaker, because
> fixtures can drift together with the code that reads them, and stronger than nothing. See
> [ADR-0038](adr/ADR-0038-rust-first-codebase.md) and [ADR-0039](adr/ADR-0039-debian-13-base-system.md).
>
> Passages further down describe Plasma, NixOS VM/KVM gates and the Qt Presence proxy. They record
> what that platform did and what its gates proved, and none of it is evidence about the current
> one: those gates were removed with the platform they ran on. Where a Debian replacement exists it
> is named in place; where none exists, the property is unproven rather than inherited. The gates
> that run today are listed in [TESTING.md](TESTING.md).

## Rust Mind foundation status

> Verification note (2026-08-19). Until this date the statement below was true only of the code
> that compiles on any platform. Everything behind `cfg(target_os = "linux")` — the twelve daemons,
> their D-Bus surfaces and the Event1 origin-authentication path — had drifted out of sync with
> zbus 5 and did not build at all, because `cybou-fabric` failed first and thirteen crates depend
> on it. The journal-writer oracle did not compile either — it has since been removed with the
> rest of the C++ tree — and the multi-daemon integration script
> had never run. All of these now pass on the Debian 13 builder, which is what "builds" below is
> claiming. A daemon listed as complete means its gate runs and passes, not that its behaviour has
> been exercised beyond what that gate covers.

The repository builds all Mind services from one locked workspace:
- `cybou-crypto`: XChaCha20-Poly1305 payload sealing, `KeyStore`, `KeyDomain`, and key epoch erasure.
- `cybou-storage`: Canonical SQLite Journal reader/verifier and atomic `JournalWriter`.
- `cybou-eventd`: The single canonical Event1 D-Bus daemon (`org.cybou.Mind.Event1`).
- `cybou-identityd`: Identity continuity across reboots (`org.cybou.Mind.Identity1`).
- `cybou-healthd`: Capability health evaluation and snapshot projection (`org.cybou.Mind.Health1`).
- `cybou-selfd`: Self-model, narration, and autobiographical assessment (`org.cybou.Mind.Self1`).
- `cybou-predictord`: Empirical forecasting, calibration, and outcome settlement (`org.cybou.Mind.Predictor1`).
- `cybou-intentiond`: Commitments and obligations tracking (`org.cybou.Mind.Intention1`).
- `cybou-perceptiond`: Linux system perception adapter (`org.cybou.Mind.Perception1`).
- `cybou-epistemicd`: Epistemic proposition projection and belief validity (`org.cybou.Mind.Epistemic1`).
- `cybou-contextd`: Associative and situational context management (`org.cybou.Mind.Context1`).
- `cybou-workspaced`: Global Workspace Theory attention coalitions and focus selection (`org.cybou.Mind.Workspace1`).
- `cybou-lifecycled`: Sleep/wake lifecycle and consolidation scheduler (`org.cybou.Mind.Lifecycle1`).
- `cybou-presenced`: Unified Mind presentation and command gateway (`org.cybou.Mind.Presence1`).
- `cybou-web-gateway` & `living-canvas`: Unified browser and desktop frontend.

## What is wired to what (2026-08-20)

Every daemon above existed before this date and most of them were connected to nothing. Listing an
organ says it runs; this section says what actually reaches it, because the two were not the same
thing and the difference was invisible from the code.

Contributions flow one way. `perceptiond` observes the host — the operating system plus kernel
version, hostname, CPU count and total memory, each contributed once and then only when its value
changes — and submits to Event1. `identityd` records session starts. `intentiond` records a formed
intention as a `Kind::Intention` contribution caused by whatever prompted it, and its conclusion as
the one terminal `Outcome` of that contribution; an intention formed with nothing to cite stays in
its own organ, because a derived kind must name a cause.

Three organs derive their state from that flow and follow Event1's `Accepted` signal to do it:
`epistemicd` forms beliefs keyed by the subject of each observation, `contextd` activates a concept
per subject and links subjects that co-occurred in one episode as `TemporalCooccurrence`, and
`workspaced` keeps attention over the recent tail. All three previously read only the beginning of
the Journal, once, at startup.

`healthd` probes `Ready` on every organ each five seconds and emits `Changed` only when the
capability states actually differ; `presenced` re-emits that as `Presence1.Changed`, which is what
wakes the gateway's event stream. `eventd` replays a bounded page of the hash chain every thirty
seconds from a persisted checkpoint, and reports verification as a position rather than a verdict,
so `Verified` means the chain was replayed to the head and nothing weaker.

Two organs are wired but not complete against their ADRs, and the difference matters more than the
wiring did. `Context1` is live and reconstructible: it rebuilds from sequence zero on every start and keeps
nothing on disk, because a saved graph is the one structure in the system that could outlive the
evidence behind it. It now holds
two of ADR-0029's invariants: the graph is bounded by node and edge budgets, dropping the least
salient concept and the weakest link rather than whatever arrived first (A2, A11), and an erasure
epoch discards the projection and rebuilds it from the surviving Journal, so a derived index
neither outlives the evidence a person destroyed nor loses the context that erasure did not touch
(A7). An association also inherits the most restrictive privacy and the shortest retention of its
evidence, and corroboration can only tighten them (A9): a derived claim that came out looser than
what it was derived from would be a way to launder a private fact by observing it twice. Still
missing: depth, time and token budgets, which belong to an activation session this version does not
have, and epistemic status, which defers to `Epistemic1` by design. Its status is
`live integration implemented, ADR-0029 partial`.

`Lifecycle1` schedules one piece of maintenance and only one. After fifteen minutes without a
person, and at most once every six hours, it moves to `Consolidating` and asks Event1 to
re-verify the whole chain page by page, then returns to `Awake`. Both conditions are required:
on a machine nobody touches, idleness alone means always.

The sweep exists because the incremental pass trusts a checkpoint and never looks behind it, so a
row that rots after it was verified would never be questioned again. It rewrites nothing —
consolidation is not permission to revise a biography — it never writes the checkpoint the
incremental pass trusts, and it stops between pages the moment someone arrives. `LifecycleRun`
remains a shape nothing constructs, and no other mode has work behind it yet.

`presenced` accepts commands by asking owners: Promise reaches Intention1, Reflect asks Self1 for an
assessment, Observe and Predict reach Predictor1, InterruptLifecycle tells Lifecycle1 a person is
present. It holds none of that state itself. No gateway route exposes any command; the web surface
is read-only.

`Predictor1` follows Event1 and forecasts the subjects the Journal actually holds measurements for.
It reads an observation's own subject and its numeric value; observations whose value is words are
facts about the world rather than a series, and are left to `Epistemic1`. Today that means
`cpu-count` and `memory-total-kib` from `perceptiond`, plus whatever a person asks to track through
`Presence1.Observe`. Its samples and the Journal position they stand at are one write, so a restart
resumes after the row it last learned from. Calibration still needs a person to settle a forecast
against an outcome: nothing in the system settles its own predictions yet.

`cybou-fabric` and `cybou-runtime` carry the bounded RPC policy and the state foundation. Retry
eligibility distinguishes read-only, idempotent, and non-idempotent operations; one outer deadline
bounds all attempts, delay is capped with deterministic jitter, and the circuit breaker admits only
one half-open probe. Runtime paths follow the predecessor XDG contract. Legacy-state migration
preflights every collision before moving data, preserves unrelated target entries, and attempts the
entire reverse-order rollback after a partial failure.

The Linux-only `cybou-fabric::zbus_rpc` executor applies that policy to real asynchronous D-Bus
method calls and returns the raw successful reply for owner-specific decoding. It preserves typed
timeout, unavailable, rejected, unknown-outcome, and circuit-open results and counts only actual
bus dispatches as attempts. Presence `Snapshot` is the first production caller and uses a single
900 ms outer budget across proxy creation, dispatch, retries, and delays.

`cybou-perception` is the acquisition boundary. `LinuxSystemSource` reads `/etc/os-release`, and
`LinuxHostSource` reads the kernel version, hostname, CPU count and total memory. A source that
cannot be read yields no observation for that subject rather than an observation of nothing. The
facts are deliberately ones that stay put: load and free memory change on every read, and a Journal
that is a biography should not fill with the fact that a number moved.

`cybou-protocol` owns Observation v1, the first byte-proven cognitive payload. Rust validates
RFC3339 instants, a strictly forward freshness horizon, non-null typed evidence, and non-empty
provenance before encoding; UUID identity is derived from the validated acquisition time rather
than a second caller-supplied clock. Its canonical bytes were proven against the Qt implementation
while that existed; the fixtures produced then are checked in and the tests still verify against
them, which is weaker than a second implementation answering live and is what remains once there is
no Qt-written data left to be compatible with.

The Rust storage foundation is `cybou-storage`. It opens existing Journals with explicit SQLite
read-only and no-follow flags, requires `user_version=2`, verifies required tables and contribution
columns, and reports row count plus erasure/rotation epochs. Tests prove missing paths stay missing,
future and partial schemas fail closed, and compatible databases remain unchanged after inspection.
Inspection walks the complete chain shape: sequence numbers must be contiguous, each `prev_hash`
equals the preceding stored hash, hash versions are 1–3, hashes are 32 bytes, and v3 rows carry both
32-byte commitments. The protocol crate reproduces canonical envelope v2, non-erasable v3, Journal-row
v2, split-commitment v3, and Journal-row v3 byte streams plus SHA-256 digests. The storage verifier
decodes persisted envelopes and evidence for full hash replay (v1 concatenation, v2 by-value row,
v3 split-commitment verification with payload commitment verification). Erased payload bytes are skipped
identically to the predecessor while surviving metadata remains verified.

The storage crate also exposes a typed read-only checkpoint and suffix-verification result, and a
row-bounded page API for large history replay. `cybou-eventd` is the canonical Rust Event1 daemon
owning consumer offsets and checkpoint persistence.

The admission rules are strictly separated in `cybou-protocol::admission`: it decides what may enter
the Journal without touching storage — structural envelope validity, frozen kind/privacy/sensitivity
numbering, root-versus-derived reference shape, sealed-payload schema and key-domain requirements,
and the reference-dependent rules — duplicate identity, missing cause or evidence, privacy that may
not be weakened, retention that may not be outlived with the erasure kinds exempt, sensitivity checked
only for schema 4, and one terminal Outcome per cause. The caller resolves each reference and supplies
four facts about it, so no database is reachable from the rules. Unknown schema, kind, privacy and
sensitivity values are refused rather than defaulted, and every rule refuses rather than silently
correcting a declaration.

`cybou-protocol` additionally carries five **contract-only** modules — `meaning` (ADR-0031),
`learning` (ADR-0032/0033), `governance` (ADR-0034/0035), `action` (ADR-0022/0036) and `security`
(ADR-0036). They fix the shared spelling of the types the future agent/worker runtime, model broker,
authorized action executor and security control plane will exchange. No daemon depends on them and
no runtime enforces them: they are vocabulary, not behaviour, and the README statement that this
repository does not yet implement those subsystems remains accurate.

`cybou-storage::writer` provides the atomic SQLite write engine. It creates schema v2 with required
tables, indexes, and the partial unique index enforcing one terminal Outcome per cause; reads WAL
and `synchronous` back and refuses to open when either did not take; refuses a declared schema with no
tables rather than repairing it; and refuses a v1 database rather than migrating it while opening a
connection. `append` performs the reference reads, the tail read, and the insert inside one
`BEGIN IMMEDIATE` transaction, chains hash v3 over the split commitment, and stores canonical column
spellings. Rows it writes are cryptographically replayed by the existing read-only verifier.
`cybou-crypto` provides sealed payload encryption via XChaCha20-Poly1305 and KeyStore management.

`cybou-eventd` wraps `JournalWriter` and exposes `org.cybou.Mind.Event1` over D-Bus with fail-closed
origin authentication (`RESERVED_ORGAN_IDENTITIES`), consumer offset tracking (`consumer-offsets.json`),
and emits `Accepted` signals only after the SQLite transaction commits.

Concurrency and migration are present with the same discipline. The write lock is taken before the
reference reads, so a contended append is refused at the start rather than after the admission rules
have been decided against state that moved, and a busy database surfaces as its own typed result
rather than a generic query failure. A refused append leaves the database byte-identical.
`migrate_v1_to_v2` is an explicit call rather than something that happens while opening a
connection: it takes a `VACUUM INTO` backup before its transaction opens, converts legacy
comma-joined evidence into ordered join-table rows while refusing malformed, duplicate, or dangling
identities, refuses a cause carrying more than one terminal Outcome, and verifies the whole legacy
chain at hash v1 before committing. A partially versioned schema is refused rather than repaired.
Interruption leaves either a v1 database with a backup beside it or a complete v2.

The writer also has a batch path and a scale probe. `append_batch` shares one commit across many
contributions while validating, hashing and chaining each exactly as a single append does; it exists
for fixture construction and is deliberately unreachable from Event1, where acceptance must stay per
contribution. `cybou-journal-scale` measures build, append, full verification, paged verification
and row size against the recorded budgets. Every per-contribution cost is flat across an order of
magnitude, reproducing the predecessor's linearity finding through an independent implementation;
the absolute values come from a different host than the C++ baseline and are not a comparison
between the two writers.

`cybou-journal-inspect` exposes one such page as a Debian-native command-line probe. It accepts an
existing database path, positive row budget, and optional sequence/hash checkpoint; it prints only
verification counters and the next checkpoint, never contribution payloads. The tool performs no
copy, checkpoint persistence, repair, migration, or write.
A deterministic 513-row v3 chain now exercises the page boundary at 64 rows: every page stays
within the declared budget, checkpoints advance monotonically, nine pages cover the exact history,
and the final checkpoint equals a full replay. This is a correctness/complexity regression gate,
not yet a production latency SLO. The current Debian public host contains no Journal in the
canonical XDG paths and remains fixture-backed, so a real predecessor-database differential run is
still pending.

The additive W1 gateway seam is also present. `cybou-web-gateway` binds only to
`127.0.0.1:8787`, exposes typed read-only `/api/v1/session` and `/api/v1/snapshot` routes, applies
no-store and browser-security headers, enforces an outer projection timeout, and has no generic RPC
or mutation route. Its Linux zbus adapter calls the existing `Presence1.Snapshot`, validates the Qt
CBOR fabric envelope, and maps capability states into the separately versioned web contract. Legacy
Nix expressions remain only until Debian-native checks and packaging replace their evidence. When
`CYBOU_WEB_ROOT` names a
Trunk output directory, the gateway serves that artifact from the same origin as the API. Living
Canvas uses the async `GatewayMindClient` for its production browser build; `MockMindClient` remains
only as the deterministic test boundary.

This is not yet a complete W1 delivery path. The gateway exposes `/api/v1/events` as bounded SSE:
snapshot events carry their cursor as the SSE ID, browser reconnect uses `Last-Event-ID`, duplicate
cursors are suppressed, and keepalives retain idle connections. The Linux zbus source establishes
and retains a native `Presence1.Changed` signal stream before serving snapshots, so changes queue
across the snapshot/wait boundary. Deterministic non-D-Bus sources retain a bounded two-second
polling fallback. Desktop bootstrap authentication is still absent.
An operational deployment runs at `https://vps-d0669a91.vps.ovh.net` and directly at
`https://51.255.46.58`: Caddy terminates public TLS while the Rust gateway remains bound to
loopback. It is a working desktop over the live Mind, and it is unauthenticated. That is the
owner's deliberate and temporary choice, made while no authentication boundary exists and the host
holds nothing worth protecting; the intended replacement is authentication plus a demo user granted
per request. Until then everything the deployed Mind observes is public.
The gateway runs as a user unit in the `cybou` user manager so it shares the session bus with the
organs — as a system service it had no session bus and could reach nothing, which is why this
deployment served fixtures until 2026-08-19.
Its server-issued session mode is `publicPreview`, so the browser cannot mistake this deployment
for a device-bound local desktop or an authenticated remote session. The mode names the trust that
was established, which is none, and that remains true with live state behind it.
The repository now also contains independently buildable `cybou-web-ui` and
`cybou-desktop-shell` derivations. The development VM offers an opt-in `Cybou Living Canvas`
Wayland session: Cage owns the single surface, Chromium/Ozone opens the loopback application origin,
and an ephemeral runtime profile is used. This is W2 preview plumbing, not desktop parity or the
default session; Plasma remains the fallback, and lock screen, multi-display, input-method,
accessibility, renderer recovery, and navigation-policy gates are still open.

Status date: 2026-08-20.

This document is intentionally limited to implemented behavior and current limitations.

## Build and evaluation environment

The sole active Linux build and deployment environment is Debian 13 at
`debian@vps-d0669a91.vps.ovh.net`. `scripts/vps-checks.sh` transfers the unfinished working tree and
runs Cargo, Clippy, and WASM gates remotely. WSL and NixOS are not active targets. The old in-place
Debian-to-NixOS conversion remains permanently forbidden.

## Repository gate status

The P0 baseline is green: formatting, REUSE 3.3, package metadata, cognitive documentation, Mind
access, QML API, UI polish, `cybou-mind`, and `cybou-presence-applet` pass through pinned Nix checks.
The Mind package runs thirty-seven CTest suites, including Event1, lifecycle persistence/recovery,
Lifecycle1 process restart, and multi-process integration across the twelve Mind owners. Both counts
are checked against the build rather than trusted: the documentation validator derives them from the
package's daemon list and the tests CMakeLists, so a document that falls behind the code fails the
build instead of quietly misdescribing it. The process suite also proves a
simulated new login preserves identity and an accepted open intention while incrementing the
logical session count, and that compound Presence reads and mutations obey one bounded deadline.

The M5 lifecycle owner is present: lifecycle schema v1, legal mode transitions, atomic persistent
run state, `org.cybou.Mind.Lifecycle1`, D-Bus/systemd activation, D-Bus run requests, and restart
recovery of an active run into `Recovering`. Legacy v0 state is backed up and migrated to v1;
unknown future versions fail closed. Two gates run against the deployed host.
`scripts/test-systemd-continuity.sh` proves the owners recover from process death: identity and
Journal survive `systemctl restart cybou-mind.target`, the session count advances, and the start it
counted is a contribution Event1 holds. `scripts/test-reboot-continuity.sh` reboots the machine and
proves the Mind comes back on its own with nobody logged in — which is what lingering, unit
enablement and a user manager without a session amount to — with the same subject, an advanced
session whose start Event1 holds, a Journal that did not shrink, a chain Event1 can still answer
for, and the read-only surface serving again. The boot id is compared on both sides, so a reboot
that silently did not happen fails rather than passes.

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

Three boot cycles were covered by the headless NixOS gate — baseline active-run/identity
continuity, and both split-commit recovery windows — and that gate went with the platform. The
split-commit behaviour it exercised is still implemented and still covered by unit and process
tests; what is no longer covered is the same behaviour across a real boot.

Plasma was the desktop of the removed Qt/NixOS platform, and the VM gate that restarted it around
an active run went with it. There is no shipped desktop shell on Debian yet, so there is nothing
for a UI-recreation gate to be about.

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
eventd verified rather than accepted; a restart re-reads and records once, then falls silent. The epistemic projection lives in
`cybou-epistemicd` itself; there is no separate library. It derives `observed`, `stale`, `disputed`
and `superseded` from accepted observations: an unchanged restatement is re-affirmation rather than
replacement, a different value from a source that is still within its declared freshness horizon is
a disagreement it refuses to resolve, and a different value arriving after that horizon has passed
is the newer report taking the place of the older one rather than an argument with it. Staleness is
decided when a belief is read, against the reader's clock and the horizon the observation itself
named, so a belief nothing has restated does not go on reading `observed` for as long as the system
happens to stay quiet. Beliefs and the cursor that produced them are written as one value, and
beliefs derived by an older rule version are rebuilt from the Journal rather than trusted because
they are on disk.

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
that model. P6.7 is post-M6 latency hardening; the tree does not yet contain the planned M8 meaning boundary,
the M9 learning runtime, the M10 authorized executor, the M11 agent, worker and model runtime, or
the M12 security control plane.

## Process topology

Mind now has twelve real user-session processes:

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
cybou-contextd
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
session marker and restarting the process graph. Booted NixOS gates once proved identity, exact
active-run continuity, both split-commit recovery windows, required-owner failure, and
capability-specific recovery across real system transitions; they were removed with NixOS and have
no Debian replacement for the boot-crossing part. Stronger in-place
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
- M8 typed meaning boundary and language implementations;
- M9 lifelong learning and learned-artifact governance;
- M10 planning/authorization/executor pipeline for privileged external actions;
- M11 agent, worker, model and tool runtime with grants and brokers;
- M12 autonomous security and operations control plane;
- M13 distributed perimeter governance.

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
- no model-selection policy for M8 exists, though ADR-0030 delivery already governs what any
  consumer may receive;
- no authorization policy or typed privileged executor for M10 exists;
- no agent, worker, model broker or tool broker for M11 exists;
- no firewall, endpoint, credential or remediation control plane for M12 exists.

## Milestones

- M1: complete.
- M2: complete.
- M3: complete after the M3 compile repair included by M4.
- M4: implementation present; repository gates remain the acceptance authority.
- M5: lifecycle, restart continuity, the consolidation transaction and the Presence projection are
  implemented and gated on Debian. The reboot and VM/ISO evidence behind the original evaluation
  belonged to the NixOS platform and does not stand for the current one.
- M6: complete. P6.1–P6.6 implement the health graph, persistent snapshots, typed homeostasis,
  capability-aware Presence, authorized evidence-bound automatic scheduling, degraded behavior,
  recovery fault matrix, and focused KVM gate.
- P6.7: complete post-M6 latency hardening. Compound Presence mutations and reads share monotonic
  budgets and cannot multiply per-owner transport deadlines.
- P6.8: complete. Substrate audit repair — durable commit mode enforced at open, presenced health
  derived from real projection outcomes, consolidation backlog counted by aggregate query, user-unit
  hardening limited to directives a user manager can enforce, and the whole Presence surface moved
  to non-blocking asynchronous transport. Scalable biography replay remains open and is carried
  into M7 with the scale budgets that give it a target.
- Incremental verification is no longer open. `eventd` replays a bounded page of the chain from a
  persisted checkpoint on a timer, and a break is never checkpointed, so the trusted position stays
  where the chain was last intact.

See `ROADMAP.md` for the capability meaning of M5–M9.
