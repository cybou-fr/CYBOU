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

Every subject is readable on any Linux host with no configuration. That is deliberate and it is why
certificate expiry, per-service availability and backup age are not among them: each needs to be told
*which* certificate, service or backup, and a configured subject is a different kind of thing from
one that is universally true.

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
produces no finding can be the most important thing on the page. The slope is a Theil–Sen estimate —
the median of every pairwise slope — so one spike does not move the date, and the yardstick it is
called flat against is a median absolute deviation for the same reason. A subject that is flat or
moving away does not arrive: *not at this rate* is an answer, and a very large number would be read
as a date. A projection is measured from now rather than from the last reading, and it says when it
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
