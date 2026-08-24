<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0041: Cybou Is a Server-Side Cognitive System

## Status

Accepted (2026-08-23)

Recorded at the owner's direction. This is a product decision rather than a derivation, and it is
written down because it changes defaults that were already chosen the other way — including one
shipped the same day.

## Context

Nothing in the tree said where Cybou runs. Every decision that needed an answer therefore supplied
one locally, and they did not agree.

The evidence pointed one way. Development, integration and every deployment target a VPS. The
frontend is a browser artifact reached over HTTPS, and ADR-0037 already treats the remote session as
first-class rather than as a fallback. The desktop shell has never been run on a machine with a
seat, and its unit ships disabled.

The prose pointed the other way. The README says *local-first*. The intelligence profiles sketched
for local models assumed a laptop with a GPU. The model brokerage faculty shipped with
`PrivateNetwork=yes`, on the reasoning that a broker able to reach a network is one mistake away
from being the egress nobody noticed — which is a good argument on a personal machine and the wrong
default on a host whose whole reason for existing is to be reached.

Left unresolved, the disagreement would have been settled by whichever component was written next.

## Decision

### The deployment target is a server or a container

Cybou is a cognitive Linux environment for a VPS, server, VM or container: a machine that runs
unattended, is reached remotely, and is expected to look after itself. A personal workstation is a
supported place to run it and is not what it is for.

Stated for anyone outside this repository:

> **Linux that understands and operates itself.**

Not *Linux with a local LLM*, and not *an AI desktop distribution*. The difference is not
presentation. Those two describe a system whose intelligence is a component you install; this
describes a system whose intelligence is the runtime, with models as amplifiers of it.

Three things follow immediately, and they are the reason this is worth an ADR rather than a note:

**The frontend is the remote surface, not a local convenience.** Living Canvas over HTTPS is the
ordinary way a person meets Cybou. The Chromium/Wayland shell remains a target and stays optional;
it is one client of the same gateway, not the real product with a web fallback.

**There is usually no GPU, and often not much memory.** A cognitive layer that needs an accelerator
to function would be a cognitive layer that does not function where Cybou runs. Everything the
substrate does — biography, identity, epistemics, context, attention, meaning, planning, disclosure
— must keep running on an ordinary small instance, and today does: all of it is deterministic and
none of it loads a model.

**Reachability is the point.** A host exists to be reached. Egress therefore cannot be prohibited by
default; it has to be *governed*, which is a different mechanism and a harder one.

### A larger model may live behind an API, under the rules that already exist

Cybou may consult a model it does not run. ADR-0035 already says what that means and none of it
changes — it becomes load-bearing rather than hypothetical:

```text
remote model = external-boundary consumer   (ADR-0030)
model output ≠ knowledge                    (ADR-0021)
provider loss = capability deficit          (ADR-0035 MB6)
```

What does change is which of these is the common path. A remote route is now the expected
configuration, not the exception, and every gate around it is exercised in ordinary operation rather
than in a test.

Two things this does not license. A remote model does not become the cognitive layer: it is asked
bounded questions and its answers are proposals. And a remote model does not receive whatever is
convenient — it is a named consumer, so what reaches it is what a disclosure decision supplied, and
that decision is recorded.

### The local cognitive layer must be sufficient on its own

The system must look after itself and hold a conversation with a person **with no model available at
all** — no local one, and no reachable remote one. That is not a degraded mode to be tolerated; it
is the base case, because a host whose ability to explain itself depends on an internet connection
is least able to explain itself exactly when the connection is the problem.

The questions it has to be able to answer alone are not the ones a language model is good at. They
are the ones an operating system is uniquely placed to answer about itself:

```text
who am I?                      what is happening to me?
which session is this?         what changed?
what works, what is broken?    why do I consider that a problem?
what have I already done about it?
which of my intentions are still open?
what do I know, and what do I not know?
which actions are possible here?
which one do I propose?         may it be carried out?
did it work?
```

That is the baseline cognitive capability of an operating system. It is not the ability to write a
good essay, and none of it needs a generative model.

An external model makes the answers more fluent, plans better, summarises what is complicated, and
lets Cybou attempt things it otherwise cannot. It does not make Cybou work.

### What this changes about model priorities

The intelligence profiles sketched around laptop hardware are withdrawn. The useful axis on a server
is not how large a local model can be — it is what a request needs and what it is permitted to
reach.

The broker is local-and-remote symmetric. There is no `LocalModelBroker` and no remote special case:
one broker, and routes that differ in what they can do and what they may receive.

| Route class | What it is | Typical use |
|---|---|---|
| `NoModel` | no route at all | the base case, and it must always work |
| `LocalFast` | CPU-only, milliseconds | intent classification, routing a sentence |
| `LocalSpecialist` | CPU-only, small and narrow | embeddings, reranking, anomaly scoring |
| `RemoteStrong` | a large model behind an API | incident analysis, planning, explanation |
| `RemoteSpecialist` | a narrow remote capability | whatever a host cannot run itself |

`LocalFast` and `LocalSpecialist` are deliberately not "a small generative model". Generation on a
CPU-only instance is slow enough that a remote route is the honest answer; classification, embedding
and reranking are not, and they are what a host needs to reason about its own data.

Selection is by what the request requires, and a request that cannot be satisfied is refused rather
than downgraded. Asking to summarise private context with no external route permitted and no local
capability for it does not silently produce a worse summary from somewhere else — it produces
*strong language synthesis is unavailable under the current privacy policy*, which is a sentence a
person can act on.

