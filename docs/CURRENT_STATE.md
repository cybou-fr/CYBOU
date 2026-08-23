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
> that compiles on any platform. Everything behind `cfg(target_os = "linux")` — the daemons,
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
- `cybou-meaningd`: The meaning boundary of ADR-0031 (`org.cybou.Mind.Meaning1`).
- `cybou-epistemicd`: Epistemic proposition projection and belief validity (`org.cybou.Mind.Epistemic1`).
- `cybou-contextd`: Associative and situational context management (`org.cybou.Mind.Context1`).
- `cybou-workspaced`: Global Workspace Theory attention coalitions and focus selection (`org.cybou.Mind.Workspace1`).
- `cybou-lifecycled`: Sleep/wake lifecycle and consolidation scheduler (`org.cybou.Mind.Lifecycle1`).
- `cybou-presenced`: Unified Mind presentation and command gateway (`org.cybou.Mind.Presence1`).
- `cybou-jailfs`: Sandboxed filesystem containment library with canonical root validation, symlink traversal prevention, and bounded quota enforcement.
- `cybou-shelld`: Bounded Body capability engine strictly confined to the ADR-0040 DemoReadOnly builtins accepted in that ADR's Amendments 1 and 4 (`help`, `pwd`, `ls`, `cd`, `cat`, `echo`, `stat`, `head`, `tail`, `grep`, `wc`, `du`, `file`, `find`, `date`, `clear`). Walking commands are bounded and say so when a bound was reached. `whoami` and `uname` were withdrawn on 2026-08-22: both answered with a compiled-in constant on every host, which in a terminal reads as an observation of the Body and was an observation of nothing.
- `cybou-web-gateway`: Loopback-bound HTTP & SSE server providing read-only projections (`/api/v1/snapshot`, `/api/v1/mind`, `/api/v1/events`, `/api/v1/session`), and sandboxed execution endpoint (`POST /api/v1/shell/exec`) with strict refusal for a caller who owns no shell (HTTP 403) and execution for a live session or the local desktop seat. Shells are held per owner, not per process (`shells::Shells`).
- `living-canvas`: Unified browser and desktop frontend (**CYBOU Desktop**), implementing the generic Card model over twelve canonical system cards, Layout schema v9 with transparent v8 migration and startup self-healing normalization (`validate_and_normalize`), Spatial Compositor Invariants L1–L15, invariant-safe Deck composition with `DeckError`, interactive resize, magnetic snap guidelines (`compute_snap`), auto-scaling viewport fit ("Fit All" `Ctrl+0`), interactive minimap navigation, card pinning & collapsing, non-destructive Viewport Focus mode, accessible keyboard navigation (`Alt+Arrow` move, `Alt+Shift+Arrow` resize, WAI-ARIA tablist), automated multi-mode Arrangement Engine (`Grid`, `Compact`, `Relations`, `Focus`, and `Ctrl+K` command palette actions), live UTC desktop clock, clean SSE lifecycle teardown, and strictly truthful session/capability representations (Phases 0, 1, 2, 3, and OS-Grade Grounding completed; see [ADR-0040](adr/ADR-0040-spatial-card-desktop-and-bounded-body-capabilities.md)).

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

## The shell surface, and the helper that guards the way in (2026-08-22)

Two things were true of the surfaces a person actually touches, and neither was visible from the
code that implemented them.

`/api/v1/shell/exec` locked one `ShellEngine` held for the whole gateway process. A shell has state
— where it is standing — so that state was shared: two people signed in to two accounts issued `cd`
into the same variable and each of them moved the other. No file crossed a boundary the sandbox
would not have opened anyway, because the jail root is the same either way. What was wrong is
narrower and worse to leave standing: a working directory that answers to somebody else is a claim
about who is at the keyboard, and it was false.

Shells are now held per owner in `shells::Shells` — a live session named by its token, or the
desktop seat, which is one seat and carries none. A caller who owns neither is refused rather than
handed whichever shell was first, so entitlement and identity are the same question and it is asked
once. Signing out ends the shell with the session, because a token can be reissued and a shell that
outlived its session would hand the next holder of that name a place somebody else chose. Idle
shells expire on the session lifetime. The sandbox root is still shared deliberately: ADR-0040
bounds the Body to read-only builtins over one demonstration root, and a private root per session
would be a different capability rather than a fix to this one.

`cybou-authd` held every failed attempt for 750 ms and then answered. The delay was there so that a
wrong password and an account that does not exist take the same visible time, and it does that. It
was not a limit on guessing, and it read like one: the helper spawns a task per connection, so a
caller opening a hundred connections paid the 750 ms a hundred times in parallel, which is the same
as paying it once.

It now costs what it claims to. `Throttle` doubles an account's delay per consecutive failure from
750 ms to a cap of thirty seconds, forgets a run of failures after fifteen quiet minutes, and
forgets them immediately on success. It is deliberately not a lockout: a lockout turns knowledge of
a username into the power to deny that account, so the failure mode of the defence would be the
attack. Because backoff is per account and a caller can invent account names, a semaphore of four
permits is held across the delay, which bounds the whole socket to roughly five attempts a second
regardless of how many names or connections are used. The table of tracked accounts is bounded and
evicts the least recently failed, since its keys come from the caller. All of it is portable code
with tests that run on every push, which is the point: the property is checked where the check runs.

A third thing was smaller and of the same kind. A session token is a bearer credential — whoever
holds it is the session — and it was a v4 UUID used as one. The randomness underneath was
adequate; what was wrong is that nothing about the type said it had to be, a UUID spends six of its
bits saying which kind of UUID it is, and reading a secret as an identifier is how an identifier
ends up somewhere a secret should not be. Tokens are now thirty-two bytes from the operating
system's generator, and if that generator will not answer, the login is refused with a retryable
error rather than issued a predictable token.

Sessions are also filed under the SHA-256 of the token rather than the token, so the value that
grants access is not sitting in the process as a map key, and the time a lookup takes says nothing
about the secret. The shell registry keys on the same digest for the same reason.

## The Shell says less, and means it (2026-08-22)

Four of the Shell's builtins were answering with values nobody had established, in the one place a
person reads output as observation. `whoami` printed `cybou` and `uname -a` printed a fixed kernel
string — on every host, for every caller, compiled in. `ls -l` printed `-rwxr-xr-x 1 cybou cybou`
for every entry and `4096` for every directory. `stat` printed `Access: (0644/-rw-r--r--)` for
everything, including directories.

None of this was a sandbox weakness: there is still no `exec`, no pipeline, no redirection, and
every path goes through `cybou-jailfs`. It was a truthfulness failure, which for a surface whose
whole claim is that a projection may not state what nobody established is the more serious kind.

`whoami` and `uname` are withdrawn; both exit 127 like any other unknown command, and neither
appears in `help` or in the frontend's completion list. A command that can only answer with a
constant is not a small capability, it is a false one. `ls -l` now reports type, size and name, with
a dash where a directory's size would be, because the sandbox does not establish it. `stat` reports
the file, size and type it actually read and no access mode at all.

The accepted set is eleven and is enumerated in
[ADR-0040 Amendment 1](adr/ADR-0040-spatial-card-desktop-and-bounded-body-capabilities.md). Before
that amendment the ADR said six, this document said thirteen, `TESTING.md` said six and the engine
recognised thirteen — four statements, three answers, and the Accepted decision was the one nobody
had changed. Extending the set again requires amending that ADR in the same commit as the code.

## The host can be asked what is going on with it (2026-08-23)

```text
/proc text → reading → bounded window → baseline → finding → plan → prose
```

The detector concluded and the conclusion stopped at a struct. `meaning::plan_system_state` closes
the *explain* stage, and an end-to-end walkthrough runs the whole path from what a kernel writes to
what a person reads — with no network, no model, and no `/proc`: the kernel output is a string
literal, which is what makes it a test rather than an observation about whichever machine ran it.

Three things are decided in the typed layer rather than by whoever writes the sentence, and each is
a way this answer is normally lost.

**A finding is a hypothesis, and the words say so.** *The cause is memory pressure* and *this is
consistent with what I observed* are the same struct and different claims. The first is what a
fluent renderer produces when nobody decided; the second is what the evidence supports. A test at
the far end asserts the prose never says "the cause".

