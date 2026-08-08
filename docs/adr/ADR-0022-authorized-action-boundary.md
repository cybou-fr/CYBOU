<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0022: Authorized Action Boundary for NixOS Mutation

## Status

Proposed

## Context

Future autonomous action creates a high-risk path from uncertain cognition to system mutation.

The system may eventually observe a problem, form an intention, use planning or language faculties,
and propose a change to NixOS or another external resource. None of those cognitive stages should
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

A language/planning faculty may propose a mutation.

The proposal carries no inherent permission to perform it.

Authorization is a separate policy decision based on typed capability, target, risk, context, and
user/system policy.

### Execution is typed

Executors should expose constrained operations/capabilities rather than accepting unrestricted
model-generated shell text as authority.

Where NixOS mutation is involved, build/test/switch semantics should be used when applicable so the
system can inspect a candidate before activation.

### Confirmation is policy-dependent

Some operations may be safe enough for pre-authorized automatic execution.

Higher-risk actions may require explicit confirmation.

Destructive or forbidden actions may remain unavailable regardless of model confidence.

The exact policy matrix belongs to M9 design.

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
failure need explicit terminal or intermediate state in the final M9 protocol.

If the post-action environment cannot be observed reliably, the system should represent outcome as
unknown/degraded rather than invent success.

## Acceptance direction

M9 should demonstrate at least:

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