## Consequences

The model brokerage unit's `PrivateNetwork=yes` was correct under the old reading and is wrong under
this one. It is removed, and the protection it was providing has to come from where it belongs: no
remote route is configured, so nothing is reachable; adding one is a decision, and what it receives
is decided by disclosure and recorded.

The first user-facing capability worth building shifts, and so does the whole order behind it.
"Find my French rental contract" is a workstation question. The server question is *what is going on
with this host* — and the vertical that answers it is:

```text
observe → understand → remember → diagnose → explain → propose → authorize → act → observe outcome
```

Every stage of that exists in this tree except the first and the last two, and the first is what
blocks the rest: nothing watches the Body. Semantic search over a host's files is genuinely useful
and is not the thing that makes this different from a wrapper around a model, because a wrapper can
also search files. Nothing else can tell you *why it thinks* the database stopped, from evidence it
gathered itself, while the internet is down.

`predictord` is pointed at abstract forecasting and should be pointed at the domain: root filesystem
utilisation, memory pressure, service availability, restart rates, certificate expiry, backup age.
The same statistics, aimed at subjects an operator has opinions about, is the difference between a
forecast and *at this rate `/var` reaches 95% in about three days*.

Documentation that says *local-first* is inaccurate and becomes *local-sufficient*: the distinction
is that Cybou does not require anything remote to function, while being built to be reached.

## Acceptance gates

Two of these define the product. The rest are the properties they rest on.

### S0 — Unplugged

> Cut internet access and every external model API. On a minimal VPS, Cybou continues to observe its
> Body, answer basic questions about its own state, detect a known problem, explain it through
> evidence, remember its open intentions, form a typed action proposal, obtain the authorization its
> standing policy provides for, carry out at least one bounded Body capability, and independently
> observe whether the expected outcome was reached.

The gate ends at the observed outcome and not at the proposal, and the difference is the whole
product. A system that stops at *here is what I would do* has demonstrated self-awareness; the claim
outside this repository is that Linux **operates** itself, and the smallest honest evidence for that
is one complete turn of the loop on a machine with no network and no model.

Stating it the shorter way, as this gate did until 2026-08-24, made the gate satisfiable by
something that cannot maintain anything — and left the vertical below it describing a longer path
than the gate it was written to serve.

### S0R — Plugged back in

> Restore the network and connect a large model. The quality of language, analysis and planning rises
> sharply. Identity, memory, epistemics, permissions and the ability to maintain minimum system
> control do not change owner.

Between them these say the whole thing: the model is an amplifier, and what it amplifies exists
without it. A system that fails S0 is a client for somebody else's model. A system that fails S0R
has handed its substrate to one.

| | Gate |
|---|---|
| **S1** | Mind starts, keeps identity and answers about its own state with no model and no network |
| **S2** | Every model task has a named deterministic answer or is reported absent, never silently degraded |
| **S3** | A remote route is a named external-boundary consumer whose deliveries are recorded |
| **S4** | Losing every remote provider changes capability and changes nothing about identity, biography or policy |
| **S5** | No cognitive function requires an accelerator |
| **S6** | The browser session is a supported first-class surface, not a fallback for a missing desktop |
| **S7** | Cybou observes its own Body continuously, without that observation becoming biography |

S1, S2, S4 and S5 hold by construction rather than by a run-time check: nothing in the substrate
loads a model. S3 is a contract, and stays one until a worker exists behind the broker.

S7 carries its own constraint, which is why it is a gate rather than a task: high-frequency Body
state is *not* biography. A Journal that accumulated a CPU sample every second would be a telemetry
database wearing a life story, and the erasure, retention and provenance rules that make the Journal
worth having would be applied to numbers that mean nothing individually. Bounded, transient, and
separate; only what is meaningful crosses into Event1.

Which gates hold at any moment is not recorded here. This is an accepted decision, and a decision
that carried a progress report would be a decision that goes stale without anyone changing it —
which is how the paragraph this replaced came to say that nothing watches the Body months after
something did. [Current State](../CURRENT_STATE.md) is the implementation authority.

## Alternatives Considered

### Personal-workstation-first, with server as a deployment option

Rejected as a description of what is being built. It would justify local-only inference defaults,
GPU assumptions, and treating the browser as a fallback — three things that are already false about
how this system is developed and deployed.

### Depend on a remote model for the cognitive layer

Rejected. It would make identity, memory and self-explanation contingent on a provider, a network
and a bill, which is the dependency ADR-0021 exists to refuse. It also fails at the moment it is
most needed: a host explaining a network problem cannot do it over the network.

### Prohibit egress entirely

Rejected. It is the safe-looking choice and it is not the product: a system that may never call out
cannot use a larger model at all, and the honest mechanism is a governed, recorded, refusable route
rather than an absent one.

## Related documents

- [ADR-0021: Language and Models Are Optional Faculties](ADR-0021-language-models-are-optional-faculties.md)
- [ADR-0030: Transparent Context Selection and Prompt Delivery](ADR-0030-transparent-context-delivery.md)
- [ADR-0035: Governed Model Brokerage](ADR-0035-governed-model-brokerage.md)
- [ADR-0037: Web-First Presence and Desktop](ADR-0037-web-first-presence-and-desktop.md)
- [ADR-0039: Debian 13 Base System](ADR-0039-debian-13-base-system.md)