**An all-clear is qualified by what was never looked at.** A host whose kernel was built without
pressure accounting can honestly say nothing needs attention *among the things it watches*. Saying
it plainly would report an absence of evidence as evidence of absence, on the one surface a person
consults to decide whether to go back to sleep. So the unwatched subjects are named and the answer
carries `NotRead` — as a typed qualification and not only as a sentence, because a hedge living only
in prose is one that `compose` will drop a layer over.

**Not having watched long enough is its own answer.** For the first minutes after a restart there is
no notion yet of what is ordinary here, and *I have not been watching long enough* is a different
answer from *nothing is wrong*. A confident all-clear built on four readings is worse than silence,
because it will be believed.

Both honesty rules were checked by breaking them: disabling the two of them fails three tests rather
than none.

Eight walkthrough tests, including the control that a fully-observed calm host does not hedge —
without it, a planner that hedged unconditionally would pass everything, and a hedge that is always
there is the same as no hedge.

### Where S0 stands

```text
observe → understand → remember → diagnose → explain → propose → authorize → act → observe outcome
   ✅          ✅           ✅          ✅         ✅         ❌          ❌       ❌         ❌
```

*Explain* closed today. **S0 is not passed**, and the walkthrough says so in its own header rather
than implying otherwise: the gate asks for typed action proposals as well, and nothing proposes an
action yet. What is true is that a host with no network and no model now observes itself, notices
what is wrong, and can say why it thinks so, in two languages, from evidence it gathered.

## Something finally watches the Body (2026-08-23)

`cybou-telemetryd` is the fourteenth Mind owner, and the first thing in this tree that observes the
machine between one restart and the next. Until now perception recorded what was stable — kernel,
hostname, memory size — and nothing looked again, so Cybou could say what it knew and not what was
happening. ADR-0041 S0 was unreachable for one reason: **a system cannot detect a problem it never
saw**, and every stage after *detect* had nothing to work on.

The whole design is one line, and it is a line rather than a caveat: **telemetry is not biography.**
A `Reading` has no path into the Journal anywhere in this tree — no `Kind`, no conversion, nothing.
It is transient by construction rather than by policy. A `SystemInsight` does have one, and it is a
`Hypothesis` with its readings attached, because *the machine is under memory pressure* is an
inference and an inference recorded as an observation is a claim the host cannot support.

### Bounded twice, for two different failures

The window holds at most a span and at most a count, the same discipline the dialogue memory needed.
A duration alone lets a burst hold everything it produced; a count alone lets a slow sampler remember
a week while a fast one remembers four minutes, so the detector silently sees a different amount of
history depending on a setting nobody thought of as history. Both are tested against the failure they
prevent.

### Median and MAD, and why not mean and sigma

The thing being detected contaminates the thing detecting it. A host that has been swapping for ten
minutes has a mean pulled toward the fault and a standard deviation widened by it, so the fault makes
itself look ordinary — which is exactly why naive sigma monitors go quiet as a problem settles in. A
median moves only when half the window has moved, and a median absolute deviation is not widened by a
tail at all.

That is not an assertion here. A test builds a window that is one third fault and shows a sigma
detector already considers it unremarkable (under two sigma) while the robust one still puts it fifty
spreads out.

Four more properties the tests hold: a window too short to have an opinion says nothing rather than
being confident on four readings; a perfectly flat host does not report its first flicker as infinite
deviation; an ordinary reading on a quiet host is not called unusual; and the same observation is
unremarkable on a host that idles at 45% and extreme on one that idles at 4% — which is the property a
model trained on somebody else's corpus cannot have.

### What it concludes, and what it refuses to conclude

Categorical evidence and statistical deviation are kept apart and both are used. A filesystem at 96%
is a finding on a host where it has been at 96% for a month — a purely statistical detector says
nothing there, precisely because it is normal. Corroboration is what separates moderate from weak:
memory pressure alone is weak, memory pressure with swap growing is the same story told twice.

Six spreads before anything is said, deliberately far. A monitor that speaks at three is a monitor
people mute, and the failure mode of an alerting system is almost never that it missed something.

And `UnexplainedDeviation` is a finding rather than a dropped case. A detector that only reported what
it had a name for would be silent exactly when a host is doing something nobody anticipated, which is
the case an operator most wants to hear about.

### Everything that decides is testable without a kernel

`probe` parses text and returns a number or nothing; the only thing that touches `/proc` is
`read_to_string`, in the daemon. That is the same lesson as pulling arithmetic out of components, CSS
out of the compiler's blind spot, and the disclosure rule out of the D-Bus adapter — four times now,
and each time the code that could not be tested was where the defect was.

Every parser returns `Option` and none guesses. A missing `/proc/pressure` on a kernel built without
pressure accounting produces *one fewer subject*, not a zero that reads as a perfectly calm machine.
Memory is measured against `MemAvailable` rather than `MemFree`, because a healthy Linux host keeps
almost nothing free and a detector watching free memory would report every warm cache as an
emergency — a test asserts the two measures disagree sharply, so it cannot pass by accident.

Thirty-one tests. Nothing here needs an accelerator, a model, or a network, which is the point: the
detector has to work on a small instance with the network as the thing under investigation.

## Where this runs, decided (2026-08-23)

Nothing in the tree said where Cybou runs, so every decision that needed an answer supplied one
locally, and they did not agree.

The evidence pointed one way: development, integration and every deployment target a VPS, the
frontend is a browser artifact over HTTPS, and the desktop shell has never been run on a machine with
a seat. The prose pointed the other: the README said *local-first*, the model profiles assumed a
laptop with a GPU, and the brokerage unit shipped with `PrivateNetwork=yes` — a good argument on a
personal machine and the wrong default on a host whose reason for existing is to be reached.

ADR-0041 settles it. Cybou is a cognitive Linux environment for a VPS, server, VM or container.
Outside this repository: **Linux that understands and operates itself** — not Linux with a local
model, and not an AI desktop. The difference is not presentation: those describe a system whose
intelligence is a component you install, and this describes one whose intelligence is the runtime,
with models as amplifiers of it.

*Local-sufficient* replaces *local-first*: nothing Cybou needs is remote, and it is built to be
reached. A larger model may be consulted through an API — as a named external-boundary consumer, whose
answers are proposals, whose loss is a capability deficit. All of that was already written; what
changed is that it is now the common path rather than the hypothetical one, so every gate around it
gets exercised in ordinary operation.

The `PrivateNetwork=yes` shipped hours earlier is removed. What protects the boundary is not a
namespace flag but the thing that was always supposed to: no remote route is configured, adding one
is a decision, and what crosses is what a disclosure decision supplied and recorded. A flag would
have made that machinery feel optional, which is the worse outcome — the recorded, refusable route is
the protection, and it has to be the one that works.

### Two gates that define the product

**S0 — unplugged.** Cut the network and every model API. On a minimal VPS, Cybou keeps observing its
Body, answers about its own state, detects known problems, explains them from evidence, remembers its
open intentions, and forms typed action proposals.

**S0R — plugged back in.** Restore the network and a large model. Language, analysis and planning
improve sharply. Identity, memory, epistemics, permissions and minimum system control do not change
owner.

Between them: the model is an amplifier, and what it amplifies exists without it. A system failing S0
is a client for somebody else's model. A system failing S0R has handed its substrate to one.

**S0R is close to held. S0 is not, and one thing is why: nothing watches the Body.** Perception
records stable facts — kernel, hostname, memory size — and nothing observes the machine minute to
minute, so Cybou cannot detect a problem it never saw, and every stage after *detect* has nothing to
work on. That is now the top of the list, with the constraint that is the whole design rather than a
caveat on it: **telemetry is not biography.**

## A faculty, and the namespace is the claim (2026-08-23)

`cybou-model-brokerd` exports `org.cybou.Faculty.ModelBroker1` — a different namespace from every
`Mind1` interface, and the difference is the whole decision. An organ of Mind owns part of what Mind
is. This owns none of it: no biography, no Journal, no filesystem, no authorization, no execution,
and nothing about what is true. It does four things — select who answers, hold them to the budget,
put the request, attribute what comes back.

That claim is checkable rather than promised. The crate depends on the protocol and the fabric and
on no organ, and the layering validator gained a rule for it: **a faculty may not depend on any
organ, in either direction.** Organs may read the layer above them because they are part of the same
Mind; a faculty is not, and one that could name an organ's types is a refactor away from holding a
piece of what it was built to stay outside of. A dependency was injected to confirm the rule catches
it.

