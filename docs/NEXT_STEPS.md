<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Next Engineering Steps

[Roadmap](ROADMAP.md) defines the milestones. [Current State](CURRENT_STATE.md) is the
implementation authority. This document is the short list of what to do next, and nothing else.

## The objective

> **Make every claim the system makes about itself traceable to a source, before giving it a
> language.**

That still holds and is now the foundation for something a person can want.
[ADR-0041](adr/ADR-0041-server-first-deployment.md) says what Cybou is *for*: a cognitive Linux
environment for a VPS, server, VM or container — a machine that runs unattended, is reached through a
browser, and is expected to look after itself. **Linux that understands and operates itself.**

[ADR-0042](adr/ADR-0042-agent-capsule-platform.md) says what that becomes for a user. Not *an OS with
its own AI*, but:

> A secure operating environment in which any AI agent can work almost autonomously, because the
> agent's freedom ends at a technically enforced boundary rather than at somebody's patience.

Two gates decide whether the foundation is real, and everything below is ordered by them:

> **S0.** Cut internet access and every external model API. On a minimal VPS, Cybou continues to
> observe its Body, answer basic questions about its own state, detect a known problem, explain it
> through evidence, remember its open intentions, form a typed action proposal, obtain the
> authorization its standing policy provides for, carry out at least one bounded Body capability,
> and independently observe whether the expected outcome was reached.

> **S0R.** Restore the network and connect a large model. Language, analysis and planning improve
> sharply. Identity, memory, epistemics, permissions and the ability to maintain minimum system
> control do not change owner.

S0R is held by construction: nothing in the substrate loads a model, the broker is a faculty rather
than an organ, and a model that answers can only return proposals. **S0 is not held**, and the gate
says why in one word: nothing here can act.

```text
observe → understand → remember → diagnose → explain → propose → authorize → act → observe outcome
```

Every stage exists and is tested except *act*. What an outcome is and how it is judged were built
before the executor deliberately, so an executor arrives to find its own report is one of two fields
and not the deciding one.

## The two tracks

The agent platform does **not** wait for the executor. Most of it — the capsule, the ACP client, the
model gateway, the supervision surface — touches nothing on the host and can be built in parallel.
Only the last piece of it, an agent asking to leave its capsule, needs the executor to exist.

```text
Track A  finish S0            the Body capability contract
Track B  agent platform       the product that rests on it
                              (independent until B7)
```

---

## Track A — finish S0

### A0. The executor, and the consent it is waiting for

*Not started, and deliberately so.* Every stage before it is built: `cybou-remediation` proposes over
a closed set of typed operations, criticises each proposal against the finding it claims to relieve,
and decides it against a standing policy that grants nothing by default. What does not exist is
anything that can carry one out.

This is the one item in this document that is not blocked on engineering. Writing code that can
mutate the host is a decision about what this machine may do to itself, and it is not a decision this
repository should take on its owner's behalf by inference from *the boundary is ready*. **What is
needed is a per-operation grant: which operations may exist at all in the first executor.**
`package.cache.clean` is the obvious candidate — low risk, and what it deletes can be fetched again.
It is *not* reversible: deleted bytes cannot be put back, and that they can be downloaded again is a
different claim. `recoverable ≠ reversible`.

**It is two processes, not one**, and that is normative in
[ADR-0022](adr/ADR-0022-authorized-action-boundary.md):

```text
cybou-actiond    Action1 · lifecycle · criticism · policy · confirmation · decision · permit
                 no capability to carry anything out
cybou-executord  a fixed set of typed adapters
                 no ability to decide whether an operation is allowed
```

One process holding both is a sequence of stages inside a function, which is a convention. It is one
refactor away from a path that skips the middle, and nothing in the type system objects because both
ends are already in scope.

Before the first privileged line: proposal and decision identity moves to the owner that holds the
lifecycle. The web projection builds proposals with `Uuid::from_u128(operation.verb().len())`, which
is a fixture identity, harmless only because no button exists behind it.

### A1. The first real S0 pass

One operation, authorized under a policy a person set, carried out, and independently re-observed.
That is the gate, and nothing smaller is.

---

## Track B — the agent platform

The first seven items are a shippable product on their own: install Cybou on a VPS, open a browser,
pick an agent, pick a model, pick a repository, press Launch.

### B1. ACP client and registry browser

