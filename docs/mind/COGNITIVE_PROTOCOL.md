<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Cognitive Protocol

## Envelope

```text
schemaVersion
messageId
correlationId
causationId
originOrgan
originNode
kind
wallTime
monotonicTime
logicalClock
confidence
evidence[]
payloadCbor
privacy
capabilityScope
```

## Kinds

Observation, BeliefRevision, Hypothesis, MemoryRecall, NeedSignal, AttentionCandidate, Prediction, PlanProposal, Objection, Decision, Intention, Outcome, SelfAssessment, and Learning.

## Invariants

- `messageId` is unique and non-null.
- A contribution cannot cause or cite itself.
- References point only to persisted prior contributions.
- Evidence IDs are unique.
- Direct cause is not duplicated as evidence.
- Non-root contributions have cause or evidence.
- Confidence is finite and inside `[0, 1]`.
- Derived privacy is no weaker than cause or evidence privacy.

## Root policy for v2

Only Observation is a root. A user request, inspection request, or system event is first recorded as an Observation.

## Correlation and causation

Correlation groups an episode. Causation identifies the direct prior cause. They are not interchangeable.