### What it does when there is no model

Everything, which is the point. `submit` on an installation with no worker returns not "unavailable"
but **what happens instead** — for interpretation and realization, the deterministic thing that
already does it; for the rest, the named feature this machine does not have. A faculty that answered
"no" and stopped would make `NoModel` feel like a fault. It reports `healthy`, because nothing is
wrong with it, and it reports what it *can* answer through `AnswerableTasks`, so a surface can stop
offering a feature that cannot work here rather than offering it and failing when somebody uses it.

Four refusals, kept apart because they have four different remedies: nothing here does this task at
all (install something, or use what does it instead), every route that does it refused (fix the
request or the routing table — and each route says which of the two it was), the chosen worker
failed (a capability deficit, per MB6; the faculty still has a model and still answers the next
request), and what answered was not what was registered.

That last one is refused rather than passed on with a warning. An answer attributed to an artifact
that did not produce it is worse than no answer: the surface a person uses to ask *which model told
me this* would confidently name the wrong one, and nothing downstream could tell.

Route selection is by declaration order, not by measured latency. A broker that raced its backends
would answer differently on a busy machine, and two answers a person cannot compare is a worse
outcome than a slower one.

What it remembers is that it answered — request id, task, which provider, whether it crossed a
boundary — bounded to sixty-four attempts and holding no input or output. A broker that kept prompts
would be a second memory with different rules, which is what ADR-0029 spent a whole decision
refusing. A test greps the remembered attempts for the text of the request and fails if it is there.

The unit ships with `PrivateNetwork=yes`. No remote route is configured, so the faculty has no reason
to reach one, and a deployment that adds a remote provider has to remove that line — which is the
point: a decision somebody makes rather than a default nobody sees.

**No inference runtime is implemented.** A broker whose only backend was written beside it would
have that backend's assumptions built into it, and the first real one would arrive as a rewrite.
`llama.cpp`, `mistral.rs` and an ONNX runtime are three different shapes of process; `Worker` is the
interface they share, and registering one is the whole of what installing a model will mean.

Ten tests.

## Somewhere for a model to land, before there is one (2026-08-23)

No inference runtime exists, and that is exactly why the vocabulary for asking a model something is
worth writing now. A runtime that arrives before the shape of the request does gets whatever shape
is convenient at the call site, and every constraint this substrate spent its life establishing then
has to be re-imposed afterwards, against a working system, by whoever notices.

ADR-0021 moves to **Accepted**. It was Proposed for one stated reason — its acceptance direction
asked M8 to demonstrate the decision, and M8 was not implemented. It is now. The part that could
only be shown rather than argued: the whole meaning path — interpretation, reference resolution,
planning, composition, dialogue state, realization — is deterministic, runs with no network and no
model, and is held by tests that would fail if any of it started depending on one. Accepting it does
not commit to never having a model; it commits to a substrate that does not stop meaning anything
when there isn't one.

`protocol::model` replaces the earlier `InferenceRoute` / `ModelInferenceRequest`, which named a
provider as a string, carried a sensitivity ceiling as prose, and could not say what was asked, what
came back, or which artifact answered. Nothing consumed them, so this is a removal rather than a
migration.

**A task is a closed set, versioned in its name**, because an open `String` task would be a way to
add an input shape, an output shape and a no-model answer all at once without anybody reviewing any
of them.

**Every task answers for its own absence.** `ModelTask::without_a_model` is total and returns one of
two things: something deterministic already does this, or the feature is *absent*. Absent is not
degraded and not a stub returning something plausible — a semantic search that quietly falls back to
matching filenames answers a different question than the one asked. Interpretation and realization
are the two that answer `Deterministic`, and a test holds that they stay that way: if either became
`Unavailable`, installing a model would have quietly become a prerequisite for Mind speaking at all.

**No output can assert or command.** There is no variant of `ModelOutput` carrying a truth value, a
permission, a path, or a command to run. The strongest thing a model can return is a candidate
something else has to accept. A model cannot say "the disk is failing" through this interface
because there is no field to say it in — which is stronger than checking for it, since a check can
be forgotten at one call site and a missing field cannot.

**Attribution is by digest.** A family and a revision record what somebody intended to install; only
the artifact's SHA-256 says what answered — with the template version beside it, since the same
weights under a different template are a different thing to have asked. A worker that loaded a
different file than the manifest named would otherwise produce answers attributed to a model that
never ran, and the surface a person uses to ask *which model told me this* would confidently give
the wrong answer.

And a request names the disclosure its input came from, in a field that is not optional. A model is
a named consumer under ADR-0030; a request that could omit it would be a way to hand a model context
nobody recorded handing it.

Twelve tests. What none of them can claim: no model has been loaded, no answer has been attributed,
and no `ModelOutput` has been produced by anything but a test. This is what a model would be
permitted to do. Whether the permission is enforced is a question for a runtime that does not exist.

## The rule that decides what a stranger sees was the one rule no test could run (2026-08-23)

```
perception → contribution → epistemic belief → redaction → what a reader receives
```

The second sweep, on the path that matters more. What can be lost here is not a hedge: it is a value
the person did not agree to publish. A sensitivity that survives perception, survives the Journal
envelope, survives belief derivation and is dropped at the redaction step produces a page that looks
exactly like a page with nothing to hide.

The decision lived inline inside the D-Bus adapter, so it could only run with a session bus and a
live Mind behind it — the same shape as every defect this codebase has actually shipped: arithmetic
welded to a component, a stylesheet nothing compiles, a daemon behind a `cfg`. And it is where the
one live disclosure bug came from: three thousand contributions reported as accounted for against
ten items supplied.

`gateway::redact` is that rule, extracted. The policy is one comparison of two numbers, deliberately
a free function rather than a method, so it cannot come to depend on the state of whoever applies
it. The `Ledger` beside it keeps the account: what crossed, what did not, what it came from, and the
two counts whose *difference* is the point — a concept carries no evidence, so it is supplied and
unaccountable, and a surface reporting one number could never show that. Seven unit tests, none of
which needs a bus.

### What the walkthrough asserts

Not that the filter returned the right count. That the **withheld value appears nowhere in the bytes
a stranger receives** — serialised whole, searched as text. A projection that leaked the value into a
field nobody thought to check passes a length assertion and fails this one.

Around it, the controls that keep the strong assertion honest: something *did* cross for the
stranger (otherwise proving a leak-free empty page proves nothing), and the person is told what
belongs to them (otherwise a redactor that dropped everything would satisfy every test and the
surface would be private and useless). Plus: a contradicting observation makes a private belief
disputed without downgrading its class, and an empty Mind does not produce the same account as a
fully redacted one — one supplied nothing because there was nothing, the other because everything
was kept.

Seven tests, all passing on the first run — which is only worth stating alongside the check that
they bite: with the comparison forced open, six of the seven fail.

### And then the growth it named (2026-08-23)

The provenance set a delivery accumulated was unbounded, and membership was tested by scanning a
list, so a wide delivery was quadratic in what it cited and wrote a set that grows with the
biography into a permanent contribution every time. This is the mechanism behind the three-thousand
figure, and the one unbounded growth anywhere in the system.

Bounding it alone would have been worse than leaving it: **a permanent record that silently omits**.
So `ContextDisclosedV1` carries `provenance_count` beside a bounded `items`, and a record says which
kind it is by arithmetic — where the count exceeds the length, it is a sample. No flag, and nothing
to keep in sync.

The count is `Option<u32>`, not `u32`. Records already in Journals do not have the field, and a zero
default would have turned every one of them into a record claiming it cited nothing, on the day the
field shipped. `None` says it cannot say. A test decodes a map built without the field — assembled
as CBOR rather than by re-encoding the struct, since re-encoding could only ever produce what this
build writes today — and holds that it reads as `None` and not as zero.

The `Ledger` keeps two structures for the same sources: a set that decides whether a contribution is
new, and a bounded list of what gets written. They answer different questions, and only one of them
may grow into a permanent record. The set is discarded when the delivery ends; only its size
outlives it.

## Walking the whole path found something no layer could see (2026-08-23)

```
utterance → act → activation → proposals → attention → plan → prose
```

