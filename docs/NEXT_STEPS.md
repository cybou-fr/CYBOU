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

On 2026-08-26 the digest-pinned OpenCode 1.18.23 artifact was installed in the development WSL host
and the complete credential-free pack gate passed: the real `opencode acp` entrypoint started inside
a model-granted capsule and completed Cybou's ACP handshake through the private Unix channel. The
same host had no provider policy, LiteLLM master key, approved profiles or aggregate capacity file;
the live gate therefore returned `NOT RUN` before a model request, and no real-provider claim is
added here.

### B7a. One owner for one session

**Done.** Every part of a session existed and none of it was
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
manager. ACP prompting is now the owner's ordinary model-backed path, including launches accepted by
`Agent1`; the remaining B7 evidence is one answer from a real provider. Confirmed immediate Stop is
available through the owner; quarantine, revoke and freeze remain B11.

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

**That dependency is now closed.** `cybou-telemetryd` puts a finding in the Journal together with the
readings it rests on, so the cause a proposal cites is there and the chain holds end to end:

```text
readings   Observation    what the host measured
finding    Hypothesis     citing them
proposal   PlanProposal   citing the finding
objection  Objection      per criticism that failed
decision   Decision
```

A finding is a `Hypothesis` and not an `Observation`, which `SystemInsight` had said all along: what
was observed is the readings, and that they add up to *the database stopped because the disk filled*
is an inference — one recorded as an observation would be a claim the host cannot support.

Only the readings a finding cites are written. Every reading a host takes is transient and belongs
nowhere near a biography; a Journal holding all of them is a metrics database, which is the thing this
system has said it is not. What a finding rests on is the answer to *why do you think that*, and an
inference nobody can trace back is indistinguishable from one a model made up — which is why the
Journal will not hold a `Hypothesis` citing nothing, and why a finding that cites nothing is refused
here rather than submitted to be rejected.

Once per finding rather than once per sample: a finding's identity is derived from what it is about
and when it began, so an ongoing problem is the same finding ten seconds later.

The durability gate now drives that same path rather than a hand-built envelope, so what it proves is
that a finding and a proposal about it form one chain a real `Event1` accepts — not that one file can
satisfy the admission rules.

### B7c. An owner that outlives its sessions, and one that they outlive

**Done for ownership and visibility.** `cybou-agentd serve` holds what is running and answers on
`org.cybou.Runtime.Agent1` — `Runtime` rather than `Mind`, because an agent runtime starts and holds
software Cybou did not write and does not trust, and a bus name under `Mind` would assert the
opposite of that in the one place an operator looks.

Recovery is the part that had to be right. A capsule and a gateway outlive their coordinator on
purpose, so an owner that restarted and reported nothing running would be wrong about the host in the
direction that matters: a working agent, unwatched, with no surface offering a way to stop it. The
registry is therefore a reading of the host rather than a memory of what a process started, and every
session is re-derived through the same `plan()` a launch used.

The surface is `Sessions`, `Session`, `Launch` and `Stop`. `Launch` takes a selection — profile,
agent, workspace, model class and prompt — but no ceilings. The owner reads the root-owned registry
of operator-approved profiles itself, derives every grant from that profile, and admits the promise
against all sessions live on the host under the registry lock before it starts anything.

The model gateway publishes a typed `ModelUsageSnapshot` beside its socket and the owner reads it, so
a listing reports a real figure together with the instant it was observed rather than a spend of
*unknown* — and never a nought nobody measured.

The profile registry exists, and both `cybou-agentd start` and the owned `Launch` path use it: a
caller names a profile, an agent, a workspace and one of the models that profile offers, and every
bound comes from a file only root can write. Deployment creates it empty, so a host offers nothing
until an operator writes something. Deployment also creates an explicit zero-capacity host policy;
an operator must choose aggregate totals before the reachable launch surface opens.

Confirmed endings leave live admission immediately and remain visible as the most recent 32 final
views held by that owner process. The record is deliberately not reconstructed after restart: the
host can prove what is still running, but it cannot prove why an already-gone unit ended. Durable
agent biography remains separate future work. See [The agent session owner](agent-session.md).

