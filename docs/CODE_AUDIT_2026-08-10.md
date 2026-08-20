<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Implementation Audit — 2026-08-10

## Audit identity

| Field | Value |
|---|---|
| Audited commit | `108f0304` |
| Method | source reading of the Mind C++ tree, Nix modules, and CI workflow |
| Relationship to the checkpoint | complements the 2026-08-10 project checkpoint (retired; see [Current State](CURRENT_STATE.md)) |
| Scope | implementation evidence only; no new capability was assessed |

The checkpoint records what the architecture claims and how mature each area is. This document
records where the shipped implementation does not yet support a claim, with the specific source
location for each finding. Where the two disagree, this document is the more specific evidence and
the checkpoint's maturity scores should be read as adjusted by the "Effect on the checkpoint"
section below.

Findings are ordered by how directly they contradict a stated invariant, not by effort.

## A1 — Journal acceptance is not durable across power loss

**Where:** ``Journal.cpp`` constructor, `PRAGMA
synchronous=NORMAL` alongside `journal_mode=WAL`; acceptance is published from `Journal::append`
after `COMMIT`.

**Claim under test:** "Durable before visible" — Mind Model invariant 1, and the checkpoint's
statement that a UI refresh cannot become evidence that a fact was durably accepted.

**What the code does:** in WAL mode with `synchronous=NORMAL`, SQLite does not fsync the write-ahead
log at commit. `COMMIT` returns, `Event1 Accepted` is emitted, Workspace admits the contribution and
Presence projects it — while the bytes may still be in the operating system page cache. A kernel
panic or power loss can therefore drop a contribution that Presence already displayed as accepted.

**Why the existing evidence does not catch it:** the fault matrix exercises process termination and
clean reboot. Process termination leaves page-cache contents intact, and a clean reboot flushes
them. Neither scenario can expose an unsynced commit. The gap is structural in the test design, not
an oversight in any individual test.

**Resolution:** either raise durability to `synchronous=FULL` and accept the write-latency cost, or
restate the invariant as durability to the operating system rather than to storage. The first is
correct for personal biography that cannot be regenerated; the invariant should not be weakened
silently.

## A2 — The resilient RPC stack is not used on the presentation path

**Where:** ``RpcClient.cpp`` `RpcClient::call` uses
`QDBusPendingCall::waitForFinished()`. ``PresenceService.cpp``
performs its compound reads and mutations through that client.

**Claim under test:** the checkpoint's "RPC resilience: 3" and "responsive async paths".

**What the code does:** the project has two RPC stacks. `AsyncRpcClient`
(``RpcResilience.cpp``) implements typed outcomes,
retry with deterministic jitter, and a circuit breaker, and is covered by `tst_rpc_resilience`. The
Presence path does not use it: every downstream call blocks the presenced main thread.

P6.7 correctly bounded the *total* latency of a compound operation to one monotonic budget, so an
unresponsive owner no longer multiplies timeouts. It did not make presenced concurrent. While
presenced is inside `snapshotMap`, it is a single-threaded D-Bus service that cannot answer any
other caller — the Plasma applet, a second surface, or healthd.

**Correction — the claim originally made here about healthd was false.** This section first stated
that healthd probes its endpoints with the same synchronous client, and that a Presence aggregation
and a health refresh therefore stall each other. That is not what the code does.
``HealthService::Refresh`` uses `AsyncRpcClient`
exclusively: it issues every probe concurrently with a 750 ms per-probe timeout under a 2 s refresh
deadline, driven by a nested event loop that keeps serving healthd's own callers. healthd was
already doing what A2 asks of presenced. The error came from reading `endpoints()` and the probe
call sites without reading the dispatch around them.

What the flaky test runs actually showed was different and benign: `Refresh` returns `false` while a
refresh is already in progress, by an explicit re-entrancy guard. Suspending an owner makes a
refresh occupy its full deadline, so a test calling `Refresh` in that window is refused — correct
behavior being misread as unresponsiveness.

One consequence is worth keeping. Because the probe clients are owned by the refresh and destroyed
when it returns, their pending watchers are destroyed with them, so a callback that misses the
deadline cannot fire against a dead stack frame. That is the hazard a nested event loop usually
carries, and it is already handled.

