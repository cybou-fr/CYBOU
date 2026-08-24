<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Current State

What exists now. Not what was built, and not when — `git log` holds that, and holds it better than
prose does.

This document is the implementation authority: where it disagrees with an aspirational description
elsewhere, this is what the code does. Where it disagrees with an **Accepted** ADR, the ADR outranks
it and the implementation is wrong.

Claims here that need defending have an [evidence document](evidence/README.md) naming the command
that re-checks them. Claims with nothing behind them are in [What is not
built](#what-is-not-built), stated as absences rather than omitted.

## Deployment

A VPS, a server, a VM or a container running Debian 13, reached through a browser over HTTPS
([ADR-0041](adr/ADR-0041-server-first-deployment.md)). A personal workstation is a supported place to
run it and is not what it is for.

Debian 13 is the integration authority: the daemons need a session bus and systemd user units, so
the multi-daemon gate and every deployment run there. The portable half — protocol, storage,
meaning, the frontend — builds and tests anywhere.

Nothing in the cognitive substrate loads a model, needs an accelerator, or requires a network.

## Mind owners

There are fourteen Mind owners: fourteen user-session processes, each owning one versioned D-Bus
interface under `org.cybou.Mind.`.

| Owner | Interface | Owns |
|---|---|---|
| `cybou-eventd` | `Event1` | the canonical Journal; the only writer |
| `cybou-identityd` | `Identity1` | subject continuity across sessions |
| `cybou-healthd` | `Health1` | capability states and the dependency graph |
| `cybou-intentiond` | `Intention1` | open obligations and commitments |
| `cybou-predictord` | `Predictor1` | statistical prediction and calibration |
| `cybou-perceptiond` | `Perception1` | stable observations about the host |
| `cybou-telemetryd` | `Telemetry1` | bounded transient Body state and system insight |
| `cybou-epistemicd` | `Epistemic1` | what is known, and with what epistemic force |
| `cybou-contextd` | `Context1` | associative context and bounded activation |
| `cybou-workspaced` | `Workspace1` | attention coalitions and admission |
| `cybou-meaningd` | `Meaning1` | the meaning boundary |
| `cybou-lifecycled` | `Lifecycle1` | sleep, wake and consolidation |
| `cybou-selfd` | `Self1` | autobiographical self-assessment |
| `cybou-presenced` | `Presence1` | the presentation-ready projection |

Beside them, and deliberately not among them:

| | |
|---|---|
| `cybou-model-brokerd` | `org.cybou.Faculty.ModelBroker1` — a faculty, owning no part of Mind |
| `cybou-web-gateway` | the HTTP boundary; not a Mind owner and holds no cognitive state |

`cybou-shelld` is a library with no binary. Its unit exists, describes a process that does not run,
and is excluded from the deploy and from `cybou-mind.target`.

Every organ is a separate process that fails separately, so a silent organ is a gap on a page rather
than an outage. A projection that could not be read reports *unknown*, never *empty*.

**Layering.** `telemetry → journal → epistemic → associative → attention → meaning → governance`.
A layer may read the one above it and may not overrule it. A faculty may depend on no organ in either
direction. Both are checked at the manifests by `scripts/validate-organ-layering.py`.

## The Journal

One writer. Append-only, hash-chained, schema v3, with v2 still readable.

- A contribution's canonical form is pinned to byte fixtures, not to a round trip. See
  [journal compatibility](evidence/journal-compatibility.md).
- Erasure removes a payload and raises an epoch; every derived projection discards and rebuilds. See
  [erasure gate](evidence/erasure-gate.md).
- A contribution whose privacy is weaker than something it references is rejected.
- Terminal outcomes are typed: an attempt that did not finish is not an outcome.

## Body observation

`Perception1` records what is stable about the machine — kernel, hostname, memory size — and stops
there.

`Telemetry1` watches what a host is doing: load, memory and swap use, memory/IO/CPU pressure, root
filesystem bytes **and inodes**, open file descriptors against the system limit, failed units.
Windows are bounded twice, by span and by count.

Those subjects are readable on any Linux host with no configuration. A subject that needs to be told
*which one* is a different kind of thing, and is declared rather than discovered — a probe that went
looking for certificates would decide for the operator what is worth watching.

Declarations live one per line in `telemetry.watch` under the configuration directory. **A line this
build cannot read is an error, not a comment**: an operator who mistypes `certificate` has told their
machine to watch one, and a skipped line means they believe it is watched, nothing is, and the first
they hear of it is an expired certificate. A refused file is announced with every bad line and its
number, and the universal subjects keep running — a mistake in an optional file must not remove
watching that was never in question.

Three subjects are declared rather than universal: `certificate.days.remaining`, `service.active`
and `backup.age.days`. One window per declared thing, one finding per thing rather than one naming a
count — an operator with four certificates needs to know which.

**What a measurement is about travels as one key, not two fields.** `MetricKey` is a subject and,
for a declared thing, which one. It is what the windows are keyed by, what a deviation is keyed by,
what a finding cites as its evidence, what a projection names, and what a remediation proposal reads
its target from. The two halves used to travel separately and were repeatedly dropped apart: two
certificates produced two windows, two findings and *one* deviation, because the map holding the
deviations had nowhere to put the name — so whichever finding was built second cited the other
certificate's readings as the evidence for itself. A proposal about a declared service now names the
unit it would act on, taken from the finding, rather than the `systemd:<unit>` placeholder a
proposal falls back to when it genuinely does not know which one it means. The count of failed units
still falls back, because it genuinely does not.

**A watched thing has four states, and three of them are not silence.** `Observed`, `NeverRead`,
`ReadFailed` and `Stale`. A declared thing that produced no reading used to be simply absent from
every surface, which reads exactly like a thing nobody declared — and the operator who declared a
certificate and sees nothing about it has been told, by that silence, that it is fine. The three
unhappy states are kept apart because they call for different actions: a path that may not exist, a
file this process cannot open (usually a permission), and a probe that worked and has stopped
(usually the sampler, not the thing sampled). A failed attempt outranks an old success, because
"last read four minutes ago" hides "and every attempt since failed"; a reading that arrives clears
the failure before it. The unread things are named in the prose *before* the all-clear, and never
counted among the things looked at.

**A finding keeps one identity for as long as it is one condition.** The identity is derived from
what makes a condition itself — what was concluded, what it is about, and when it began — rather
than generated per read. A fresh identity per read meant two requests a second apart described one
physically identical situation with two different identities: harmless while nothing referred to
them, and an architectural defect the moment an action proposal cites one as its cause, because the
cause it names would not exist by the time anybody looked. An episode that ends and returns gets a
new identity, which is correct: it is a new occurrence, and a `since` spanning both would describe a
stretch of time the host was fine for part of.

**Backup age has no universal threshold, and this build does not invent one.** How stale a backup may
get is a policy the operator holds, and two backups on one host can honestly disagree, so the number
comes from the declaration and the subject table supplies none. A backup declared without one is
refused rather than defaulted; a backup watched with no threshold at all is watched and never judged,
which is the honest state rather than a number nobody chose.

Nothing is offered to remedy an expiring certificate or a stale backup. Renewal is a deadline met
outside this machine's control, and what would relieve a stale backup is the backup succeeding —
there is no operation here that could make it. An inactive service is offered the same three things
as a failed one, because which it is affects what the host says rather than what it could do.

Two of them exist because an existing measure reads healthy while the machine is broken. A filesystem
out of inodes has free bytes, so every byte-based reading says forty percent used while nothing can be
created. A host at its file-descriptor limit has memory, disk and load all fine, and cannot open a
socket. Both are found and both are their own finding, because deleting things frees no descriptors.

**Telemetry is not biography.** A `Reading` has no path into the Journal anywhere in this tree — no
kind, no conversion. It is transient by construction rather than by policy. A `SystemInsight` may
enter Event1, and only as a `Hypothesis` carrying the readings it was drawn from.

Detection is robust statistics — median and median-absolute-deviation over the window — rather than a
model, because the fault contaminates a mean and a standard deviation while it is happening, and
because the answer must be checkable by a person and runnable on one vCPU. Categorical facts (a disk
at 95%, a failed unit) are evidence in their own right and do not need a baseline.

A window too short to have an opinion says so. That is a different answer from *nothing is wrong*.

**Where things are heading** is a separate question from what is wrong now, and a disk at 71% that
produces no finding can be the most important thing on the page. The slope is a Theil–Sen estimate
over at most 128 points sampled evenly from the window — the estimator compares every pair, so a
full six-hour window would be 2.33 million comparisons for one subject and four seconds for a page
load on one vCPU. The surface that answers *why is this server busy* must not be a reason it is.
The detector still sees every reading; only the slope is estimated from a subset, and the sampling
is deterministic so two projections of one state are the same projection. What it costs is a little
precision in a number that is rounded to "about three days" before anyone reads it. The slope is
the median of the sampled pairwise slopes — so one spike does not move the date, and the yardstick it is
called flat against is a median absolute deviation for the same reason. A subject that is flat or
moving away does not arrive: *not at this rate* is an answer, and a very large number would be read
as a date — and *away* depends on which side of the threshold the problem is on, because a
certificate losing a day a day is approaching while a filesystem losing a percent a day is
retreating. A projection is measured from now rather than from the last reading, and it says when it
is looking further ahead than the window has watched — the most useful projection is usually the
least certain, and a reader deciding whether to act tonight is entitled to both facts.

## Meaning

An utterance becomes a typed `CognitiveAct` or none at all; an unrecognised opening produces no
interpretation rather than a guess. A reference the vocabulary cannot settle stays unresolved and
names the candidates it was torn between. A bare pronoun never resolves.

Dialogue state remembers **referents, not a topic**, bounded by turns, by time, and by erasure. It
can make an ambiguity visible and has no way to make one disappear; there is deliberately no
accessor for "the current subject".

Answers are built as a typed `ResponsePlan` before any prose exists, so qualifications — not read,
stale, partial, withheld, unverified, disputed, superseded — are decided in the typed layer where a
renderer cannot lose them. Realization has no input but the plan. Plans compose only when they share
an intent, and **a qualification on any part qualifies the whole**.

Everything here is deterministic and runs with no network and no model.

## Context and attention

Activation walks associations from named seeds under a budget enforced in five dimensions — nodes,
edges, depth, time, tokens — and says which one stopped it. Every reached concept carries the path
it came along; *why did you think of that* is answered from the graph, never by asking something to
compose a reason.

An epistemic standing travels with a concept through retrieval and into attention. A concept reached
*through* a disputed one is **not** thereby disputed: the walk is association, not inference.

Attention admits proposals under a quota and **a proposal never evicts a resident**. Relevance
discovered by retrieval is not permission to displace the current focus. What was refused is counted,
so a short list is never mistaken for a whole one.

## Disclosure

Every supply of the Mind projection across a boundary writes a `ContextDisclosed` naming the
consumer, the contributions the supplied items came from, and what was held back and why.

`GET /api/v1/disclosure` shows the person it is about: how much was supplied against how much can be
accounted for, and every refusal with its reason. A withheld subject is named to the owner and
withheld from a stranger — a surface reporting a filter must not be a way around it.

Recorded provenance is bounded, and a record says by arithmetic when it is a sample: the count
exceeds the length. The count is optional, because a record written before the field existed cannot
say how many sources there were, and that is not zero.

## Desktop

One Rust/WebAssembly frontend — Living Canvas — served to browsers and, as a target, to a
Chromium/Wayland session. The browser is a renderer and an untrusted client: it talks only to the
gateway and never becomes a Mind owner, D-Bus peer, or authority.

Thirteen singleton system cards and dynamic tool cards. Arrangement follows a declared relationship
graph. Every class the components render has a rule, checked by
`scripts/validate-desktop-styles.py`; interaction is exercised in real Chromium. See
[desktop and browser gate](evidence/desktop-browser-gate.md).

A stranger is served the sign-in view and nothing else where the deployment says so.

## Model brokerage

`org.cybou.Faculty.ModelBroker1` selects a route, enforces a budget, puts a request and attributes
what comes back. It holds no biography, reads no Journal, touches no filesystem, authorizes nothing
and executes nothing.

- A task is a closed set, and every task answers for its own absence: something deterministic already
  does it, or the feature is **absent** — never silently degraded.
- No `ModelOutput` variant asserts a fact or names an action. The strongest thing a model can return
  is a candidate something else must accept.
- Attribution is by artifact digest and template version, not by configured name.
- A request names the disclosure its input was drawn from, in a field that is not optional.
- Route selection is by declaration order, so the same request always chooses the same worker.

**No inference runtime is implemented and no model has ever been loaded.** On an installation with
no worker, every request is answered with what happens instead. That is a supported configuration.

## Action boundary

The host observes itself, concludes, explains, offers, and refuses.

A proposer **cannot choose its own risk**: operations are a closed set and risk and reversibility are
functions of the operation. Reversible does not mean harmless; irreversible does not mean forbidden.

Critical operations — deleting a service's data, formatting a filesystem, powering off — are never
offered and are refused if something else builds them. A standing policy cannot grant what the
operation table forbids, and a failed critic stops a pre-authorised operation.

**Nothing is granted on an installation nobody has configured**, and nothing is carried out at all:
there is no executor and no code path that could reach one.

## Current gates

Every one of these runs on Debian 13 and in CI, and all pass:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test -p living-canvas --target wasm32-unknown-unknown --locked
bash scripts/test-multi-daemon-integration.sh
python3 scripts/validate-cognitive-docs.py .
python3 scripts/validate-desktop-styles.py
python3 scripts/validate-organ-layering.py
python3 scripts/validate-doc-links.py
reuse lint
```

`unsafe_code = "forbid"` and `clippy::pedantic` are workspace-wide.

There is currently no language-model process and no privileged action-executor process, so nothing
in this tree can execute a mutation of the host.

## What is not built

Stated as absences rather than left out, because a capability nobody built and a capability nobody
mentioned look identical to a reader.

| | |
|---|---|
| Inference runtime | no local or remote model worker exists; the brokerage contract has nothing behind it |
| Action executor | proposals and authorization exist; nothing can carry one out |
| Native desktop session | `cybou-desktop.service` is built and ships disabled; it has never run on a machine with a seat |
| Sensitive payload storage | the AEAD primitive, key store and erasure protocol exist and are tested; no payload is encrypted and no perception source is sensitive |
| Automatic retention expiry | retention classes are carried; nothing acts on a lifetime |
| Semantic file index | not started |
| Backups | `BackupState` reports `NoneDeclared`, which is true: no backup software or rotation is configured |
| Inter-node transport | no replication, no partition handling, no distributed anything |
| Learning promotion in practice | the gate is implemented and evaluated; no candidate has ever been promoted |

## Known limitations of what is built

- **Telemetry has no persistence.** Everything it holds is in memory and bounded; a restart starts
  the window again, and the organ says it has not watched long enough rather than answering.
- **The context projection has no checkpoint.** It replays from the Journal on every start. Correct,
  and slower than it needs to be on a long biography.
- **`Predictor1` is domain-neutral.** It forecasts a level — where a subject sits relative to its own
  history — which is not what an operator asks. The operational projections (filesystem growth,
  pressure, swap) are computed in `Telemetry1`, where the series lives; certificate expiry and
  service availability are not projected at all.
- **Continuity is proven across process restart, not machine reboot.** See
  [reboot continuity](evidence/reboot-continuity.md).
- **Nothing is proven under load.** The integration gate proves the system comes up and is coherent,
  not that it stays so under conditions nobody has applied. See
  [Debian integration](evidence/debian-integration.md).