### B7d. Four things a reachable surface must not get wrong

Found by review of what had been built rather than by running it, and each is the same shape: a
component that reported success for an outcome it had not established.

**A spent grant reached a provider.** The gateway checked the lease's clock and class before
dispatching and never asked it about money, so a capped grant with nothing left, and a zero-cost route
that had already broken its promise once, both went on calling providers — the refusal arriving after
the bill. It now asks `may_use_model` before a provider is called, which closes a violated zero-cost
route to everything after it rather than to nothing.

**`Stop` forgot sessions it had not ended.** It removed the session from the registry and tore it down
afterwards, so a capsule that refused to die became one nobody could see or stop. A session now leaves
the registry only on a confirmed ending; an unproven one stays listed as `ending` and the caller is
told `false`.

**A usage snapshot was believed whatever session it named.** It carries a capsule id; one naming a
different session is now ignored rather than attributed here.

**Aggregate host capacity now exists.** A profile bounds one capsule and bounded nothing else, so four
honest four-gigabyte grants fitted on an eight-gigabyte host and every one of them was within policy —
each session correct, the host oversubscribed, and no single grant able to show it.

`HostCapacity` bounds sessions, memory, CPU, processes and a spending envelope across everything live.
It decides against what has been **promised**, never against what is being used: a session admitted
because the others happen to be idle is a promise the host cannot keep the moment they are not. The
module cannot see usage at all, which is the point — it could not be tempted. The consequence is worth
saying plainly, because it will look like a bug: this refuses launches on a host that appears half
empty.

The session count is its own limit rather than something implied by memory, because sessions cost more
than their ceilings — units, sockets, brokers, gateways — and many small capsules can make a host
unusable long before any of them touches a byte of what it was promised. A zero-cost session reserves
no money, or free models would be the scarcest thing on offer.

Deciding and taking are one call: `SessionRegistry::admit` checks and inserts under one lock, because
two callers that each ask *is there room* and then each take it are both told yes. Recovered sessions
are admitted whatever the numbers say — they are already running, and refusing them would not stop
anything, only hide capsules that exist.

An absent capacity file means unbounded, which is what every earlier version did, named rather than
defaulted into. A file that exists and cannot be read means nothing is admitted: a limit an operator
believes is in force must not be read as no limit at all. `Agent1.Launch` refuses unbounded capacity
too: a reachable mutation cannot silently inherit the historical no-limit mode. Deployment writes
an explicit zero-capacity policy, so opening launches requires an operator to choose real totals.

**`Launch` now binds admission where it can be atomic.** `Agent1` prepares the profile-derived plan,
checks and inserts the live promise in one registry operation, and only then starts the capsule. Two
requests cannot both take the last slot. An immediate start failure rolls the reservation back; once
startup is owned by the background task, that task advances the shared session and moves it from
live admission to bounded final history only after teardown.

### B7e. What a browser is told about running agents

`GET /api/v1/agents` returns what `Agent1` says, and the type it returns
lives in `cybou-protocol` rather than beside the owner — so the owner and the browser share one
definition instead of two that agree on the day they are written. The route is a proxy and
deliberately nothing more: it does not read the launch directory, ask a service manager, or assemble
a session from a lease and a plan. A second thing doing that would be a second answer to *what is
running*, and the one that is not the owner's is wrong the moment a session starts or ends between
its listing and its reading.

`scripts/test-agent-card-gate.sh` compares the endpoint's answer against the owner's own, field for
field, because *looks like a session* is exactly what a second assembler would also produce. Then it
stops the capsule through the HTTP route, requires the unit to be gone, and compares the retained
ended view against the owner again. Finally it kills the owner and checks the endpoint says it could
not ask — an empty list there is the one answer a person cannot act on, since *nothing is running*
and *I could not find out* look identical on a card and only one means they can stop worrying. The
refusal names the condition without describing this host's insides to somebody who may not be
entitled to know them.