**Resolution: applied for the projection path.** `Snapshot` is now a delayed-reply D-Bus method.
Health1 remains the one sequential step, because its answer decides which reads are legitimate at
all; every read it gates then goes out together through `AsyncRpcClient` and the reply is sent when
the last lands or the shared guard expires. presenced no longer blocks its own event loop, so the
cost of a projection is the slowest owner rather than the sum, and other callers are served while a
gather is in flight.

Two defects surfaced while building it, both worth recording because neither was visible from
reading:

- **Synchronous completion.** `AsyncRpcClient` can complete without reaching the bus — an open
  circuit rejects immediately. The first such completion drove the outstanding count to zero while
  later reads were still being issued, and the request replied with everything after that point
  missing. The issuing loop now holds its own reference until every call is dispatched.
- **The circuit breaker was wrong for this path.** A latched circuit outlives the request that
  opened it, so one transient stall blanked that owner's section of *every* projection for the
  next five seconds. Because the projection reports typed defaults for what it could not read, the
  UI would render that as "nothing there" rather than "not asked" — a transient fault converted
  into a sticky lie. The projection clients now use a single attempt and no latching. Resilience
  here is the shared deadline and the typed empty projection; retry belongs on the mutation paths,
  where idempotency is actually reasoned about.

**Now also migrated: the mutations and the remaining reads.** `Promise`, `Reflect`, `Observe`,
`Predict`, the commitment mutations, `InterruptLifecycle`, `Activity` and `DetailedObligations` are
all asynchronous continuations with delayed replies. No call on the Presence surface blocks.

The distinction that matters: mutations are **not** parallelised, and should not be. Their steps are
causally ordered — the capability gate decides legality, the Event1 preflight decides durability,
and the accepted Observation is the cause the domain mutation links to. Asynchronous is a separate
property from concurrent: the chain still runs in order, it just continues from each reply instead
of blocking on it. A process test suspends intentiond mid-chain and measures a second caller being
served in 0 ms while the command is in flight.

Safety comes from the operation semantics rather than the retry policy. A `NonIdempotentMutation` is
never retried whatever the policy says, and a timeout on one surfaces as `unknown-outcome` rather
than failure — the contract the shell relies on for `InterruptLifecycle`, since a reply that never
arrived does not mean the run was not finished.

Two cleanups fell out. `LastError()` walked each synchronous client in turn; those clients no longer
carry commands, so every fallback could only return state left over from an unrelated earlier call —
a stale error attributed to whichever command asked next. Each command now records its own failure
and publishes it when it replies. The synchronous clients are gone except where their `Changed`
subscriptions are still how presenced learns to re-emit its own.

The new coverage is ordered last in the suite because suspending an owner leaves healthd's capability
graph degraded until it re-probes, and the scheduling tests refuse to start a run against that. The
disturbance is to the *graph*, not to healthd's responsiveness.

## A3 — presenced cannot be reported as degraded

**Where:** ``PresenceService.cpp``,
`PresenceService::Ready()` returns `true` unconditionally and `PresenceService::Health()` returns
`"healthy"` unconditionally.

**Claim under test:** capability-specific degradation, and the `presence-presentation` capability in
healthd's graph.

**What the code does:** healthd derives component health from exactly these two answers. Because both
are constants, `presence-presentation` cannot enter a deficit state for any reason. presenced can
have every downstream owner unavailable, be returning empty projections, and be reporting its own
failures through `LastError()` — and still be graphed as healthy.

`LastError()` already aggregates the real failure state from each client. The information exists;
`Health()` simply does not consult it.

**Resolution:** derive `Health()` from the most recent aggregation outcome and the reachability of
required downstream owners, keeping "presenced is running" separate from "presenced can present".
`Ready()` may legitimately stay `true` — presenced has no startup state to load — but that should be
a stated reason rather than a coincidence.

## A4 — Unbounded read paths on the single-writer process

**Where:** ``EventService.cpp`` and
``Journal.cpp``.

Three separate paths on `cybou-eventd` have no size bound, and each is a synchronous D-Bus method on
the process that owns the only write path to the Journal:

1. `EventService::Verify` calls `Journal::verify`, which reads every row and recomputes the entire
   hash chain. Cost grows without limit with biography size.
2. `EventService::ConsumerBacklog` special-cases the `lifecycle.consolidation` consumer by issuing
   one `atSequence` query per row of backlog, rather than one aggregate query.
3. `Journal::recent` treats a non-positive limit as "no limit" and serialises the entire table into
   one D-Bus reply.

