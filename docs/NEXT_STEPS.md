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

### A0. The executor

*Granted 2026-08-25, three adapters and no more.* Every stage before it is built: `cybou-remediation`
proposes over a closed set of typed operations, criticises each proposal against the finding it
claims to relieve, and decides it against a standing policy that grants nothing by default.

```text
service.status          read-only; exercises the whole transport before anything mutates
package.cache.clean     a bounded mutation with a clear outcome
service.restart         concrete .service units only
```

Nothing else is to be implemented. An operation absent from the code is a stronger statement than one
refused by policy, and an implemented adapter is still not a pre-authorized one — the standing policy
grants nothing by default and grants separately for this host and for an agent.

The executor speaks the systemd manager API over D-Bus. Not `sh -c`, not `Command::new("systemctl")`,
and **no general execution API at all**, not even a private one: an adapter is a function taking a
typed target and doing one thing. The `systemd:<unit>` placeholder is refused there — it means *some
unit, and this host cannot say which*, which is not something to carry out.

The first live S0 pass uses a harmless unit created for the purpose, not a database.

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

**Reordered 2026-08-25: the capsule comes before the agent.** Prove there is somewhere safe to put
any agent, and it stops mattering which one goes in first. That is also the shape of the claim — it
is not the agent that decides where its freedom ends; a human grant is turned into a physical
environment before the agent is started in it.

### B1. The Agent Capsule primitive — first, and the item everything depends on

```text
workspace · process namespace · filesystem namespace · network namespace
resource budget · model grant · MCP grants · secrets lease · lifetime · audit
```

Rootless first, behind a `CapsuleBackend` trait with a bubblewrap implementation, and no privileged
helper until a gate proves one is needed. In order:

```text
done  1. user + mount + PID + IPC + UTS namespaces
done  2. an empty filesystem, built up by explicit bind mounts — never a host / with things removed
done  3. Landlock as a second barrier: the mount says a path is absent, Landlock says no rights
done  4. PR_SET_NO_NEW_PRIVS before exec
done  5. seccomp for the syscalls that change the sandbox's own shape, not a brittle allow-list
done  6. no nested user namespaces
done  7. cgroup as the physical budget, including TasksMax; lifetime as the unit's lifetime
done  8. lease expiry freezes or kills the cgroup — never an ACP stop message
done  9. network deny-all: a fresh namespace with loopback and no route
     10. an egress broker for the granted hosts, because a grant is DNS identity and a firewall is not
```

Nine of the ten hold. **Step 10, the egress broker, is what remains**, and until it exists a
`NetworkGrant` naming hosts is compiled to a denial of all of them — named in the spec, not honoured,
and the gate checks the denial rather than the naming.

Two of the nine were not the tidying-up they looked like. Step 3 was not only defence in depth:
bubblewrap builds the capsule's root as a writable tmpfs, so until Landlock was applied an agent could
write to `/` — a real hole the mount namespace did not close, found because the gate went looking for
somewhere the two barriers could be told apart. And step 5 closed a declared debt by moving rather
than by solving: the filter was waiting for a file descriptor bubblewrap could be handed, and what it
actually needed was to be installed by the process that becomes the agent, which is where Landlock
had to go anyway.

Step 8 is done as two commands and not one: freeze, then kill. Not tidiness — a capsule under a task
ceiling can fork faster than signals arrive, and freezing ends that race before it starts. It is only
sound because `SIGKILL` reaches a frozen cgroup, which the gate rechecks on every run rather than
trusting, and the gate's own capsule ignores every signal it is allowed to ignore, so a build that
merely *asked* it to stop fails.

**The gate is adversarial and runs twice**, the second time with Mind stopped. Details in
[ADR-0042](adr/ADR-0042-agent-capsule-platform.md); the short version is that a capsule holding only
while Mind is watching has cognition for a boundary, which the ADR refuses in its first section.

`Reach` stays what it is: vocabulary for explaining, telemetry, audit and boundary crossings. The
grant is compiled **once** into a kernel policy and the kernel enforces it. Consulting Rust per
`open` would rebuild the runtime permission mediator this whole design replaces.

### B2. ACP client and registry browser

Speak ACP to an agent process, and list what the public registry offers. The registry is upstream;
Cybou does not maintain a catalogue of its own.

### B3. Standing capability lease

*Mostly done.* `Lease` carries the lifetime, the ledger and revocation, and a capsule ending is kept
apart from a model grant being spent. What remains is issuing one from a profile a person chose on a
screen, and an interface that then asks nothing while the agent stays inside it — one that asks
anyway has not made a weaker promise, it has made the grant meaningless.

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

One agent, end to end, inside a capsule, against a real provider — and only after B1's gate passes
twice. The candidate is the one with the widest existing provider support and a custom-endpoint
option, because it exercises the gateway without needing anything special from the agent.

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