`POST /api/v1/agents` is the launch proxy. It accepts only a local desktop seat or an authenticated
session and refuses public preview before touching D-Bus. It carries the selection whole to
`Agent1`; it does not read profiles, derive ceilings, or preflight capacity. The owner performs those
steps and returns the canonical session already reserved as `launching`. This is authentication at
the HTTP boundary, not a claim of per-method D-Bus identity: processes already running as the
`cybou` service user share that user's bus authority and must still be contained as such.

`DELETE /api/v1/agents/{capsule_id}` is the Stop proxy. It accepts the same local or authenticated
seat as Launch. A confirmed or already-ended session returns `204`; if `Agent1.Stop` cannot prove
teardown, the gateway re-reads the owner's canonical listing and returns retryable `409` while the
session remains live. The gateway never reports an unconfirmed ending as success.

The canonical `Agents` card is now drawn in Living Canvas from that same `SessionView`. It names the
agent, profile and workspace; distinguishes starting, running, ending and ended; presents promised
memory, CPU and task ceilings as ceilings rather than invented usage; names the exact allowed hosts;
and timestamps model spend at the instant it was observed. Runtime unavailable and zero running
agents remain different answers. The card also submits the bounded selection to the launch proxy and
adds the owner's returned `launching` session immediately; resource authority never enters the form.
While that session is live it refreshes from `Agent1`, so `running`, newly observed spend and the
final ended reason replace the launch receipt rather than leaving an optimistic state on screen. A
manual refresh covers longer sessions after the bounded polling window. Every live row offers Stop;
after confirmation the card re-reads `Agent1` and presents the retained final view.

### A2b. The episode runs to what the host saw afterwards

Durable authorization answered *why was this allowed*. It could not answer *was it done, and what did
the host independently see* — and those are the two halves of the only question worth asking a month
later.

The episode now continues:

```text
ExecutionStarted   Intention   when the effect first became possible
ExecutionAttempt   Intention   what the executor finally reported
ActionOutcome      Outcome     what it independently saw afterwards
```

Both execution records are `Intention`s because the Journal has no kind for *acting*. The first is
written synchronously by `Action1.ClaimPermit` before it returns the typed action, and therefore
before the executor can touch the Body. The second is the executor's final report. An outcome is an
`Outcome`, which the Journal treats as terminal and permits once per cause — right for an action,
which happens once and is answered for once.

They are two contributions and not one. What a thing says about itself and what the readings say
afterwards are separate accounts, and the entire value of re-observation is that they can disagree;
folding them together would delete the disagreement, which is the only part that could ever surprise
anybody. A test holds exactly that case: the executor reports completion, the service is still down,
and both survive the restart.

A decision nobody acted on stays one. Absent is a real answer and a common one, and filling it in
would answer *was it done* with a guess.

**And now something produces them.** `cybou-remediationd` is the join: it reads what `Telemetry1`
concluded, asks [`initiative`] whether this host may act, proposes to `Action1`, hands the opaque
permit to the executor, waits out `TOO_SOON_AFTER`, asks telemetry what it sees now, and reports the
outcome. Executor1 reports its own attempt directly to Action1; the coordinator is no longer a
courier between the thing that acted and the lifecycle owner. Until it existed the only thing that had ever run that loop was
`action-roundtrip`, an example written for a gate — so a host left to itself reached *explain* and
stopped, which is not what this repository's own summaries have been saying.

It takes no bus name. It offers nothing to anybody, and a surface here would be a second place to ask
about actions when `Action1` already owns the lifecycle and answers for it. It sits in the governance
layer beside `Action1` for a structural reason rather than a tidy one: an organ may read the layers
above it and not the ones below, and this must read telemetry *and* call authorization. Anywhere above
governance it would be reaching downward. Being a peer of the gate rather than above it is also the
honest description — it is a party that asks, not one that decides.

`scripts/test-self-maintenance-gate.sh` proves it, and proves it the only way that means anything:
four daemons, a harmless unit stopped, and then nothing. No script touches `Action1` or the executor
after that point. The host notices, proposes, is permitted by a standing policy an operator set,
carries it out, waits, looks again, and concludes — and the gate checks the log to be sure the unit
came back because *this host restarted it* rather than for any reason at all.