**Claim under test:** the checkpoint's P0 risk "no performance envelope for Journal and compound
projections", and the P0 risk that same-user D-Bus is not a security boundary. These three paths are
where the two risks meet concretely: any process in the user session can hold the canonical writer
busy by calling a read method in a loop.

**Resolution, corrected during implementation.** The original resolution here proposed capping
`Recent` and bounding `Verify` per call. Reading the callers showed that would be wrong, and the
correction is recorded rather than quietly dropped:

- `ConsumerBacklog` **is** a pure oversight and is fixed. One aggregate counting query replaces the
  per-row decode. The NULL capability scope is handled explicitly, because a plain SQL inequality
  would drop those rows through three-valued logic and undercount the backlog.
- `Recent` **must not** be capped. `recent(0)` is deliberate: intentiond, predictord, and selfd
  reconstruct their entire state by replaying the whole biography through it. A cap would silently
  truncate organ state reconstruction — a correctness failure strictly worse than the latency it
  would fix.
- `Verify` **must not** be bounded per call without changing what it means. selfd calls it on the
  ordinary self-assessment path, so a partial verification would have to be reported as partial
  rather than presented as an integrity result.

So two of the three are not oversights. They are a design that is correct and does not scale: every
organ rebuild pulls the full biography across D-Bus, and every self-assessment rechains it. Fixing
that properly means incremental verification against a persisted checkpoint and a replay API that
carries a cursor — a schema and contract change, not a parameter change. It is scheduled as its own
slice rather than mechanically capped here.

None of this closes the same-user authorization gap; that needs caller checks in `ServiceHost`.

## A5 — Fault injection is compiled into shipped binaries

**Where:** ``RpcResilience.cpp`` reads
`CYBOU_RPC_FAILPOINT` and `CYBOU_RPC_FAILPOINT_METHOD`;
``LifecycleService.cpp`` reads
`CYBOU_LIFECYCLE_FAILPOINT`. Both call `qFatal` when the variable matches.

**What the code does:** an environment variable crashes a Mind process in the installed build.
Related timing knobs (`CYBOU_PRESENCE_ARTIFICIAL_DELAY_MS` and the predictord equivalent) insert
real `QThread::msleep` calls into the production binary for the same reason.

**Withdrawn on examination. This is not a finding; the original resolution was wrong.** Two facts
that reading the code more carefully made clear:

1. **The failpoints grant no capability.** `qFatal` terminates the process. Any process in the same
   user session can already do exactly that with a signal, and the same-user boundary is the one
   the checkpoint already records as a P0. Setting the variable for a running service additionally
   requires control of the service manager environment, which is strictly more privilege than the
   crash it would buy. Removing them would close nothing.
2. **Removing them would weaken real evidence.** `tests/lifecycle-continuity.nix` sets
   `CYBOU_LIFECYCLE_FAILPOINT` against the **installed** package to prove split-commit recovery
   across a real reboot. Gating the hooks out of the package build would move that evidence onto a
   binary that is not the one shipped — trading proof about the artifact for tidiness in it.

**Resolution:** the hooks stay, and the reason is recorded in the threat model as an accepted
property rather than left to look accidental. The general rule this case illustrates is worth
keeping: test scaffolding in a shipped binary is a smell, but a smell is a reason to check the
threat, not a finding on its own.

## A6 — Mind units carry no hardening directives

**Where:** ``mind-services.nix``.

Every unit sets `Type`, `BusName`, `ExecStart`, and restart policy, and nothing else. There is no
`NoNewPrivileges`, no filesystem protection, no address-family restriction, and no memory bound.

**Scope of the fix:** this is the cheapest item in the audit, and it is also the one most likely to
be overstated. Unit hardening constrains what a compromised Mind process can reach. It does nothing
about the checkpoint's actual P0 — that any same-user process may call Mind mutation interfaces —
because that caller is a peer, not a child. Both are worth doing; they are not substitutes, and the
hardening should not be recorded as progress on the authorization boundary.

**Resolution: applied and verified.** The units now set `NoNewPrivileges`,
`RestrictAddressFamilies=AF_UNIX` (Mind opens no network socket), a `@system-service` system-call
filter less `@privileged` and `@resources`, and the namespace/realtime/SUID/personality
restrictions. All of these are enforced through seccomp, rlimits, or the no-new-privileges bit,
which apply to unprivileged `systemd --user` units.

