<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0022: Authorized Action Boundary for System Mutation

## Status

Accepted

The boundary is built up to the executor, and the executor is deliberately absent.
`cybou-remediation` implements proposal, criticism and authorization as separate steps over a
closed set of typed operations. No executor exists: nothing in this repository can carry out any of
them. That is the state this ADR describes, not a gap in it — the boundary is the part that had to
exist first, and building it in the other order would mean an executor waiting for a policy.

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
                │
         cybou-actiond
             Action1
                │
        ExecutionPermit
                │
                ▼
         cybou-executord
          Body capability
                │
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
    └──────────────► Mind
```

### Models are not execution authorities

ADR-0021 remains in force.

A model may:

- interpret;
- propose;
- explain;
- criticize.

It does not become the authorization authority or privileged executor.

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