The same gate covers both restart boundaries. It kills the driver after execution but before outcome
and requires the new process to finish the inherited episode without executing again. It also leaves
a service broken until the remedy is concluded `StillPresent`, restarts the driver after that terminal
outcome, and requires zero new executor attempts. The second case is distinct: terminal episodes are
not returned by `UnfinishedEpisodes`, so the driver asks `Action1.EpisodeForCause` before treating its
empty process memory as permission to act.

There is a third, narrower boundary before both of those. `ClaimPermit` consumes the capability,
mints `ExecutionStarted`, and requires Event1 to durably accept it before the typed action is returned
to Executor1. If the Body effect then happens but the executor process or D-Bus reply disappears,
Action1 still recovers that stable attempt as `DidNotFinish`. The adversarial test holds the exact
sequence `effect happened → final report lost → Action1 restart → zero permission to repeat`.

Running it found two defects in the first two attempts, both of the kind that reading could not have
shown. It decoded the telemetry organ's answer as a fabric envelope, which is a convention that organ
does not use — the driver had assumed one rather than reading the one in use. And it proposed the
first entry in the remedy table, which is an inspection: `relieves()` says plainly that reading a
unit's state relieves nothing, so a host doing that would look at a stopped service, learn it was
stopped, and repair nothing.

The second fix has a second half worth stating. Proposing only the gentlest remedy and stopping at a
refusal would make an authorization unusable unless everything gentler was authorized too — an
operator who permits a restart and nothing else would have granted something their host could never
reach. So it walks the operator's own order and takes the first remedy that is *permitted*, which is
not escalation past anybody's decision: `Action1` refuses every step the operator did not authorize.

Four things stop it doing something rash, and none of them is the file being careful. It cannot choose
an operation: the remedies for a finding are a closed table, ordered least committal first, and it
takes the first. It cannot authorize itself: every proposal goes to `Action1`, so on a host where
nobody pre-authorized anything it proposes and is refused every time — and the refusal is recorded,
which is the useful part, because an operator can then read what their host wanted to do. It cannot
act twice on one finding. And it cannot conclude success: what it carried out and what the host saw
afterwards are gathered separately, the second from the organ that did not carry it out and has no
notion that anything happened.

That is the next thing worth building, and it is a bigger piece than recording: something has to
decide when a finding deserves an action, wait out the re-observation delay, and answer for the
result. Until it exists, the durable episode ends at the decision on any real host, which is what it
already did — the difference is that the record can now hold the rest when there is a rest to hold.

**The decision that driver would otherwise make by accident now exists.** `cybou_remediation::
initiative` answers, for one finding and what was already tried about it: act, wait, or leave it
alone. Written before the wiring, for the reason this crate's own header gives — the natural shape of
that code has no place for the decision to live, so it gets made by whoever joins a proposer to an
executor, on a working system, under pressure.

Three of its answers are the ones a retry loop would get wrong:

A finding is present *while the remedy is taking effect*. A restart takes longer than a sample
interval, so acting on presence alone restarts a service, looks, still sees it down, and restarts
again — the loop that turns a self-maintaining host into an outage.

A remedy that ran, was observed, and did not relieve the finding is evidence about the remedy, not
about the effort. Trying again is what something does when it cannot tell *not yet* from *not this
way*, and this is exactly the point where a person is genuinely needed — the point a retry loop hides.

*We could not see* is not *it did not work*. One needs a remedy and the other needs somebody to fix
the looking, and an attempt whose end nobody knows is not repeated at all, because doing it again is
the wrong answer to *something may well have happened*.

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

**Implementation complete; deployed-host proof pending.** Agent1 now serializes Freeze, Resume,
Quarantine and Stop through one per-capsule
control gate. A transition is published only after a second kernel read confirms `cgroup.freeze`
while that gate is still held, so concurrent callers cannot leave the registry saying `Paused`
after a later thaw made the capsule runnable. The gate is covered by a concurrent regression test.

Quarantine now stops and verifies both the user-owned network broker and the system-owned model
gateway. It also verifies that systemd removed the gateway socket and ephemeral bearer with its
`RuntimeDirectory`; a partial revoke is reported as `Paused`, never `Quarantined`.