Namespace-based directives — `ProtectSystem`, `PrivateTmp`, `ProtectHome`, `PrivateNetwork` — are
omitted deliberately. A user manager cannot reliably apply them, and a directive that silently fails
to apply is worse than an absent one because it reads as protection that is not there. `ProtectHome`
would be wrong in any case: the canonical Journal lives under `$XDG_STATE_HOME` in the user's home.

Two directives were removed after the VM gate rejected them, which is worth recording because it is
the argument for running these gates at all:

- `CapabilityBoundingSet` failed every daemon at step `CAPABILITIES` with status 218 — a user
  manager cannot drop the bounding set. It was also redundant, since unprivileged user units hold no
  capabilities to drop. Had this been merged on the strength of the fast checks alone, it would have
  broken Mind on every desktop while CI stayed green.
- `MemoryDenyWriteExecute` is not set, because Qt allocates executable pages.

All four KVM gates pass with the hardening applied, including both split-commit reboot recoveries.

## A7 — No fault evidence runs in hosted CI

**Where:** [`checks.yml`](../.github/workflows/checks.yml) and the four gates in `tests/`.

The fast workflow runs the static validators, both packages, and — through the `cybou-mind` package
build — the CTest suites. None of `vm-smoke`, `p4-plasma-lifecycle`, `lifecycle-continuity`, or
`m6-recovery-boundary` runs on a push, because all four need KVM.

The workflow comments explain this decision for the ISO, and that reasoning is sound: a gate that
fails for infrastructure reasons trains everyone to ignore red CI. The consequence is broader than
the comment acknowledges. Restart continuity, split-commit recovery, reboot reconstruction, and
Plasma recreation are the project's most distinctive evidence, and a regression in any of them is
invisible until someone runs the gates by hand.

**Resolution:** this is a policy question, not a code fix, and it is recorded here rather than
scheduled. The options are a KVM-capable runner, a scheduled rather than per-push run, or an explicit
documented statement that the fault tier is a pre-tag manual gate with a named owner. The current
state is the third option without the statement.

## A8 — One bounded-budget assertion tested ordering rather than the contract

**Where:** `presenceSnapshotHasOneBoundedOwnerBudget` in
``tst_m4_process_integration.cpp``.

Found while fixing A1, not by reading: raising the Journal commit mode shifted timing by roughly
sixteen milliseconds and turned this test red.

**What the test asserted:** that after suspending selfd under a 500 ms budget, both the selfd
section *and* the lifecycle section of the projection come back empty.

**Why that is not the contract:** P6.7 guarantees that a compound read consumes one shared budget
and stays structurally valid, with typed defaults for whatever it did not reach. It does not
guarantee which owners go unreached — that depends on how much of the budget the suspended owner
happens to consume. Here the suspended owner used 484 ms of 500 ms, and lifecycled, being local and
fast, answered inside the remaining margin. The projection was correct; the assertion was pinned to
the ordering of `snapshotMap` rather than to the property under test.

The test also carried an unstated precondition. presenced gates the selfd read on Health1 reporting
`self-assessment` as available, and healthd returns an instantly empty snapshot before its first
refresh. With Health1 unavailable the gated read is skipped, the budget survives untouched, and the
test passes without exercising accumulation at all — passing for the opposite of the intended
reason.

**Resolution:** assert the bounded total, the presence of every projection key, and the suspended
owner's own empty section; wait for the capability precondition rather than assume it; and state in
the test why no specific later section is pinned. This is the "no silent caps" problem in test form:
a green result was reporting more coverage than it had.

## Effect on the checkpoint

The checkpoint's structural conclusions hold. The maturity matrix needs three adjustments:

| Area | Checkpoint | Adjusted | Reason |
|---|---:|---:|---|
| Canonical memory/Event1 | 3 | 2 | A1: the durability claim is not supported against power loss, and the test design cannot test it |
| RPC resilience | 3 | 3 | A2 closed: the whole Presence surface now uses the rated stack, and healthd already did. Restored rather than raised — the evidence is the same focused kind the score already described |
| Degraded operation | 3 | 2 | A3: one of eight projected capabilities cannot enter a deficit |

The recommended M7 entry sequence does not change. A1, A3, A4, and A5 should land before P7.0,
because M7 raises event volume and adds a second projection — which is precisely when an unbounded
read path, a dishonest health answer, and an untestable durability claim become expensive. A2 is
scheduled with them but sized as a refactor. A6 is independent. A7 is a decision for the maintainer.