Every joint of this existed and was tested on its own. That is exactly the condition under which
things get lost: each layer holds its own invariant, nobody holds the composition, and a hedge that
survives five boundaries and dies at the sixth looks — from the only place a person stands —
identical to a hedge that was never raised.

Walking it end to end found one immediately.

A budget cut the retrieval to a single concept. That one concept fits the attention quota
comfortably, so admission had nothing to refuse and reported itself complete — correctly, on its own
terms. The plan asked the admission whether the answer was whole, the admission said yes, and the
prose presented one concept as everything the word brings to mind. **From inside `workspaced` a
truncated activation offering one concept is indistinguishable from a graph that holds one concept.**
No test of any layer could have caught it, because no layer was wrong.

The fix is a second flag rather than a stronger one: `upstream_complete` beside `complete`. Kept
apart because they are different facts with different remedies — a quota turning proposals away is
attention being busy, a budget cutting a walk short is retrieval never having finished — and one
flag would report "there is more" without saying where the more is. The plan hedges on either, since
a reader acts on the same thing; which one it was stays on the admission for anyone who needs it.

Three things now provably travel the whole way, asserted at the far end rather than at the joint
that produced them: a dispute the epistemic owner set, the fact that a budget or a quota cut the
answer short, and why each concept came back at all. Plus a control that unhedged answers stay
unhedged — without it, a renderer that hedged everything would pass, and a hedge that is always
there is the same as no hedge.

Two more the walk makes checkable for the first time: an erasure reaches the sentence as *"nothing
is associated with lemon"* rather than as a sentence still naming an erased concept, and asking Mind
a question does not change what Mind was already attending to.

The vocabulary gained `Disputed` and `Superseded` as qualifications on the way. Folding disputed
into `Unverified` would have been the same class of loss: unverified is a check that has not
finished, disputed is a check that finished and came back contradictory, and a reader told the
weaker of the two would not know to go and look.

Held as a dev-dependency test rather than an organ. The daemons are separate processes over D-Bus;
this composes the libraries the way a caller composes the interfaces, and its subject is the loss,
not the transport.

### Then it went flaky, which found two more

A flaky test is normally a test to fix. This one was reporting.

**An unfinished search was claiming an empty world.** A wall clock cut a wide walk before its first
step. The activation honestly returned nothing, and the plan rendered that as *"Nothing is associated
with lemon"* — a claim about the world, made from a search that never ran. The `Partial` hedge was
there and did not help: **a hedge qualifies a claim, it does not withdraw one.** The claim itself had
to change, to *"the search did not finish, so nothing came back"*. Which also forced a distinction
worth having: a seed the graph does not hold is *not* a walk that was cut short. Nothing is
associated with bergamot is true; the walk that happened finished. `was_cut_short()` separates them.

**Two determinism gaps under one roof.** Overflowing the concept budget with equally salient
concepts left eviction to hash order, so the same sequence of activations produced a different graph
on every run of the process — including runs that evicted the concept being asked about. Fixing that
exposed the second immediately: `bundle()` sorted by salience alone, so even an identical graph came
back in a different order each run. A1 asks that one snapshot produce one bundle; the first fix is
what makes one *history* produce one snapshot, and the second is what makes one snapshot produce one
bundle. Both are a label tiebreak, and neither was visible to any single-layer test, because a set
that is correct-but-unordered passes every assertion about its contents.

## A dispute that does not survive retrieval was never held (2026-08-23)

ADR-0029 A4 asks that a disputed epistemic state still be disputed after retrieval. It held, in the
way the last two gates held: retrieval did not touch epistemic standing at all, so nothing could
drop it. A retrieval with no word for "disputed" cannot carry one — it hands back the value, and the
loss looks exactly like there having been nothing to lose.

`EpistemicStatus` moved into the protocol for that reason. A standing only the organ that derived it
can name is a standing that gets dropped at the first boundary it crosses. Naming it in the shared
vocabulary moves no authority: `epistemicd` remains the only thing that decides what a subject's
status is, and everyone else may only carry what it decided, unchanged. `Unknown` is the default,
deliberately — something arriving without a standing has not been established to be settled, and
treating silence as corroboration is how an unread projection comes to read as a healthy one.

A concept now carries its standing into activation, and out of activation into attention. Both
boundaries have a test, and the retrieval one was checked by breaking it: dropping the standing on
the walk fails three tests rather than none.

### The half an instinct gets wrong

A concept reached *through* a disputed one is **not** thereby disputed, and this is the opposite of
what `compose` does two layers away.

The rule there is that a qualification on any part qualifies the whole, because the parts were
claims about one answer and a reader cannot tell which half a hedge applied to. Here the walk is
association, not inference. `lemon` being contested says nothing about whether honey exists, and
propagating the dispute along the edge would be association conferring epistemic force — the precise
thing A5 forbids. So each concept carries its own standing and only its own, and the session can say
that *something* in it is qualified without pretending to know that everything is.

`Unknown` never overwrites a stated standing either: a caller that did not know is not evidence a
dispute went away. A stated standing does replace a stated standing, so the rule is not "disputes
are permanent" — the epistemic owner settling one is carried through the same way.

## A gate nothing evaluates is a comment (2026-08-23)

ADR-0032 defines a `PromotionGate` with three criteria — independent episodes, success rate, replay
evaluation — and nothing evaluated it. The thing it was written to stop, a pattern noticed once
becoming a rule Mind applies, happened exactly as it would have without it. `protocol::promotion`
now decides, and every answer is either a promotion carrying the numbers it was granted on or a
refusal naming which criterion was not met. There is no "probably ready".

**The unit of evidence is the episode, not the message.** One episode that produced three messages
is one demonstration. Counting messages would let a single lucky occasion satisfy "three independent
episodes" while nothing was ever repeated — and repeatability is the entire content of that
criterion. The same trap sits in the rate: an episode that went well ten times beside two that
failed is ten-twelfths successful by message and one-third by episode. So an episode counts as a
success only if everything observed in it succeeded, and the rate is over episodes. Both traps have
a test.

**Association is not promoted** (ADR-0029 A5). An associative candidate is refused with fifty
successful episodes behind it — not because the evidence is weak, but because the associative layer
is recomputed from the Journal, so a durable artifact promoted into it would be a durable claim on
something the next erasure epoch rebuilds from nothing. `contextd` may offer associations,
co-occurrence and activation paths as *inputs* to a candidate. Nothing about having offered them
makes the candidate promotable. Every other layer stays reachable by earning it — a blanket ban
would make the gate unpassable and look like caution.

## The rule that held because nothing had been wired yet (2026-08-23)

ADR-0029 fixes an order — Journal, epistemic, associative, attention, meaning — and one rule over
it: *each layer may read the one above it and may not overrule it.* That rule is what stops a memory
architecture from being decided by accident.

It held. It held because the wiring did not exist: `contextd` could not reach `epistemicd` because
`contextd` could not reach anything. That is not a rule holding, it is a rule untested, and it is the
same shape as A11 — which held right up until activation existed, and then needed a quota. The
pattern is worth naming: *a gate that passes because its subject is unimplemented will fail the first
time somebody implements the subject for a good reason.*

`scripts/validate-organ-layering.py` makes the edge itself the thing that fails, in CI. Three
inverted dependencies were injected to confirm it catches them; none of the real manifests trip it.
It checks manifests, and says plainly what it does not claim to see — reading upward is allowed, so
`contextd` naming `epistemicd`'s types would pass and should. What A5 forbids is `contextd` making
something *known*, and that direction is held by the promotion gate instead.

## Bounded in size is not bounded in attention (2026-08-23)

Activation can now return what a word brings to mind. The next question is what happens when it
returns two thousand of them, and that seam had nothing holding it.

The failure is easy to miss, because a naive workspace still looks correct while suffering it.
`accept` keeps the moment at capacity by dropping the oldest, so a flood of proposals leaves the
buffer exactly as bounded as it promised — and empty of everything that mattered. A workspace that
was answering a `NeedSignal` a moment ago and now holds thirty-two associations of the word *lemon*
has stayed inside every limit it declared and lost the only thing it was for.

ADR-0014's amendment already said the rule: *relevance discovered by associative retrieval is not
permission to displace the current focus.* `workspaced::admission` now enforces it.