`scripts/test-agent-control-gate.sh` now launches the complete disposable host fixture, proves
process progress stops and resumes, proves both egress paths and the model bearer are revoked by
quarantine, then stops the session and proves every unit and authority file is gone. It is wired into
the main gate and returns `3` rather than passing when the deployed gateway/provider/polkit boundary
is absent.

Do not call B11 done until that gate has passed on the deployed Debian host.

### Personal Core privacy boundary

**Multi-user isolation closed at the current gateway boundary; owner extraction remains.** Every
Mail, Calendar, Notes and Contacts route now requires the numeric UID established by the
authentication owner. Personal records are partitioned by that UID in SQLite, and every mutation is
one database transaction. A browser cannot supply a principal, an unauthenticated request is
refused, and an endpoint-level regression proves that a note written by UID 1000 is absent from the
projection returned to UID 1001.

The old process-wide JSON store is no longer a production source. Do not automatically assign its
unscoped records to the first account that signs in: those rows have no trustworthy owner and need
an explicit operator migration decision.

**`cybou-personald` now exists and the gateway is its proxy.** One instance per admitted Linux
account, started as that account, holding that account's records in a SQLite database under their own
home, answering on a per-UID socket the gateway reaches the way it already reaches the host-files
owner. Its protocol carries no UID field at all: the process identity *is* the partition, so there
is no request a caller could form that names somebody else's mailbox, and no code path that could
answer from another store. It refuses to start as root, because one root process owning everybody's
personal records is the arrangement this daemon exists to end.

Where `CYBOU_PERSONAL_SOCKET_DIR` is configured the gateway holds no personal records of its own; an
account whose owner is not running is reported unavailable rather than as an empty mailbox. Without
it, the in-gateway UID-partitioned store remains as the fallback: not a privacy regression, simply
not owned by the person whose records it holds. `scripts/test-personal-owner-gate.sh` proves the
separation against real processes and real sockets — one account's note is unreachable through
another's owner, is absent from the other's store on disk, and survives its own owner restarting.

The old process-wide JSON records still have no trustworthy owner and still need an explicit
operator migration decision rather than being assigned to whoever signs in first. Real Mail and
Calendar providers remain the next step, and `send_mail` keeps refusing until one exists.

### Cognitive Graph grounding

**The false grounding and duplicate Journal are removed.** Cognitive nodes and edges now carry a
typed provenance (`Observed`, `Configured`, `Architectural`, `Derived`, or `Inferred`), evidence IDs,
and an optional observation instant. Live systemd and `/proc` nodes are observed; daemon relations
are architectural; Mind beliefs are derived. The desktop inspector renders that provenance instead
of labelling every node “Observed”.

The gateway no longer creates `/home/demo` or `/etc/cybou` nodes without a reader, and no longer
asserts that every belief derives directly from Event1 without evidence. An input containing no
services, processes or Mind projection now produces an empty graph.

The graph query contract now does what it declares. `nodeTypes` and `maxDepth` were accepted and
ignored, so a control could constrain nothing while appearing to work. A query now selects starting
nodes by term (or by an explicit `focusId`), constrains categories, and walks a typed breadth-first
expansion exactly as far as the requested depth; a type constraint is not widened by traversal, and
an edge is returned only when the projection contains both of its endpoints.

`CognitiveHub` physically contains no Journal. `/api/v1/cognitive/journal` projects the canonical
Event1 view obtained through Presence and fails explicitly when that owner cannot answer.

`evidenceIds` are now real. Epistemic1 already named the contributions each belief was formed from,
and the gateway was dropping them on the way to the web contract, which is what left the graph
unable to say where a belief came from. The belief projection carries them and derived nodes cite
them. An owner that accounted for nothing still cites nothing, and the desktop says "none accounted
for" instead of the "0" that read like a count of something absent.

### Meaning1 ownership

