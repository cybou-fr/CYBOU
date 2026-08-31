<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Glossary

**CYBOU Desktop** — The primary spatial interactive surface and Presence layer for CYBOU. It organizes
system projections and bounded capabilities as interactive Cards across a unified spatial canvas.
It is Presence, not Mind, and is not an authorization authority.

**Living Canvas** — The underlying web-first frontend package and renderer for CYBOU Desktop, built in
Rust/WebAssembly and delivered via `cybou-web-gateway`.

**Card** — The primary interactive surface unit in CYBOU Desktop representing system projections
(System cards), interactive capabilities (Tool cards), or temporary previews (Ephemeral cards).

**CardSpec** — Static specification defining the architectural kind, capabilities (movable, resizable,
collapsible, closable, deckable), and dimensional bounds (min/max/default sizes) of a Card.

**Deck** — A spatial presentation container grouping multiple Cards into tabbed views without mutating
underlying card identities or domain state.

**Arrangement** — A pure, deterministic algorithm that calculates spatial placement for unpinned Cards
across modes including Free, Compact, Grid, Relations (causal graph clustering), and Focus (radial orbit).

**CYBOU Shell** — A strictly bounded, typed capability surface to the Debian 13 host (Body), executing
only builtins (`help`, `pwd`, `ls`, `cd`, `cat`, `clear`) inside a jailed filesystem without arbitrary execution.

**cybou-jailfs** — Safe filesystem isolation library providing strict path normalization, fd-relative
directory resolution (`RESOLVE_BENEATH`), symlink escape protection, and resource budgeting.

**Web gateway** — Proposed separate browser/network boundary that authenticates sessions, serves
the frontend, validates versioned web requests, enforces transport policy, and maps only explicit
operations to local Presence/Mind APIs. It is not a cognitive state owner.

**Rust-first** — Target policy that Cybou-authored executable product components, including Living
Canvas compiled to WebAssembly, are implemented in pure Rust.
tree or exempt a rewrite from compatibility and continuity gates.

**Body** — Debian 13, hardware, kernel, storage, processes, and system/external state that Cybou can observe or
eventually affect through explicit capabilities.

**Mind** — the persistent cognitive substrate: typed state and processes for biography, identity,
commitments, prediction/calibration, self projection, and bounded attention. A language model or
one daemon is not Mind by itself.

**Presence** — the outward presentation/projection boundary. Living Canvas Presence is a remote
proxy/cache rather than a cognitive state owner.

**Organ** — a component with one narrow cognitive responsibility and explicit state ownership.

**Faculty** — optional replaceable capability such as language, perception, or planning. A faculty
is not identity, canonical memory, or authorization authority.

**Capability** — an explicitly available system ability. Health1 derives current availability and
typed deficits from component observations and the dependency graph, so an optional-organ failure
does not become a whole-Mind failure.

**Contribution** — a typed cognitive message/envelope that may become part of durable causal
history.

**Observation** — direct input and the only legal root contribution for new Journal v2 causal
chains.

**Outcome** — a terminal typed result tied to prior causal state. Future external actions should
return observed consequences as outcome/evidence rather than ending at command dispatch.

**Correlation** — membership in one episode.

**Causation** — direct prior cause.

**Evidence** — additional supporting prior contributions.

**Journal** — append-only canonical durable biography owned by `cybou-eventd`.

**Biography** — accepted durable cognitive history represented by Journal contributions and their
causal/evidence relationships.

**Identity** — the persistent subject identifier/state whose continuity is stronger than any one
process lifetime. The term is an engineering concept, not a claim of consciousness.

**Session** — a logical user-login period associated with identity continuity. Restarting one organ
inside a login should not automatically create a new session.

**Intention** — an unresolved commitment/goal-like typed state owned by `cybou-intentiond`, with
explicit terminal transitions such as fulfilled or abandoned.

**Prediction** — typed expectation state owned by `cybou-predictord`.

**Calibration** — accumulated comparison between prediction and later observation/outcome, used to
measure how expectations perform over time.

**Self model** — structured self projection/assessment owned by `cybou-selfd`. Future natural
language narration may formulate it, but should not invent authoritative self facts.

**Workspace** — bounded active context owned by `cybou-workspaced`; reconstructible transient
attention, not a second biography.

**Coalition** — related active Workspace material competing/cooperating for bounded attention.

**Salience** — Workspace relevance/priority signal used to select current focus.

**Continuity** — verified persistence of identity, biography, commitments, and supported transition
state across lifecycle boundaries.

**Cognitive lifecycle** — explicit software modes governing availability, consolidation,
maintenance, recovery, degradation, and suspension. It is not a claim of biological sleep.

**Consolidation** — a bounded, recoverable maintenance run that asks existing owners to derive,
calibrate, reconcile, expire, or rebuild state from an accepted Journal high-water mark. It does
not rewrite accepted history.

**Lifecycle coordinator** — `cybou-lifecycled`, the current owner of lifecycle transitions,
persistent run state, bounded maintenance dispatch, recovery, and evidence-bound automatic
scheduling. It owns orchestration, not the state of every organ.

**Provenance** — source, acquisition, transformation, and trust context attached to an observation
or derived claim.

**Epistemic projection** — current derived view of claims and their status: observed, reported,
inferred, assumed, disputed, superseded, stale, or unknown. It is not canonical biography.

**Reconciliation** — causally recorded handling of conflicting or stale state without silently
discarding the conflicting evidence.

**Retention** — explicit policy for active storage, archive, expiry, erasure, and propagation of
those decisions into derived or replicated material.

**Homeostasis** — typed regulation signals for internal pressure such as storage growth, backlog,
latency, stale projections, unresolved contradiction, or calibration drift.

**Metacognition** — structured projection of what Mind knows about its own uncertainty, freshness,
assumptions, evidence, and capability deficits.

**Executive attention** — bounded policy for interruption, deferral, return, and competition among
active concerns; an extension of Workspace admission, not authorization to act.

**Value constraint** — typed consideration such as user authority, safety, privacy, reversibility,
cost, urgency, evidence quality, or resource budget used to criticize and prioritize proposals.
It does not grant execution permission.

**Degraded mode** — current explicit state in which Mind remains partially available while one or
more capabilities are unavailable or uncertain. Lifecycle mode and aggregate health are separate
axes: for example, Mind may be `Awake + Limited` or `Recovering + Limited`.

**Node** — one device/runtime participating in a future distributed continuity topology.

**Language faculty** — a capability for interpretation, proposal and explanation. It may
be a grammar, a parser, a classifier, a local or governed remote model, or a hybrid; no generative
model is required.

**Faculty** — a replaceable ability. Not an authority.

**Worker** — a temporary actor created for one bounded task, holding only the context, capabilities,
network destinations, budget and lifetime its grant names.

**Agent** — a longer-lived actor responsible for a continuing domain. It does not own the state of
that domain; Mind does.

**Broker** — the mediated path between an actor and a model, tool or MCP server. Discovery is not
authorization, so nothing becomes reachable merely by existing.

**Standing authorization** — durable policy permitting a bounded unattended action in advance. Past
approval, high confidence and a successful history are none of them standing authorization.
It must not directly own Journal, identity, authorization, or privileged execution.

**Authorized Action Boundary** — the policy boundary between uncertain cognition/planning and
typed external mutation.

Terms such as **Mind**, **identity**, **self**, **attention**, and **cognitive** are software
architecture terms in this repository. They do not assert sentience or biological equivalence.