**A proposal never evicts a resident.** Something that happened outranks something that came to
mind, whatever their scores. Structural rather than a threshold: no relevance is high enough,
because relevance is not the currency being spent.

**Proposals share a quota, not the capacity.** A quarter of the slots, at least one and never all.
Even an empty workspace does not become entirely associative, because a moment made only of what a
word suggested has nothing left to notice an interruption with.

**What was refused is counted.** Three admitted out of two thousand and three admitted out of three
look identical unless the difference is reported, and every proposal offered is accounted for
exactly once — admitted, over quota, unreached, or a duplicate of something already named.

`Workspace1.Consider` exposes it, and considering is deliberately not accepting: proposals reach the
workspace by a different door than contributions, and only one of those doors makes room.

Two tests at the seam. One offers five thousand associations to a workspace answering a `NeedSignal`
and holds that it is still answering it. The other is kept as an executable statement of why the two
paths must differ: the same flood through `accept` leaves the workspace bounded, as promised, and
the thing worth attending to gone.

## Asking a word what it brings to mind (2026-08-23)

`contextd` could say what was active and what was associated with what. It could not say what one
word brings to mind. Filtering every concept by salience is not that answer: it reports what is loud
right now, which is the same answer whatever you asked.

`ActivationSession` walks the associations from named seeds. Three properties make it worth having,
and each is a refusal.

**Bounded.** Nodes, edges, depth, time and tokens each stop the walk by themselves, and the session
names which one did. "The budget ran out" would leave an operator unable to tell a graph that is too
wide from one that is too deep from a machine that is too slow, and those need different responses.

**Inspectable.** Every reached concept carries the path it came along and the links on it — `lemon →
honey`, episodic, strength 0.84, depth 1. Relevance is the product of those strengths, chosen
because it is the one number a person reading the path can arrive at themselves. A richer score
blending freshness and personal relevance would very likely rank better and could not be accounted
for by what it came from. Nothing is ever asked to compose a reason: a generated explanation of a
retrieval is not evidence about that retrieval.

**Partial says so.** A walk cut short is not complete and says what cut it. A seed the graph does not
hold is reported too, because "nothing came back" and "nothing is associated with it" are different
answers and only one of them is true. And a walk that ended exactly at its depth limit with nothing
further to reach is *not* called truncated — reporting a complete answer as a cut one is the same
lie in the other direction.

### Where the clock is allowed to act

A1 wants determinism; A2 wants a wall clock able to stop the walk. Taken naively they contradict —
a slower machine would return a different bundle.

They reconcile at the point the clock touches. Expansion order is fixed entirely by the graph
(strongest first, ties by label), so one canonical sequence exists per snapshot and seeds. The time
budget may only **truncate a prefix** of that sequence; it can never reorder it or admit something a
patient run would not have reached. A hurried bundle is therefore always a prefix of the unhurried
one, and it says it was cut. A test holds exactly that.

Reachable, not just correct: `ContextCore::bring_to_mind` walks the graph the organ actually holds
under a real clock, and `org.cybou.Mind.Context1.BringToMind` exposes it. An erasure invalidates the
projection, so after one the walk finds the graph gone rather than an erased concept still
reachable.

Twenty-one tests in the organ.

## A conversation remembers what was named, never what it was about (2026-08-23)

With no memory between turns, "it" in the second sentence points at nothing and a person restates
the subject every time — which is not a conversation, it is a series of unrelated commands. The
obvious repair is to remember what was being talked about and let the next pronoun mean that. The
obvious repair is also a machine for guessing, and ADR-0031 C2 exists to stop exactly that guess.

`meaning::Dialogue` remembers **referents, not a topic**. What was named in the recent past is
offered to the resolver as one more candidate beside whatever the caller already had. Memory can
therefore make an ambiguity visible — two things were named, so now "it" could be either — and it
has no way to make one disappear. There is deliberately no accessor for "the current subject",
because a type that could answer that question would be asked it, and answering it is the guess.
A bare pronoun still resolves to nothing; what memory changed is that the clarifying question can
now name the choices.

Three bounds, each for a different failure. Turns, because a referent from twenty exchanges ago is
not what "it" means and offering it makes every pronoun ambiguous forever. Time, because a
conversation resumed the next morning is a new conversation whatever the turn counter says. And
erasure: what ADR-0028 erased leaves here in the same act, since a referent left behind would let
the system offer, by name, a thing a person had already had removed — and offering it by name is
the disclosure the erasure was for. A single turn that names two hundred files does not become a
two-hundred-item candidate list either; a list that long is not a clarification anyone can answer.

Ten tests. M8 is complete.

## Plans compose, and a hedge on one part holds for the whole (2026-08-23)

One question often needs two answers, built by different parts of Mind. The tempting way to join
them is to concatenate the prose, and that is exactly where the loss happens: two sentences run
together keep the claims and quietly shed the hedges, because a hedge reads as belonging to the
sentence beside it rather than to the answer.

`meaning::compose` joins plans instead, where the hedges are typed. The rule is one line:
**a qualification on any part qualifies the whole.** Joining an answer read from a stale projection
with one that was not produces an answer that is stale, because a reader cannot tell which half a
hedge applied to and should not have to.

Two compositions are refused rather than fudged. Plans built for different intents are not one plan
— picking one intent would present half the claims under a purpose their author did not have. And an
empty composition is refused, because the absence of an answer is not an answer that says nothing,
and returning an empty plan would let a caller present "no parts" as "nothing to report".

Everything else is a union that states each thing once: a contribution cited by two parts is one
contribution, identical claim text is one claim, and the order a plan put its claims in survives.
Eight tests hold it.

Still missing from M8: dialogue state across turns.

## A plan now exists to be realized from (2026-08-23)

ADR-0031 puts a `ResponsePlan` between typed state and anything a person reads, and the realizer has
been honouring that boundary for some time: it is handed a plan and nothing else, so a fluent
sentence cannot quietly acquire a claim Mind never made. What was missing was the other side.
Nothing built a plan. `Realize` was reachable and unused, and C5 — *a plan expresses claims,
evidence references and qualifications before language realization* — had no assertion behind it,
because there was no plan for an assertion to be about.

Two things changed. `ResponsePlan` gained `qualifications`, a closed set: `not-read`, `stale`,
`partial`, `withheld`, `unverified`. And `meaning::plan_status` builds a plan from typed capability
facts.

The point is not the wording. It is that **the hedges are decided in the typed layer**. A capability
nobody could read, a projection outside its freshness, a listing cut short by a bound — each becomes
a qualification on the plan, where the realizer cannot lose it. Left to whoever writes the sentence,
"eleven of eleven capabilities are available" and "eleven of eleven, as far as I could read" would
be the same function of the same state, chosen by tone.

The planner reaches nothing, which is the realizer's discipline one layer earlier: it is a function
of the facts it is handed, down to the plan's identity being supplied rather than generated. A test
asserts that the same facts produce the same plan, so anything it reached for would fail rather than
be described as absent in a comment.

Two properties now have assertions. A projection that was never read does not become "0 of 0
capabilities available" — it becomes a `not-read` qualification and a sentence saying so. And a
qualification carried by a plan reaches the reader in both languages, because a plan that hedged
beside prose that did not would put the confident reading in front of the person while the honest
one stayed in a struct.

Still missing from M8, unchanged: composition operators, and dialogue state across turns.

## An erasure says what it reached, not that it is finished (2026-08-23)

`Event1.RequestErasure` destroys the key, redacts the payload of the target and everything derived
from it, advances the epoch and records that it happened. All of that reaches the live database and
every future backup. None of it reaches a copy already taken: a backup made before the erasure, plus
a recovery root that still unwraps the key captured in it, defeats the erasure for that record.
ADR-0028 says so plainly (E11, E12) and the code said nothing at all — so the surface reported the
reassuring reading without justifying it.

`BackupState` is the typed answer the ADR asks for, and the terminal `ErasureApplied` record now
carries it:

| state | means |
|---|---|
| `no-backups-declared` | this deployment states it keeps none, so nothing outside the database holds a copy |
| `pending-rotation` | a copy predating the erasure may still be in rotation; the record names the instant that ends |
| `complete` | every copy that predated the erasure has left the declared rotation |
| `unknown` | this deployment has not said, so nothing can be claimed |