**Gateway cognitive ownership removed.** `cybou-web-gateway` no longer calls `interpret()` or
`realize()`, constructs response plans, invents fallback interpretations, or stores a `Dialogue`.
Its Meaning hub is now a stateless D-Bus client. If Meaning1 or Event1 cannot accept an utterance,
the HTTP boundary refuses or reports the owner unavailable rather than producing a plausible local
answer that no canonical owner holds.

Meaning1 now owns the complete vertical: interpretation, Event1 admission, response planning,
deterministic realization and bounded referent memory. Dialogue state is partitioned by the
server-established principal supplied by the gateway, so two authenticated Linux accounts do not
share referents. The browser cannot choose that principal in its request body.

The remaining proof is to extend the deployed multi-daemon gate through the HTTP routes as well as
the existing direct Meaning1 calls, including two-principal dialogue isolation and fail-closed
behaviour while Meaning1 is absent.

### Learning evidence authority and durability

**Browser-authored lineage is removed.** A learning proposal contains only the layer,
generalization and scope. It cannot submit episode or outcome identifiers. New candidates therefore
start with empty evidence, and evaluation resolves both evidence sets exclusively from the
owner-held `DemonstratedOutcome` records before applying the promotion gate. Artifact lineage is
derived from that resolved evidence, never from a caller assertion.

Learning state is now one locked transaction image rather than four independently sampled vectors.
Proposal, evaluation and revocation first serialize, flush and atomically rename the complete next
image; only then does the in-memory state change or the HTTP request succeed. Serialization and I/O
failures are explicit retryable server errors, and a regression test proves failed persistence does
not publish a RAM-only candidate.

**The evidence now has a real producer.** Until now nothing ever wrote a `DemonstratedOutcome`, so
the resolver was correct and empty: a safe skeleton rather than a learning pipeline. Demonstrations
are now derived, at evaluation time, from the canonical Action1 records the host actually holds.
One proposal is one episode, so two outcomes of the same proposal are one occasion rather than two.
A record contributes only when it falls inside the candidate's scope — matched by whole dotted
segments against the operation verb, or exactly against the target, never by substring, because
`service.restart` must not claim the evidence of `service.restart-preflight`. An outcome whose
effect was never established, or where the executor's claim and the telemetry disagree, is evidence
of nothing and counts as neither a success nor a failure.

Because demonstrations are derived rather than accumulated, evidence Action1 no longer establishes
stops supporting a promotion: re-evaluating the same candidate against a record set that lost those
episodes refuses it again. And when Action1 cannot be read at all, evaluation refuses with `503`
instead of falling back on the demonstrations resolved last time — a promotion granted on a memory
of evidence is one nobody can check now.

The remaining architectural step is to move the same owner/resolver and durable transaction boundary
behind Learning1, leaving the gateway as an authenticated proxy as was done for Meaning1, and to add
producers beyond Action1 (agent task outcomes and Operation1 terminal records are the obvious next
two).

### Operation1 ownership

**The gateway is no longer an operation owner.** Its in-process operation and log collections, and
the local `cancel()` that merely painted a record `Cancelled`, have been removed. The HTTP routes
are stateless D-Bus clients of `org.cybou.Runtime.Operation1`; an absent owner is unavailable rather
than an empty successful operation list.

Operation1 registers a real worker together with a cancellation watch token. A cancel request only
signals that token. It deliberately leaves the record running until the worker reports its actual
terminal state, so a requested cancellation is never projected as a completed cancellation. That
distinction is now typed rather than implied: `CancellationAccepted` and `CancellationConfirmed` are
separate outcomes, the record carries `cancellationRequested` so every reader sees *cancelling*
rather than *cancelled*, the HTTP boundary answers `202 Accepted` for a recorded request and `200 OK`
only for a confirmed teardown, and Living Canvas reports "cancellation requested" until a worker
publishes the ending.

