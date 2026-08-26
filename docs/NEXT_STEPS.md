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
than an organ, and a model that answers can only return proposals. **S0 is now held by the offline
walkthrough plus the live action gate**: one disposable service is authorized by configured policy,
restarted through the typed executor, and independently re-observed through systemd.

```text
observe → understand → remember → diagnose → explain → propose → authorize → act → observe outcome
```

Every stage exists and is tested. What an outcome is and how it is judged was built before the
executor deliberately, so the executor arrived to find its own report is one of two fields and not
the deciding one.

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

**Done.**

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

**Done.** One operation is authorized under an explicit standing policy, carried out against a
harmless disposable unit, and independently re-observed. The permit is short-lived and single-use;
the gate requires a replay to fail. The unprivileged Action1 service and root executor now export
separate names on the system bus under a closed D-Bus policy on the VPS, with a root-owned action
policy that starts empty and is changed only through the closed `cybou-action-policy` command.

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
done 10. an egress broker for the granted hosts, because a grant is DNS identity and a firewall is not
```

All ten hold. `cybou-egressd` decides by name, resolves for itself, rejects special and exact host
addresses, and is bounded by concurrency and time. `cybou-egress-bridge` is the capsule-local last
hop from an ordinary HTTP proxy port to one pathname socket; it copies bytes and owns no policy.
The bridge is forked by the entry program after Landlock and seccomp, counts inside `TasksMax`, and
the compiler refuses a brokered capsule too small to contain both it and the agent.

The gate exercises `curl` and `git` from a real capsule, denial outside the grant, direct no-route,
mapped/local-address refusal, another capsule's absent socket, bridge death and broker resource
amplification. B1 is complete. A0/A1 are complete too: `Action1` and the executor have a separate
live gate. The next implementation work is B2, the ACP client and registry browser.

The layering rule that keeps the two apart is now checked rather than remembered: a governance crate
that names `cybou-egressd` fails `validate-organ-layering.py`. `cybou-capsule` decides what an agent
may reach and `cybou-egressd` connects it, which is the `cybou-actiond` / `cybou-executord` split one
layer down.

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

**Done.** `cybou-acp` uses the official Rust SDK to negotiate stable ACP v1 over process stdio and
projects the agent identity, authentication methods and capabilities. Its read-only browser fetches
the canonical upstream index, bounds and validates the response, records when it was observed, and
searches it locally. It neither installs a distribution nor keeps a Cybou catalogue. The gate uses
a disposable fake peer and also requires an unsupported wire version to be refused.

Authentication, `session/new`, installation and running a real registry agent belong to the later
agent-pack/session stages; B2 deliberately grants none of them.

**A whole prompt turn now exists beside the handshake.** `AcpSession` runs `initialize`,
`session/new` and `session/prompt` against an agent process, collects every `session/update` in order
and keeps the agent's message apart from its internal reasoning — a surface that showed a thought as
an answer would be presenting a draft as a conclusion. The turn is bounded by a deadline its caller
supplies rather than by a constant here, because the caller is the one holding the lease.

`session/request_permission` is refused, and the refusal reaches the agent rather than only a log.
Every reference ACP client auto-approves, which puts the decision in the hands of the thing being
bounded. Cybou's answer is already given elsewhere: inside its capsule an agent needs no permission,
and outside it the answer is an `ActionProposal` a person decides. The protocol client can reach
neither, so the only honest answer it has is no — and each refused request is returned to the caller,
because an agent that keeps asking for something is a fact worth surfacing.
`scripts/test-acp-session-gate.sh` proves both against a stand-in agent that asks.

### B3. Standing capability lease

**Done.** `CapabilityProfile` holds the reusable limits a launch surface shows, while `LeaseRequest`
binds one explicit selection to a fresh capsule id, agent and workspace. The public mint validates
exact hosts and tools, rejects ambiguous or un-runnable profiles, compiles the exact resulting grant
before issuing it, and records the selected profile id on the `Lease`. It adds no ambient capability.

The standing-lease gate drives that public boundary and proves one selection stays silent for reads,
writes, execution, granted egress, mediated tools and model use inside the live lease. Rendering the
launch screen belongs with the first agent pack; the authority it must invoke is complete here.

### B4. The Model Gateway

**Done.** `cybou-model-gateway` provides the bounded OpenAI-compatible
`/v1/chat/completions` router beside `ModelBroker1`, not through it. Both surfaces meet at the same
registered provider workers, route policy and bounded usage ledger; Mind's closed `ModelTask`
vocabulary and D-Bus interface are unchanged.

The only credential accepted from a capsule is a freshly generated ephemeral token bound to one
live capsule lease, agent, task, model class, sensitivity policy, token ceiling and spend ceiling.
The gateway derives attribution and usage from the selected worker, charges the lease itself, and
refuses expiry, revocation, class changes and reservations that would cross a ceiling. The gate
drives both public request shapes through one fake worker and the real HTTP/auth/accounting path.

No host listener is opened implicitly. Binding the router to the capsule-only endpoint and injecting
the issued token are lifecycle work for the first agent pack; the multi-provider implementation
behind the worker interface is B5.

### B5. A multi-provider worker

One worker in front of a multi-provider proxy, so provider breadth is not this project's maintenance
burden. Behind an interface Cybou owns, so replacing it later changes nothing above it.

**Done.** `cybou-provider-litellm` implements only the provider-neutral `Worker` chat surface and
maps lease capability classes to operator-owned LiteLLM model groups. The gateway has no dependency
on this crate. For every request the worker uses its private proxy master key to mint one five-minute
virtual key scoped to that model group, the remaining spend budget and one parallel request; only
that virtual key reaches `/v1/chat/completions`, and cleanup is attempted after the response.

Cost is read from the proxy rather than trusted from the agent, converted from decimal dollars to
integer operator units with upward rounding, and checked again by the broker and lease. The usage
record carries the proxy model group, concrete deployment id, response model and call id, so the
proxy spend row can be joined without pretending a remote artifact digest was locally verified. A
fake-proxy HTTP gate proves the credential split, request ceilings, attribution and replacement
boundary. Registration requires a database-backed LiteLLM deployment with budget reservation enabled
and token pricing known for every mapped group; otherwise the proxy cannot reserve maximum request
cost before dispatch and the lease ceiling is not proven. No real provider, proxy deployment or
provider catalogue is implied; those are B7 and B6.

### B6. Provider catalogue with observation times

Free tiers exist and their limits change without notice. Cybou must not contain the sentence *this
model is free* — a catalogue entry with a timestamp, and a warning where a free tier carries a
condition a person would want to know before sending their code through it.

**Done.** `cybou-provider-catalogue` starts empty and parses only external schema-v1 observations.
Availability and zero-cost access are separate claims, each with UTC `observedAt`, `validUntil` and
credential-free HTTPS evidence. Data-use, payment-method, regional, quota and other material
conditions carry their own evidence and validity window. Expired claims remain displayable as stale
evidence but cannot make a provider eligible; future-dated, unsourced, non-HTTPS, duplicate or
malformed entries fail the entire snapshot.

Provider order is operator policy rather than catalogue data. Resolution accepts an explicit
preferred provider and explicit ordered alternatives and returns `Preferred`, `NamedAlternative` or
`Absent`, preserving why the preferred route was rejected. The gate uses only reserved `.invalid`
examples and proves the compiled default asserts no real provider fact. The observer that produces a
deployment snapshot and the first live route remain B7.

### B7. First agent pack

One agent, end to end, inside a capsule, against a real provider — and only after B1's gate passes
twice. The candidate is the one with the widest existing provider support and a custom-endpoint
option, because it exercises the gateway without needing anything special from the agent.

**Implementation and credential-free gate done; live provider gate remains.** The first pack pins
OpenCode 1.18.23 to the ACP-registry SHA-256 for each supported Linux architecture and starts its
official `opencode acp` entrypoint. A model grant compiles into a second capsule-local loopback
bridge backed by a private host Unix socket. The only authority mounted into the capsule is a
read-only ephemeral lease-token file; provider credentials stay in the host worker and never appear
in the pack configuration or process arguments. The Debian gate verifies the digest, ACP v1
handshake, no-route namespace, read-only installation and real capsule plumbing.

`cybou-agent-gateway` now owns the missing host lifecycle: one process registers the configured
LiteLLM worker, issues one token for one capsule lease and task, and exposes the router only through
that capsule's mode-`0600` Unix socket. Its systemd template consumes root-owned provider policy, a
root-only `LoadCredential`, and a short-lived launch file; it has no boot install target and is never
started by deployment. The credential-free lifecycle gate drives a real gateway token through a
fake LiteLLM peer and proves the proxy master key is absent from runtime artifacts.

`scripts/test-opencode-pack-live.sh` remains the non-optional completion criterion, and it now proves
the stronger of the two claims available. It drives the pack's own `opencode acp` entrypoint with
Cybou's ACP client rather than running `opencode run`: a green run means Cybou opened a session and
prompted the agent, not merely that OpenCode could reach the gateway on its own. It requires a real
provider's answer to come back over the protocol. The active VPS had no
operator-selected LiteLLM deployment or credential at the last inspection, so that gate remains
visibly `NOT RUN` and B7 is not marked Done. The exact fail-closed operator contract is documented in
[Per-capsule model gateway deployment](agent-gateway-deployment.md); provisioning a provider is an
operator decision rather than a secret silently copied by this repository.

### B7a. One owner for one session

**Derivation done; carrying it out remains.** Every part of a session existed and none of it was
owned, which is how the gateway came to rebuild its own lease from environment values and produce a
second authority beside the approved one. `cybou-agentd` is the single owner: one selection becomes
one lease, and the capsule spec, the lease file, the model token and the clock are all derived from
that one object. `cybou-agentd plan` prints every file, unit and teardown step a launch implies
without touching a host, and its gate asserts the two properties that matter — every runtime name
carries the session identity, and the launch file names nothing that is authority.

`cybou-agentd launch` carries the same plan out: it writes the lease and the launch file, starts this
capsule's egress broker and its private gateway, runs a program inside the capsule under its cgroup,
and tears the session down in the planned order including when the way up failed. The privileged step
— starting the system gateway unit — is delegated by a polkit rule granting start and stop on exactly
that unit-name shape to exactly the `cybou` user, and nothing else; the reasoning and the rejected
alternative are in [The agent session owner](agent-session.md).

`scripts/test-agent-launch-gate.sh` proves a launch and its teardown on a deployed host and is
visibly `NOT RUN` anywhere without a gateway template, a configured provider and a user service
manager. What remains is driving an ACP agent rather than a program, which is B7's remaining half,
and withdrawing a running lease from outside, which is B11.

### B7b. Free is a selection, not an empty budget

**Done.** The provider catalogue distinguishes availability from zero-cost access, and none of it
could be used: a grant said *spend nothing* with the integer nought, and the transport read that as
*this capsule has spent everything* and refused. One number carrying two opposite facts, with every
component downstream guessing which one was meant — and the single selection a person makes in order
to use a free model was the one selection the system could never serve.

`SpendPolicy` says which. `Capped(n)` is a ceiling; `ZeroCostOnly` is a hard routing constraint —
only a route an operator has *declared* to cost nothing may serve it, `--spend-limit zero-cost` is
not `--spend-limit 0`, and a route that was declared free and then bills has broken a promise rather
than used up a budget. That failure is refused rather than returned: handing back an answer somebody
has now been charged for, having asked for none, would make the policy cosmetic.

The declaration is `CYBOU_LITELLM_ZERO_COST`, exactly `yes` or `no`, with no default. Only an
operator knows what their deployment charges; a default either way would be Cybou deciding on their
behalf — one direction silently forbids the free models a person selected, the other silently spends
their money.

### B7c. A stream, so an agent can run at all

**Protocol done; incremental delivery is B8.** The gateway refused `stream: true`, and that was not
"no streaming yet" — a coding agent asks for a stream and treats a refusal as a broken endpoint, so
it was an agent that could not run. `/v1/chat/completions` now answers with a real
`text/event-stream` in the shape every OpenAI-compatible client expects.

What it is not, said here because a client cannot tell: the completion is produced whole before the
first byte leaves. Nothing arrives sooner. The upstream request to the provider is still not itself a
stream, and the gate asserts that — the boundary stays where it was.

Doing it in this order is the safe direction rather than the lazy one. The lease is charged before
any of the response is sent, so a completion that would exceed the ceiling is refused while refusing
is still possible. Delivering tokens as they arrive means charging as they arrive, and a ceiling
reached mid-sentence cannot be honoured by unsending what has already gone. That needs a mid-stream
cancellation design, and it belongs with B8's live session rather than in the compatibility adapter.

### A2. What was authorized outlives the process that authorized it

**Done.** `Action1` held proposals, criticism and decisions in memory. For the first vertical that was
enough; for a host that acts on its own — and far more for one that lets an agent ask it to — it is
not. The question a person asks a month later is not *what is Action1 holding*, it is *why did nginx
restart on the fourteenth*, and answering it means the proposal, the objections and the decision are
still there. A restart destroyed all three, which made the causal chain a property of a process's
uptime.

Nothing was invented to fix it. The Journal's contribution kinds already lined up with the lifecycle,
so an action is ordinary Journal content rather than a private log beside it: `PlanProposal` for what
was proposed, `Objection` for each criticism that failed, `Decision` for what was decided, all sharing
the proposal's identity as their correlation and each citing the step before. A reader following
causation arrives at the decision from the proposal without needing to know Action1 exists.

The permit is deliberately not written. It is a single-use sixty-second capability, and a durable
record of one would be a durable record of a key — worse, one whose presence could be mistaken for
the authority itself. Losing permits on restart is correct: an authorization nobody claimed was not
claimed, and restoring one would be a permission reissued by a crash.

`cybou-actiond` reads its own history back before it answers anything, and a Journal it cannot reach
leaves it empty rather than stopping it — a host that will not repair itself because it cannot
remember is worse than one that has forgotten. Recording is best effort for the same reason, and the
failure is printed rather than swallowed.

**It did not work when it was written, and running it is what found that.** The proposal was recorded
as a root contribution — one citing nothing — on the reasoning that nothing Action1 can name caused
it. The Journal admits only `Observation` and `ContextDisclosed` as roots; everything else is derived
and must cite a cause that *exists*. So every contribution was refused, every time, and because
recording is best effort the refusal went to stderr and nothing else changed. The tests could not see
it: they checked the shape of the envelopes and never handed one to a Journal.

The proposal now cites the finding that gave rise to it, which `ActionProposal::cause_id` has always
carried, and a proposal with no cause is refused up front rather than submitted to be rejected.
`scripts/test-action-durability-gate.sh` runs a real `Event1` and a real `Action1`, decides an action,
kills the owner, starts it again and asks it what it authorized. `Action1` gained a `Record` method
for that question: a durable history nothing can read is a history only in the sense that the bytes
exist.

**One dependency is still open, and it is load-bearing.** Nothing in this repository puts a
`SystemInsight` into the Journal, so on a real host the cause a proposal cites is not there and the
lifecycle is still refused — now with a legible reason rather than silently. Journalling findings is
what makes this work outside a gate, and it belongs to the organ that observes them.

### B7c. An owner that outlives its sessions, and one that they outlive

**Partly done.** `cybou-agentd serve` holds what is running and answers on
`org.cybou.Runtime.Agent1` — `Runtime` rather than `Mind`, because an agent runtime starts and holds
software Cybou did not write and does not trust, and a bus name under `Mind` would assert the
opposite of that in the one place an operator looks.

Recovery is the part that had to be right. A capsule and a gateway outlive their coordinator on
purpose, so an owner that restarted and reported nothing running would be wrong about the host in the
direction that matters: a working agent, unwatched, with no surface offering a way to stop it. The
registry is therefore a reading of the host rather than a memory of what a process started, and every
session is re-derived through the same `plan()` a launch used.

The surface is `Sessions`, `Session` and `Stop`. `Launch` is not on it: a CLI launch is bounded by
who can run it, and a bus method is not, so it arrives together with a registry of operator-approved
profiles and takes a profile id rather than a set of ceilings.

The model gateway publishes a typed `ModelUsageSnapshot` beside its socket and the owner reads it, so
a listing reports a real figure together with the instant it was observed rather than a spend of
*unknown* — and never a nought nobody measured.

The profile registry `Launch` needs now exists, and `cybou-agentd start` is the door that uses it: a
caller names a profile, an agent, a workspace and one of the models that profile offers, and every
bound comes from a file only root can write. Deployment creates it empty, so a host offers nothing
until an operator writes something.

What remains: `Launch` itself on the bus, and a record of finished sessions a person can still read. See [The agent session owner](agent-session.md).

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