`unknown` is the default and deliberately the unreassuring one. Silence about backups is not
evidence that none exist, and an erasure reporting completeness because nobody mentioned a copy
would be stating what nobody established. A deployment declares its rotation in the unit —
`CYBOU_BACKUP_ROTATION_DAYS`, where `0` means none — and this one declares zero.

Only the terminal step carries a state. A request that has not been carried out has achieved
nothing, and saying anything about backups there would be a claim about work not done.

One thing the tests caught rather than the design: a rotation long enough to overflow the instant
arithmetic came out the other side as `complete`. A sum that did not fit is a calculation that could
not be made, not a window that has passed, and turning "we cannot say" into "it is done" is the one
direction this type exists to prevent. Six tests hold the states, including that one.

What this is still not: backup software. Nothing in this tree takes a backup. What changed is that
the erasure no longer implies it reached copies nobody has described — the deployment states what it
keeps, and the erasure reports the consequence.

## The bottom of the screen has an order (2026-08-22)

The command bar floated in the middle of the canvas, over the cards. It and the palette were both
`position: absolute` with no positioned ancestor, so they resolved against the document rather than
the window — and because the palette section is itself a containing block once fixed, the bar's
`bottom` was measured from the palette instead of the screen.

One anchored container now, two children: the section is fixed to the window, the menu is a box that
appears above the bar, and the bar sits in normal flow inside the section. The three bottom surfaces
have a stated order — dock, command bar above it, menu above that — and the viewport controls sit
clear of all of them.

Measured rather than eyeballed, at 1440x900 and at 760x700: no pair of them overlaps, and nothing
crosses the edge of the window at either size.

## Instants are shown the way a person reads them (2026-08-22)

The owners record time as RFC 3339 with whatever precision they had —
`2026-08-22T16:33:54.409963793Z`. That is the right thing to store and the wrong thing to put on a
card. The topbar stopped printing them earlier the same day; the cards did not, so the desktop was
still covered in nine subsecond digits and a pair of format characters, which is what a surface
built for whoever wrote it looks like.

`instant::instant_label` renders `2026-08-22 16:33:54 UTC`, and the exact string stays reachable
beside every display. Nothing is rounded away: what is dropped is the part nobody was reading. An
input the formatter does not understand is returned untouched rather than reshaped — a formatter
that invented a value for something it could not parse would be the failure it exists to stop.

It lives outside `components`, so it is tested natively: five tests, including that a non-UTC offset
is not labelled UTC and that an unparseable string comes back as it went in.

## The desktop was drawing what nobody had styled (2026-08-22)

Sixty-five classes the components render had no rule in the stylesheet at all, including every
element of the topbar. An unstyled element does not look wrong in an obvious way — it falls into the
document flow — so the controls stacked vertically under the logo and drew themselves over the
canvas, and the status text ran together into one word. Meanwhile the stylesheet still carried rules
for a previous generation of components that nothing renders: a telemetry card, a web browser card,
an intent launcher. Neither half was visible to the compiler, to `cargo test`, or to the browser
gate, because CSS is not code to any of them.

`scripts/validate-desktop-styles.py` now fails if a class a component renders has no rule, and runs
in CI. One direction only: "rendered but unstyled" is exact, while "a rule nothing renders" is not
decidable from source, and a check that guesses gets ignored.

Four faults behind the same wall were fixed with it:

- The topbar is a grid of **three** columns, because it has three children. Two put the actions on a
  second row. The narrow-screen rule had the same fault; what gives way there is the status detail,
  not the controls.
- Cards carried a padding each — 19, 20, 22, 24 pixels — with nothing recording why. On a desktop
  where every card is the same kind of object, twelve arbitrary geometries is twelve small
  surprises. One shape now; the per-kind class stays as an identity hook and does nothing to how a
  card looks.
- Arrangements were called with `None` for the viewport, which meant a hardcoded 1440x900 whatever
  the window was. On a maximised screen the cards were laid out for a smaller desktop than the one
  they were on, and "Fit All" then shrank the result to around 61% — which a person saw as the
  arrangement changing the zoom by itself. Every arrangement is told the real size now.
- Columns stepped by a constant while cards range from 220 to 560 wide, so wide cards ran into the
  next column and the last column could start past the edge of the window. A column is as wide as
  its widest member.

Focus was scaled by the canvas. A `position: fixed` element inside a transformed ancestor is
positioned against that ancestor and scaled with it, so a focused card meant to fill the window was
drawn at the canvas zoom and offset by the pan — a large empty frame with its contents small in one
corner. The canvas drops its transform while anything is focused, so focus means the same thing at
every zoom.

And the Shell and the File Manager could not read anything on the deployed host. Nothing ever set
`CYBOU_SHELL_JAIL`, so the gateway chose its sandbox from a list of candidate paths and picked
`/home/demo` because that directory happens to exist there — owned by somebody else and unreadable
by the service. Every `ls` answered with an I/O error and every listing was a 502. The unit names the
sandbox now, and the gateway guesses nothing: a sandbox chosen by what happens to be on disk is a
sandbox nobody chose.

## The half of the desktop no test could see (2026-08-22)

Everything under `components` is `cfg(target_arch = "wasm32")`. `cargo test --workspace` compiles
none of it, and that is where three of the day's faults lived: selection comparing a kind key,
collapse destroying a terminal session, and a minimap drawing docked cards through stylesheet rules
that did not exist. Each was found by looking, and each could have gone on indefinitely.

`src/interaction_gate.rs` runs under `wasm-bindgen-test` in a headless Chromium. It mounts real
components against real signals and asserts on the DOM a person would have seen: that clicking one
Shell card selects that card and no other, that collapsing a card and expanding it returns the
history that was in it, that two Tool cards of one kind keep separate state, that closing one
releases it, and that a card docked into a deck stops being drawn standing on its own.

The gate was checked against the fault it was written for. Restoring the key comparison in
`CardFrame` makes `clicking_one_shell_card_does_not_select_the_others` report `[true, true]` where
`[false, true]` is expected — the original bug, named exactly. A test that has never failed for the
right reason is a test nobody has checked.

It runs in CI on every push as the `desktop` job, alongside the workspace and the multi-daemon
gates. The rule it exists to enforce is cheaper than the gate itself: arithmetic over the layout
belongs in `layout/`, where it is tested natively, and components should only draw.
`layout::selection` and `layout::minimap` moved there for that reason.

## Selection names an item, not a kind (2026-08-22)

The desktop learned to hold several Shell cards and several File Managers, and one place did not
learn with it. Selection was a `&'static str` holding a card's key — and a key names a *kind*:
`Shell(0)`, `Shell(1)` and `Shell(2)` all answer `"shell"`. So clicking one Shell card marked every
Shell card selected, and the action attached to the selection resolved that key back through
`CardId::from_key`, which answers `Shell(0)`: clicking the third Shell brought the first one
forward.

Selection is a `DesktopItemId` now — what the layout has always used to tell one thing on the
desktop from another, decks included, so a deck can be selected in its own right. `key()` stays what
it always was: a name for a kind, good for CSS and routing, never an identity.

The arithmetic moved to `layout::selection`, where it can be tested without a browser. Six tests
hold it, including the one that would have caught this: two Shell cards at different coordinates
must not resolve to the same place. It was invisible to every existing test because the whole of it
lived in wasm-only component code.

Two smaller things went with it. A deck created by dropping one card onto another started from a
constant `420 x 480` and only ever grew, so a merge could double the footprint of what a person had
just arranged; it takes the place of the card it replaced, expanded only where a member's own
minimum requires. And the global keyboard listener was installed with `forget()`; it is removed on
cleanup now. A listener nobody can take off keeps answering after whatever installed it is gone.

## A stranger is served nothing, and the page says less (2026-08-22)

The deployed surface ran in `PublicPreview`: a filtered projection of a live Mind, served to anyone
who had the address. Filtering is not the same as not showing, and the person the projection is
about had agreed to neither. `SessionMode::SignInRequired` is now the default whenever a deployment
can authenticate anybody, and it is what the deployed unit sets. `PublicPreview` still means what it
meant — a surface deliberately opened — but it has to be asked for rather than being what happens
when nobody chooses.

The refusal is at the gateway. `/api/v1/snapshot`, `/mind`, `/events` and `/disclosure` answer `401`
without a session; only the routes that establish one stay open, because a gate that refused those
would be a locked door with no handle. Hiding the cards in the page would not have been a boundary:
every one of those routes is reachable with `curl`.