Lifecycle state and observation state are also separate. A record carries `observation`
(`Known`/`Stale`/`Detached`/`Unavailable`) and `lastObservedAt` beside its lifecycle state. Restored
records start `Stale`; agent operations Agent1 no longer establishes become `Detached` while keeping
the last state a worker actually published; an unreadable Agent1 makes them `Unavailable`. No record
can sit at `Running` forever on the strength of a memory, and none is given an ending nobody
observed. The desktop paints an indeterminate bar when the owner reports no percentage, because
unknown is not zero.
The typed notification cancel shortcut now dispatches to Operation1 and reports success only after
that owner signals the worker cancellation token. Custom actions still refuse instead of returning
an invented “Executed” outcome. Operation records, logs, notification state and both mutation
surfaces now require an authenticated Mind-readable session rather than being public routes.

The daemon and user unit are wired into the workspace, deployment binary set and Mind target startup
(as a dependency, while its D-Bus namespace correctly remains `Runtime`). Operation records and log
entries are transactionally stored in a SQLite WAL database and restored when the owner restarts.
Cancellation intent is durable too: a request accepted while a local worker is detached survives
another owner restart, and `reattach` returns a token already set to the pending value. Only the
worker's subsequent lifecycle update publishes `Cancelled`.

The first real external producer is Agent1. Operation1 reconciles its canonical session views every
two seconds into stable, deterministic operation identities. Agent progress remains indeterminate;
the phase comes from Agent1 rather than a simulated percentage. Completed, stopped and failed agent
sessions remain distinct. Cancelling one dispatches the typed Stop call back to Agent1 and publishes
`Cancelled` only after Agent1 confirms teardown.

Reconciliation that finds no semantic change now refreshes observation freshness in memory only, so a
steady fleet of agents costs no durable writes every two seconds. The durable transactions that do
run now execute on a blocking worker rather than on the async executor, so a `synchronous=FULL`
commit waiting on the disk no longer stalls every other caller of this owner.

**Action1 is the second producer.** Every action that has crossed the durable execution boundary is
now one operation, with an identity derived from its proposal, so the Operations Monitor shows the
system work a host actually does rather than agents alone. A proposal still waiting on a decision is
not included: that is something asking for attention, not something running. The lifecycle follows
the attempt — whether the work ran is the first question — while the independent outcome is reported
as the step. Three distinctions are kept that a single "failed" would have flattened: a refusal never
ran and is published as `Refused` rather than as a failure; an attempt that began and whose ending
nobody knows keeps its last lifecycle state and is marked `Detached`; and an executing permit offers
no cancel button, because Action1 has no way to recall one and a button that does nothing is the
thing this owner exists to stop. Each producer is reconciled and marked independently, so one owner
going quiet never detaches the other's operations.

Retention is enforced in the same SQLite transaction as registration or lifecycle update: every
active operation is retained, only the newest 100 terminal operations remain, and each operation
keeps its newest 500 log entries. Startup applies the same limits to an older database.

The deployed continuity gate now proves that a real Agent1 session keeps one Operation1 identity
through both gateway and Operation1 restarts. Living Canvas re-reads that owner every two seconds
while the Operations card is visible, reconnects after a transient gateway failure, and restores
the selected operation and its logs by stable identity. The remaining work is more producer
adapters and wiring each local producer to the reattachment contract. Until those exist, this is an
honest durable owner boundary rather than a complete operations substrate.

### Notification audience isolation

Notifications carried no audience. Every authenticated seat read one process-wide collection, and a
"dismiss all" from one account reached every other account's items. No producer had filled that
collection yet, so this was a latent disclosure rather than a live one — and the right time to fix
it is before mail, calendar, agent and personal producers start writing into it.

Each notification now names its audience: `Operator` for notices about the host, which reach every
authenticated seat, or `Principal` for one account's own work, which reaches nobody else. The routes
resolve the server-established principal instead of merely asserting that someone is authenticated.
Listing shows only what that principal may see, "dismiss all" means all of theirs, and acting on
another principal's notification is *not found* rather than refused, because a refusal would itself
disclose that it exists.

Persistence and the producers themselves are still open, and per-principal notifications should move
to a canonical owner when they carry real personal content rather than living in the gateway.

### Real system surfaces

**Packages and Network are read, not declared.** Both surfaces reported `Unknown` and an empty list,
and the desktop said so honestly — "no package database reader is implemented". Two readers now
exist, both read-only and neither needing any privilege the gateway did not already have.