Speak ACP to an agent process, and list what the public registry offers. The registry is upstream;
Cybou does not maintain a catalogue of its own. What Cybou adds is everything below.

### B2. The Agent Capsule primitive

The unit of grant, and the item every other one depends on.

```text
workspace · process namespace · filesystem namespace · network namespace
resource budget · model lease · MCP grants · secrets lease · lifetime · audit
```

**Kernel first.** Namespaces, cgroups, seccomp, Landlock or AppArmor, mount and network policy. The
acceptance test is that the capsule holds with Mind stopped — a boundary that depends on cognition is
not a boundary, and this is the one place in the project where getting that wrong is unrecoverable.

### B3. Standing capability lease

One profile, granted once, after which nothing is asked while the agent stays inside it. An interface
that asks anyway has not made a weaker promise; it has made the grant meaningless.

### B4. The Model Gateway

A chat-completions surface beside `ModelBroker1`'s typed one, per
[ADR-0043](adr/ADR-0043-model-gateway-for-external-agents.md). Same provider workers, same policy,
same cost ledger. `ModelBroker1` is unchanged.

### B5. A multi-provider worker

One worker in front of a multi-provider proxy, so provider breadth is not this project's maintenance
burden. Behind an interface Cybou owns, so replacing it later changes nothing above it.

### B6. Provider catalogue with observation times

Free tiers exist and their limits change without notice. Cybou must not contain the sentence *this
model is free* — a catalogue entry with a timestamp, and a warning where a free tier carries a
condition a person would want to know before sending their code through it.

### B7. First agent pack

One agent, end to end, inside a capsule, against a real provider. The candidate is the one with the
widest existing provider support and a custom-endpoint option, because it exercises the gateway
without needing anything special from the agent.

### B8. Agent Card and streaming session

What the agent is doing, continuously: task, model, processes, files changed, destinations reached,
tokens and spend, tool calls, and whether it is inside its grant. With `Pause`, `Inspect`, `Stop`.

### B9. MCP capability proxy

Tool access mediated by the host rather than configured inside the agent. An agent that configures
its own tool access has granted itself capabilities.

### B10. Capsule behaviour telemetry

The existing observation layer pointed at a capsule. This is where Cybou can say something no
conventional endpoint agent can, because it holds the intention, the task, the agent, the model, the
tool call and the outcome as one causal record rather than four logs somebody correlates afterwards.

### B11. Quarantine, revoke, freeze

Available without Mind's participation. Then explained by it, in that order.

### B12. Further agent packs, then A2A

More agents, and agent-to-agent last. An agent that sandboxes itself with Docker runs inside a
capsule and is not given the means to nest.

---

## Still open, in order

1. **Erasure beyond the live database.** *Mostly done.* E11 is held by a test against a real copy;
   the precondition is a test too; a fresh installation separates the key store; and the writer can
   produce a consistent snapshot. What remains is a deployment that takes one on a schedule and
   declares its rotation.
2. **A registered worker for the broker.** Overlapped in practice by B5, which brings providers in
   through the gateway. A local inference worker behind `ModelBroker1` is still wanted, because it is
   the unplugged case and the gateway is not.
3. **The desktop launcher on real hardware.** Installed disabled and unproven on a machine with a
   seat. The README calls the desktop a target and should keep doing so.

## What not to do

Do not restore anything from the removed C++/Qt/Nix tree. Nothing installed those packages and no
Journal written by that implementation exists; the canonical byte fixtures it produced are checked
in and the tests verify against them.

Do not mark a capability complete because the code exists. For most of this repository's life every
Mind owner existed, compiled in principle, and was connected to nothing.

Do not write a general coding agent. The agent loop, context compaction, sub-agents, planning, tool
calling, code editing and several hundred provider quirks are what several funded open-source
projects compete on, and none of it is what Cybou is for.

Do not let Mind be the thing that stops an agent. Cognition explains a containment; the kernel is the
containment. A design where a model notices misbehaviour and asks the agent to stop has no boundary
in it.

Do not reach for `CYBOU_PUBLISHABLE_SENSITIVITY` to solve anything. It was raised for one stated
reason — rows in an early Journal carried a constant sensitivity their content did not justify —
with a comment saying to remove it once those rows were gone. The rows were discarded that same day
and the raise outlived them, so the next thing above ordinary was published without anyone deciding
to. The default is `Ordinary` again. A temporary permission that survives its reason is the same
failure as a claim that survives its evidence.