Reading the session is now the first and separate thing the frontend does. Asking for a snapshot
anyway turned a closed door into a connection error, and the page then drew a whole desktop of
em-dashes — telling a stranger the machine was broken while showing them the entire structure of the
Mind. `RuntimeState::SignInRequired` is its own state for that reason: nothing is wrong, nothing is
being shown, and those are different.

Separately, the page stopped saying things only its authors could read. The brand said
`Mind · Body · Living Canvas`; the header printed `Projection v42 · Cursor fixture:presence:42 ·
Expires 2026-08-22T23:06:59.51365708Z` across the top of the screen, which told everybody who opened
it that it was not for them. The mode a person is in stays visible in words; the plumbing moved to a
tooltip, because removing it from sight is not the same as removing it. The menu is labelled `Menu`
rather than `Mind`, sign-in is `Sign in` rather than `Authenticate`, and `Zone 3 Body` and
`Zone 3 Storage` are gone from every card that showed them. The organ names in card headers stayed:
`Identity1` on the Identity card is the claim that the page composed nothing.

The viewport chrome was not anchored to anything. `.canvas-controls`, `.zoom-controls` and
`.canvas-btn` had no stylesheet rules at all — the same fault the minimap had — so the zoom controls
sat wherever document flow left them. They are fixed to the window now. And the floating action
beside a card was positioned from `Capabilities`'s geometry whatever a person had actually selected,
so it sat under a card nobody had chosen and acted on one they had; it follows the selection.

## A Debian-native desktop session exists, and what is proven about it is narrow (2026-08-22)

`scripts/cybou-desktop-session.sh` and `systemd/user/cybou-desktop.service` are the smallest thing
that can be called a session: Cage owns the display and shows one window, Chromium draws Living
Canvas from the loopback gateway. There is no panel, no launcher and no second application, because
the desktop is inside the surface rather than around it. Cybou wrote no compositor and no shell.

The unit is installed by a deployment and **not enabled**. This project's deployed host has no seat
and no display; a target that dragged a compositor onto it would fail at something it does not
need. A person who wants the session enables it on a machine that has one.

The Chromium profile is durable, under `$XDG_STATE_HOME/cybou/desktop/chromium`. The layout lives in
that profile's `localStorage`, so an ephemeral profile — which is what the removed Plasma-era
preview used — meant a desktop that forgot where every card was put, on every start. It is state and
not biography: where a person likes their windows belongs under `XDG_STATE_HOME` and never in the
Journal.

**What is proven.** The launcher refuses rather than starting a browser when the gateway does not
answer, so a late gateway does not become a desktop showing a browser error page; it exits non-zero
when no Chromium binary can be found, including when one is named explicitly and is not there; it
creates the profile directory under `XDG_STATE_HOME`; and the argument vector it would run is
printable without running it, so it can be checked without a compositor. Those four were exercised
on Debian 13.

**What is not proven.** That Cage acquires a seat, opens a display and shows that Chromium window on
real hardware. It could not be exercised here: the only Linux machine available is WSL, whose
compositor mounts `/tmp/.X11-unix` read-only, and Cage 0.2.0 always starts XWayland and has no
option to decline. Nothing about the desktop session should be read as evidence until it has run on
a machine with a seat. `README.md` continues to call the desktop a target.

## Focus is recorded in one place (2026-08-22)

`CardPresentation` persisted a `maximized` flag. Nothing set it and nothing read it: focus is
`DesktopViewMode::Focus`, which fills the viewport without touching the geometry underneath and
restores the desktop on `Escape`. Two fields that could each answer "is this card filling the
screen?" is one too many, and the one being written to disk was the one that never knew.

It is removed. A layout saved while the flag existed still loads, because unknown fields are ignored
on the way in — dropping a field is only safe when its absence and its presence both parse, and a
person's saved desktop is not something to discard over a value that never meant anything.

## The minimap shows the desktop it is on, and where you are in it (2026-08-22)

It drew `layout.cards`, which is not the set of things on the desktop. A card docked into a deck
stays in `cards`, so the map drew it standing alone at coordinates it had left, and never drew the
deck it was actually inside. `desktop_items()` is the set Invariant L8 defines, and it is what the
map draws now: one item per top-level thing, a deck as one item rather than as its contents
scattered.

Every coordinate was divided by a constant `1280 x 650`. That is a projection of one particular
desktop: a card dragged past those numbers was drawn outside the surface and disappeared. The
transform is derived from `bounding_rect()` now, so whatever the layout is, it fits — with one scale
for both axes, because a map that stretched one of them would draw a wide desktop as a square one.

There was no viewport rectangle at all. The map showed where the cards were and never where the
person was, which is the one thing an overview is for. The rectangle is derived from the canvas
transform — `translate(pan) scale(zoom)` inverted — and moves as the desktop is panned or zoomed.

The projection is ordinary geometry over `Rect`, so it lives in `layout::minimap` and is tested
natively rather than only by looking: nine tests hold that a desktop of any size and origin lands
inside the surface, that shape survives, that nothing is drawn too small to see, and that centring a
card puts it in the middle of the screen. The classes the old component used had no stylesheet rules
at all, which is its own answer to how much of this was working.

## The relationship graph decides the arrangement, and one edge ran backwards (2026-08-22)

`DesktopRelationshipGraph::canonical()` was already the single source for the lines drawn between
cards. It was not the source for where the cards went: the causal layers were a hand-kept table, and
it had drifted away from the edges it was meant to summarise. `Identity` proved `Session` while both
sat in layer 0. `Capabilities` audited `Journal` from inside layer 1. `Beliefs` reached `Perception`
backwards through two layers. The graph drew one story and the arrangement laid out another.

A card's layer is now the longest path into it, computed from the edges. Every edge therefore points
forward by at least one column, and the arrangement can be read as the causality it came from. The
number of columns comes from the graph too, rather than a constant five that silently folded
anything deeper into the last one. Tool cards are not in the graph and sit after every organ,
because they consume what the organs produce and feed none of it back.

One edge was reversed. `Beliefs -> Perception` was labelled "empirical observation updating
propositions", which describes the opposite direction, and the wiring this repository documents runs
that way: `perceptiond` observes the host and submits to Event1, and `epistemicd` forms beliefs
keyed by the subject of each observation. Nothing flows back. It is `Perception -> Beliefs` now.
While these edges only drew a line the error was cosmetic; the moment they decide placement, a
reversed edge is a reversed desktop.

`Disclosure` gained the three edges it actually has — `Intention1`, `Epistemic1` and `Context1` are
the organs the gateway's disclosure bookkeeping counts, and no others contribute to a
`ContextDisclosed`. It is placed after them rather than by an exception in a table.

Seven tests hold the properties rather than the numbers: the graph is acyclic, every edge advances
at least one layer, an organ nothing feeds starts at the beginning, what leaves is placed after
everything that produced it, and a tool card sits last.

## The File Manager reads the sandbox instead of reading a terminal (2026-08-22)

The File Manager asked the Shell for `ls -la` and parsed the columns back into names, kinds and
sizes. A typed filesystem turned into text and then guessed at — and it failed the way such loops
fail. The parser wanted nine whitespace-separated fields; the engine's long format produced six; so
every entry fell through both branches, and the panel reported an empty directory. Nothing raised an
error, because from the parser's point of view there was nothing there. The bug was invisible
precisely because both halves were working as written.

`POST /api/v1/files/list` and `POST /api/v1/files/read` hand back what `cybou-jailfs` already
established: name, kind, size, contents. There is deliberately no mode and no owner, because the
sandbox does not read them and a surface that showed them would be showing a constant. A listing
that hit its bound reports the total it was cut from, so a bounded answer is never mistaken for a
small directory. A path that left the sandbox is answered exactly as a path that does not exist:
distinguishing them would let a caller entitled to read inside the sandbox map its edge by watching
which refusals differ.

Both routes share the Shell's entitlement boundary and refuse a public reader, and neither goes
through the Shell. The stopgap shell number the File Manager was borrowing is gone with the parser.

Two things surfaced while replacing it. The panel opened on the words "Empty directory" before it
had asked anything — an assertion about a directory it had never read, and a person had to press
Refresh to find out whether the first screen had been true. It now reads on first mount, and while
it has not read, it says so. And an empty list is no longer the answer to a failed read: a directory
nobody could open reports why, rather than reporting that it is empty.