Packages come from `/var/lib/dpkg/status`: only entries dpkg itself calls installed, sorted by name,
verified on a live host against `grep -c '^Status: install ok installed'`. Nothing consults the
repositories, so no package is reported upgradable and `upgradableCount` is now `Option`, left
unestablished rather than reported as zero — "up to date" and "did not look" are different answers.
A candidate version is left absent for the same reason, and the originating repository stays empty
because dpkg records what is installed, not where it came from.

Network comes from the kernel's own accounting: `/sys/class/net` for interfaces, link state and byte
counters, `/proc/net/route` for the default gateway of each interface, `/etc/resolv.conf` for
nameservers. An interface the reader cannot classify is `Other` carrying the host's own word for it
rather than being called Ethernet, and an address nothing established is `None` rather than a blank
that reads like an address. Where the kernel cannot be read the surface stays `Unknown`.

Users and Storage are read too. Accounts come from `/etc/passwd` and `/etc/group`: only identities a
person can sign in as, with administrator status decided by real membership of `sudo`, `admin` or
`wheel`. Whether an account is locked lives in the shadow database, which an unprivileged reader
cannot open, so `isLocked` became an `Option` and stays unestablished rather than being reported as
unlocked. Authorized SSH keys belong to the account that holds them and this gateway reads none.
Storage reports `Known` only where a filesystem read produced a capacity; its subvolume list stays
empty everywhere, because that needs a privileged btrfs query this gateway does not make.

**Installing and upgrading are typed operations now.** `package.install` and `package.upgrade` join
the closed operation table, both High risk and neither reversible: they bring code onto this host and
run maintainer scripts as root, and removing a package afterwards does not undo what those scripts
did. Neither relieves any finding, so nothing this host concludes about itself can reach for one —
the machine cannot install software on its own conclusion while nobody is present. Neither is
pre-authorizable through the standing policy file either; the parser there still knows three verbs
and refuses the rest by name.

The path is the ordinary one: the gateway establishes the seat and nothing else, Action1 decides,
and the executor holds the only adapter. A person asking from their own authenticated seat is the
confirmation, exactly as it already is for restarting a service, and the record names whose seat it
was. A package name is checked against Debian's own naming rule twice — once where the proposal
becomes a typed action and once in the adapter — so a placeholder, an option, a version pin or a
path never becomes a permit, and `--` ends the argument list before the name is reached. Removing and
reinstalling stay refused by name: neither has an operation, and removal takes software away from a
host that may be depending on it, which wants its own decision about risk before it gets a verb.

Every such action appears in Operation1 through the Action1 producer, with no cancel offered,
because an executing permit cannot be recalled.

The executor's own sandbox stayed shut. A package manager has to write `/usr` and reach the network,
and relaxing the executor's unit to allow that would have handed every adapter in that process what
one of them needs. So apt runs somewhere else: the executor asks systemd to start it as a transient
unit with its own confinement, waits for the job it was given, and reads the unit's own verdict.
`ProtectSystem=full` and the Unix-only address family are back.

Reading that verdict is the part worth being careful about. `StartTransientUnit` returns when the
job is *enqueued*, so the wait is subscribed before anything starts; and a command that ran and
exited non-zero makes the job "failed" too, so the exit status is read separately from the job
result. Three outcomes stay three sentences: ran, ran and exited *n*, and could not execute its
command at all. `scripts/test-transient-unit-gate.sh` proves all three against a real service
manager, and that nothing is left loaded afterwards.

The missing halves are an `apt-get update` operation and a repository reader — until one exists, no
package is reported upgradable and the count stays unestablished.

Security and Backup still report `Unknown`, and every remaining mutation on these surfaces still
refuses. Installing, upgrading, connecting and snapshotting are new powers over the
host, and they belong behind an Action1 proposal, an operator decision and an Executor1 permit —
`ExecutableAction` has no variant for any of them yet, which is the honest state and an explicit
decision rather than an oversight.

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

The codebase is 100% Rust and WebAssembly on Debian 13. Maintain strict single-language discipline and no
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
