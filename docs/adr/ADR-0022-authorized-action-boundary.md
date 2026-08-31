<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0022: Authorized Action Boundary for System Mutation

## Status

Accepted

The boundary is implemented through the first executor. `cybou-actiond` owns proposal identity,
criticism, standing policy, decisions and short-lived single-use permits. `cybou-executord` can
atomically claim such a permit and exposes only the three typed adapters below. The live gate uses a
disposable systemd service and independently reads its state after a restart; the executor's own
report is not treated as that observation.

## Context

Autonomous action creates a high-risk path from uncertain cognition to system mutation.

The system observes a problem, forms an intention, may use planning or language faculties, and
proposes a change to the host or another external resource. None of those cognitive stages should
automatically imply execution authority.

A direct model-to-shell path would combine uncertain interpretation, planning, authorization, and
mutation into one opaque operation. It would also make it difficult to establish what was proposed,
what was authorized, what actually happened, and whether rollback is possible.

## Decision

External mutation follows an explicit boundary:

```text
proposal
→ critics / checks
→ decision
→ capability authorization
→ typed executor
→ build/test where applicable
→ confirmation when required
→ execution / switch
→ observation
→ outcome
→ rollback where possible
```

No model or UI component invokes arbitrary privileged shell commands as the normal architecture.

### Proposal is not authorization

A meaning or planning faculty, or any other proposal producer, may propose a mutation.

```text
learned preference ≠ authorization
learned skill      ≠ authorization
```

A skill learned under [ADR-0032](ADR-0032-layered-lifelong-learning.md) may instantiate a proposal.
It does not grant its own capability: a procedure that carried permission because it had worked
before would make repetition into authority, which is the substitution this boundary exists to
prevent.

The proposal carries no inherent permission to perform it.

Authorization is a separate policy decision based on typed capability, target, risk, context, and
user/system policy.

### Deciding and doing are different processes

The boundary above is a sequence of stages, and a sequence of stages in one process is a convention.
One process holding both the authority to decide and the capability to perform is one refactor, one
convenience method, one *while we are here* away from a path that skips the middle — and nothing in
the type system objects, because both ends are already in scope.

So the split is physical:

```text
  MIND
  
  cybou-actiond
  Action1
  
  claim + durable
  ExecutionStarted
  
  ▼
  cybou-executord
  Body capability
  
  ▼
  host
```

`cybou-actiond` owns the proposal lifecycle, criticism, standing policy, user confirmation, the
`AuthorizationDecision` and the permit that follows from it. It has **no capability to carry any
operation out**: no privileged adapter, no shell, no ability to reach one.

`cybou-executord` owns a fixed set of typed adapters and nothing else. It cannot decide whether an
operation is allowed; it can only refuse a permit it cannot verify.

```text
Action1 can authorize but cannot execute
Executor can execute but cannot authorize
```

This is the same rule the rest of the architecture already runs on — a faculty is not an organ,
`contextd` may propose and `workspaced` decides, a model may interpret and never authorize. Action
is where getting it wrong costs the most, and is therefore the one place it must be a process
boundary rather than a function boundary.

### An agent's request to leave its capsule is a proposal

Under [ADR-0042](ADR-0042-agent-capsule-platform.md) an agent is free inside its capsule and has no
capability outside it. When it reaches for the host — restart a service, change the firewall, read a
host key, publish a port — that is not a new mechanism and not a permission dialogue. It enters here,
at the top, as an `ActionProposal` from a named actor.

That this boundary needed no change to accept a request from an agent is the strongest evidence
available that it was drawn in the right place: it was designed for Cybou's own remediation
proposals, and the same gate serves a party Cybou does not trust at all.

### Execution is typed

Executors should expose constrained operations/capabilities rather than accepting unrestricted
model-generated shell text as authority.

Where the host offers a way to inspect a candidate before activating it, that way is used. On a
declarative host that is build/test/switch; on the Debian servers ADR-0041 makes the primary target
it is whatever dry-run, `--simulate` or staged form the operation has. Where an operation has no
such form, that is a property of the operation and belongs in its risk, not something to paper over
with a rehearsal that does not rehearse anything.

### Confirmation is policy-dependent

Some operations may be safe enough for pre-authorized automatic execution.

Higher-risk actions may require explicit confirmation.

Destructive or forbidden actions may remain unavailable regardless of model confidence.

The policy matrix is carried by the standing policy rather than compiled in: a host on which
nobody has granted anything is the default, and it grants nothing.

### Every attempted action returns to cognition

Action is not fire-and-forget.

Every attempted external action should produce enough typed observed state to determine:

- what was proposed;
- what was authorized;
- what was attempted;
- what actually happened;
- what evidence was observed afterward;
- whether the intended outcome was reached;
- whether rollback was attempted or remains available.

The observed consequence returns to Mind as `Observation`, `Outcome`, and/or evidence according to
the final protocol design.

This closes the future cognitive loop:

```text
Observation
  ↓
Mind / Intention
  ↓
Planning
  ↓
Authorization
  ↓
Typed execution
  ↓
Environment
  ↓
Observed consequence
  - ► Mind
```

### Models are not execution authorities

ADR-0021 remains in force.

A model may:

- interpret;
- propose;
- explain;
- criticize.

It does not become the authorization authority or privileged executor.

### Amendment: the first executor holds three adapters (2026-08-25)

The operations that may *exist* in the first executor, decided by the owner of the machine this is
built for. Nothing else is to be implemented, and an operation absent from the code is a stronger
statement than one refused by policy.

| Operation | In the first executor | Why |
|---|---|---|
| `service.status` | Yes | Read-only, and it exercises the whole `Action1` → executor transport before anything mutates |
| `package.cache.clean` | Yes | A bounded mutation with a clear outcome |
| `service.restart` | Yes, concrete `.service` units only | The first genuinely useful self-healing action |
| `service.reload` | Not yet | Its outcome is poorly defined |
| `log.rotate` | No | Needs retention semantics first |
| `tmp.trim` | No | Too hard to establish that a file is actually disposable |
| `service.data.delete` | Never in v1 | Critical, and on the forbidden list |
| `filesystem.format` | Never | Critical, and on the forbidden list |
| `system.poweroff` | Never | Critical, and on the forbidden list |

**An implemented adapter is not a pre-authorized one.** The standing policy still grants nothing by
default, and it grants separately for this host's own findings and for an agent.

`service.restart` is included now and would not have been six months ago. It is included because a
finding now carries what it is about, so `service.active (postgresql.service)` produces a proposal
naming `systemd:postgresql.service` rather than a placeholder. An operation that cannot name its
target is an operation nobody can authorize, and until that was true this one could not be offered.

The first live S0 pass should use a harmless unit created for the purpose, not a database.

### Amendment: the executor speaks to systemd, not to a shell (2026-08-25)

A typed operation must stay typed all the way to the Body. Three shapes are excluded, in order of
how bad they are:

```text
sh -c "systemctl restart …"     a string becomes an instruction
Command::new("systemctl")       argv becomes the interface
execute(program, args)          the executor becomes a shell with extra steps
```

The last is the one to name explicitly: **the executor exposes no general execution API**, not even
a private one. An adapter is a function that takes a typed target and does one thing.

For services, that means the systemd manager API over D-Bus — `RestartUnit("foo.service",
"replace")` — and concrete `.service` units only. Not `.mount`, `.socket`, `.target`, `.timer`. The
`systemd:<unit>` placeholder is refused at the executor: it means *some unit, and this host cannot
say which*, which is not something to carry out.

For the package cache, one fixed adapter with a fixed argument vector and no shell.

### Amendment: durable execution starts before mutation (2026-08-27)

Exactly-once external effects are not promised. The stronger honest invariant is:

> Once execution may have begun, CYBOU never automatically repeats it merely because the final
> report was lost.

`Action1.ClaimPermit` therefore does three things as one ownership decision: consumes the one-use
permit, mints a stable attempt identity, and requires Event1 to accept an `ExecutionStarted`
contribution. Only then does it return an `ExecutionClaim` containing the typed action to Executor1.
The durable boundary precedes the first Body adapter call.

`ExecutionStarted` is not a fabricated completed attempt. It says only that the executor may now
begin. Executor1 later records the matching `ExecutionAttempt` directly with Action1. If the
executor dies, the machine reboots, or the D-Bus reply is lost after an effect but before that final
report, replay turns a start with no report into `AttemptReport::DidNotFinish`. Initiative treats
that as `OutcomeUnknown` and does not repeat the mutation automatically.

If Event1 cannot acknowledge the start, ClaimPermit fails and the executor receives no action. A
submission whose acknowledgement was lost may conservatively leave a start for an effect that did
not occur; refusing an automatic retry in that ambiguous case is the intended fail-closed result.

## Consequences

Actions become traceable and reversible where possible.

The system can distinguish:

```text
planned action
authorized action
attempted action
successful action
observed outcome
```

These are not treated as the same event.

Prediction calibration and self/attention state can later use observed outcomes instead of trusting
that command dispatch implied success.

The design requires explicit capability APIs and policy rather than convenient unrestricted shell
access.

## Failure behavior

Executor failure must not be silently reported as achieved outcome.

Partial application, failed validation, failed switch, missing confirmation, timeout, and rollback
failure need explicit terminal or intermediate state in the final M10 protocol.

If the post-action environment cannot be observed reliably, the system should represent outcome as
unknown/degraded rather than invent success.

## Acceptance direction

M10 should demonstrate at least:

- a model/UI cannot invoke arbitrary privileged shell through the intended action path;
- proposal and authorization are distinct typed stages;
- a denied capability cannot execute;
- confirmation policy can block execution;
- executor result is not considered final until relevant consequences are observed;
- failure/rollback paths are represented explicitly;
- action observations can return into Journal/Workspace through accepted protocol paths.

## Alternatives Considered

### Direct LLM-to-shell execution

Rejected because uncertain text generation must not equal privileged authority.

### UI component performing privileged mutation directly

Rejected because presentation is not an authorization/execution domain.

### Fire-and-forget executor

Rejected because dispatch is not evidence that the intended outcome occurred.

## Related documents

- `../MIND_MODEL.md`
- `ADR-0021-language-models-are-optional-faculties.md`
- `ADR-0002-cognitive-causality-and-journal-invariants.md`