## A card's identity survived composition; what it had done did not (2026-08-22)

Invariant L8 says a Card keeps its identity through grouping, and the layout model always held that.
What did not survive was everything the card had *done*: a Shell's history, the command someone was
halfway through recalling, the directory the File Manager was looking at. All of it was created with
`signal(...)` inside the component, owned by that component's reactive owner, and destroyed the
moment the component unmounted.

Three ordinary actions unmounted it, and the smallest one is the worst. Docking a card into a deck
or pulling it out, because standalone and docked are different subtrees. Switching a deck tab,
because the deck body renders the active card and nothing else. And simply **collapsing the card**,
because `CardFrame` wraps its body in a `Show` — so a person tidying their desktop was silently
erasing a terminal session, with no warning and nothing to undo.

Tool card state now lives in `tool_state::ToolCardStates`, created under the root owner and looked
up by `CardId`. A component that mounts finds what its card already had; a component that unmounts
takes nothing with it. Node references stay component-local, because those really do belong to one
mount.

Closing the card is the one action that is a person saying they are finished, and it releases the
state. That exposed a second gap: the frontend forgot, and the gateway did not. A reopened Shell
showed `cybou:/ ›` while the engine behind it was still standing in `/somewhere`, so the first
command jumped the prompt somewhere the surface had already said it was not. `POST
/api/v1/shell/close` ends that shell, and closing the card calls it.

Verified in a browser against a local gateway rather than only by reading: after two commands the
prompt reads `/somewhere` with three entries; collapsing unmounts the body entirely; expanding
restores the same three entries and the same prompt; closing and reopening gives one entry and `/`,
and `pwd` agrees.

## A Tool card is an instance, not a kind (2026-08-22)

`CardSpec` has said `singleton: false` for Shell, File Manager and the event stream since ADR-0040.
Nothing else in the system agreed. The gateway kept one shell per seat, so two Shell cards in one
session were two views of one working directory. The frontend sent no card identity at all with a
command, so the gateway could not have told them apart if it wanted to. And the viewport rendered
exactly one `<ShellCard>`, so a second Shell opened into the layout model would never have appeared
on the desktop. Three layers, one unkept promise each.

`ShellExecRequest` now carries the instance the card belongs to, `ShellOwner` names a seat *and* an
instance, and the viewport renders one card per instance the layout holds rather than one per kind.
Signing out ends every shell the session opened, not the one numbered zero — a person who opened
three and signed out was leaving two working directories behind.

The File Manager reads through a shell number no card uses, so browsing files no longer moves the
working directory of a shell somebody is typing in. That is a stopgap and is written down as one:
it should be reading a typed filesystem capability rather than parsing `ls -la` out of a terminal,
and until it does, the least it can do is not stand in someone else's shell.

## The disclosure inspector (2026-08-22)

Every supply of a projection across a boundary already wrote a `ContextDisclosed` naming the
consumer, the contributions the supplied items came from, and what was held back and why. The
records existed and the person they were about could not read them: they were in the Journal, and
reaching them meant `busctl`. Transparency legible only to a developer is transparency for the
wrong person.

`GET /api/v1/disclosure` and the `Disclosure` system card close that. Both answer for the caller and
nobody else — the record is keyed by consumer, so a reader sees their own deliveries rather than a
log of what was done to everyone. What they show is the gap rather than the total: how much was
supplied against how much of it names the contribution it came from, and every refusal with its
reason in the frozen vocabulary. Where a withheld item could not even be named without saying too
much, it is still listed and still counted, because an unnamed refusal is a smaller loss than a
silent one.

One hazard was designed out before the surface shipped, because building it created it. The subject
of a withheld item is what makes the refusal answerable — but a concept is refused *by its label*,
so serving that label to a stranger to explain the refusal would have published exactly what the
refusal withheld. Subjects are named only to a consumer whose trust is `Owner`. A public reader is
told how much was refused and on what grounds, which are facts about the system rather than about
the person, and the projection carries `subjectsVisible` so the card can say why the subjects are
absent instead of leaving a reader to infer it from blanks. A surface that reports a filter must not
be a way around it.

Three states are kept apart that a summary would collapse: the record could not be read, nothing has
been supplied yet, and something was supplied. The middle one is the honest answer on a gateway
nobody has read from, and reporting it as an empty delivery would be a claim about a delivery that
never happened.

Building it found the failure it was built to expose. The gateway remembered a delivery only if it
could also write it to the Journal — the durable write was a precondition for the bookkeeping — so a
deployment with no sink answered that nothing had been supplied while it was supplying things.
Remembering and recording are now separate operations in that order. Having nowhere durable to write
a delivery is a reason to say the audit trail is incomplete; it is never a reason to answer as
though the delivery did not happen.

Deploying it found a second thing, which is the point of building surfaces rather than reasoning
about records. The first live read answered `supplied: 10, accountedFor: 3011` — more accounted for
than were supplied, which is not a number, it is a category error. `ContextDisclosed.items` is the
set of *distinct contributions* the supplied items were derived from, and one belief cites hundreds
of them; its length was never a count of items, though the field's own documentation had claimed
that relationship since the record was defined. The count of items that can name where they came
from is now kept separately and is the number `supplied` is read against, the total size of the
provenance set is reported as its own figure, and the identifiers themselves are a bounded sample —
the full set was a hundred kilobytes served to anyone who asked and unreadable by the person it was
for.

What this is not: a history. The surface shows the last delivery to this consumer, not the sequence
of them. A person can see what they were supplied; they cannot yet see what they were supplied last
week.

Status date: 2026-08-22.

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
Lifecycle1 process restart, and multi-process integration across the fourteen Mind owners. Both counts
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

P6.8 closes the substrate findings recorded in the [Implementation Audit](history/CODE_AUDIT_2026-08-10.md).
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

## The meaning boundary

`cybou-meaningd` is the thirteenth process and the owner ADR-0031 asks for. It turns an utterance
into a typed `CognitiveAct` and refuses to turn anything else into one: an utterance outside the
vocabulary this build recognises produces no interpretation rather than a guess, and a reference it
cannot settle stays unresolved rather than resolving to whichever candidate happened to score
highest. Nothing in it is a generative model, and the two refusals are why it does not need one.

An interpretation enters the biography as a pair. What the person said is an `Observation`, because
it happened outside the Journal; what this organ took it to mean is a `Hypothesis` caused by that
observation. No new contribution kind was needed, and a meaning layer that had required one would
have been claiming something the rest of Mind has no way to reason about. It also means an act
stays inspectable after the interpreter that produced it is stopped: the act is a row, beside the
sentence it came from.

A correction names the interpretation it disagrees with as evidence and is itself caused by its own
sentence. The earlier reading stays exactly as recorded. A correction of an act the Journal does not
hold is refused before it reaches Event1, so an ordinary rejection stays distinguishable from an
unreachable Journal.

Prose comes from a `ResponsePlan` and from nothing else: the realizer takes the plan and a language
and has no other input, so a fluent sentence cannot acquire a claim Mind never made. Two languages
render the same plan today.

What is not there: composition operators (`Sequence`, `Conditional`, `Alternative`, `Constraint`,
`Negation`), dialogue state across turns, and a planner — nothing yet builds a `ResponsePlan` from
Mind state, so `Realize` is reachable and unused. ADR-0031's C5 is therefore not met, and the
milestone is partial rather than complete.

## Process topology

Mind has fourteen real user-session processes:

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
cybou-meaningd
cybou-telemetryd
```

`cybou-shelld` was counted here until 2026-08-23 and should not have been: it is a library with no
binary, its unit says so in its own header, and it is left out of the deploy and out of
`cybou-mind.target` on purpose. Counting a unit that describes a process which does not exist is the
same error as a projection stating what nobody established — in the one document a reader consults
to find out what is running.

It went unnoticed because the count was derived from a glob over unit files, which is the right
instinct and the wrong measurement: a unit is a description, and what makes something a Mind owner is
that it owns a `Mind1` interface. The validator now reads `BusName` instead, which is the same
declaration the bus itself uses, so nothing is trusted twice.

Beside them, and deliberately not among them:

```text
cybou-model-brokerd    org.cybou.Faculty.ModelBroker1
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
