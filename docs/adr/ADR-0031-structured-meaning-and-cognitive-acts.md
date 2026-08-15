<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0031: Structured Meaning and Cognitive Acts

## Status

Proposed

## Context

Mind already distinguishes durable biography, epistemic force, associative relevance, attention,
and presentation. Natural-language input is planned for M8, but passing raw text directly into
planning or action would collapse several different questions:

- what words were observed;
- what the person probably meant;
- what object or prior episode a reference points to;
- whether that interpretation is ambiguous;
- whether the interpreted claim is true;
- whether a requested operation is authorized.

A generative model can hide all of those transitions inside one response. That is convenient and
architecturally dangerous. Cybou needs a meaning boundary whose results remain inspectable after the
parser or language implementation that produced them is gone.

The same boundary also permits multilingual and non-generative language implementations: different
surface forms may map to the same canonical cognitive act, while one `ResponsePlan` may be rendered
in different languages without changing Mind state.

## Decision

### Natural language is not the cognitive protocol

The target path into Mind is:

```text
Utterance / other human expression
          │
          ▼
MeaningInterpretation
          │
          ▼
ReferenceResolution
          │
          ▼
CognitiveAct
          │
          ▼
Mind APIs
```

The target path out of Mind is:

```text
typed Mind state
      │
      ▼
ResponsePlan
      │
      ▼
Language Realization
      │
      ▼
human-readable expression
```

`CognitiveAct` and `ResponsePlan` are protocol objects, not prompt strings.

### Interpretation is derived state, not truth

The distinctions are normative:

```text
utterance       ≠ meaning
interpretation  ≠ truth
reference       ≠ identity
ambiguity       ≠ failure
ambiguity       ≠ permission to guess
request         ≠ authorization
```

An interpretation always preserves provenance to the expression and contextual evidence from which
it was derived.

For example, a person saying:

```text
"PostgreSQL fell over because memory was exhausted."
```

may produce a semantic act equivalent to:

```text
Inform(
    claim = CausedBy(PostgreSQLFailure, MemoryPressure),
    source = person
)
```

That is a report from the person. It does not upgrade the claim to `Observed` or otherwise bypass
epistemicd.

### Cognitive acts use a versioned typed vocabulary

The first schema should remain deliberately small. It needs enough vocabulary to support ordinary
interaction without pretending to encode every human speech act.

Initial act families may include:

```text
Ask
Inform
Request
Correct
Confirm
Reject

Inspect
Explain
Compare
Verify

Resume
Pause
Cancel

Remember
Forget

Propose
```

Acts may be composed through versioned structural operators such as:

```text
Sequence
Conditional
Alternative
Constraint
Negation
```

The exact C++ representation is an implementation decision, but free-form model text is not an
acceptable substitute for the typed act once a request crosses into Mind.

### References are first-class and inspectable

References such as:

```text
"that server"
"it"
"the previous one"
"like yesterday"
"put it back"
```

are resolved against explicit candidate state: current Workspace, recent context, active intentions,
episodes, application state, and other typed entities.

Resolution preserves candidate ambiguity. Conceptually:

```text
ReferenceResolution {
    candidates: [
        homeserver      0.47,
        dev-server      0.41,
        postgres        0.12
    ],
    resolved: false
}
```

A mutating `CognitiveAct` MUST NOT silently select an unresolved referent merely because one
candidate has the highest score. The interface asks for clarification when policy requires a
resolved target.

### Corrections append; they do not rewrite prior meaning

If the system interpreted an utterance incorrectly and the person corrects it, the correction
becomes new accepted evidence linked to the prior expression and interpretation.

Conceptually:

```text
Utterance U1
   ↓
Interpretation I1
   ↓
user correction U2
   ↓
Interpretation I2 supersedes I1 for the active dialogue state
```

The existence of I1 remains auditable. Correction is not retroactive mutation of what was
previously understood.

### Response planning precedes language realization

A response should exist semantically before it becomes prose.

Example:

```text
ResponsePlan {
    goal: ExplainStatus,
    claims: [PostgreSQL = Healthy],
    causalHistory: [DiskPressure, Failure, Cleanup, Recovery],
    unresolved: [BackupVerification],
    qualifications: []
}
```

A Russian, French, or English realizer may render that plan differently. The renderer may vary
style, brevity, morphology, lexical choice, and presentation; it may not invent new authoritative
facts outside the plan.

### Language implementations are replaceable

A `MeaningInterpretation` may be proposed by:

- grammar and deterministic parsing;
- a statistical semantic parser;
- a small classifier/ranker;
- a local generative model;
- a hybrid pipeline.

The architecture does not privilege one implementation.

The producer may attach scores or alternatives, but the canonical act remains typed and inspectable.

## Consequences

Cybou can support natural interaction without making language-model hidden state the cognitive
protocol.

Multilingual interaction becomes a boundary concern rather than separate cognitive worlds.

Reference ambiguity becomes explicit state that can be tested and surfaced instead of a hidden
hallucination risk.

Corrections become valuable evidence for future learning without rewriting prior history.

Response generation becomes more constrained: fluent wording cannot silently add claims that Mind
did not provide.

The design introduces schema/versioning work for meaning objects and requires careful handling of
references, dialogue state, and clarification.

## Relationship to context

ADR-0029 owns associative retrieval and `ContextBundle`. ADR-0030 owns which context is delivered to
a consumer. The meaning interface consumes a permitted context projection; it does not retrieve
unrestricted history on its own.

A meaning implementation may use context to rank interpretations or references, but context
relevance does not turn an interpretation into truth.

## Relationship to learning

A correction, confirmed interpretation, or repeated reference resolution may provide evidence for a
`LearningCandidate` under ADR-0032.

The meaning layer does not directly promote its own guesses into durable preferences, skills, or
neural parameters.

## Relationship to action

A `Request` or other action-bearing `CognitiveAct` is still only interpreted intent.

```text
CognitiveAct(Request(...)) ≠ authorized action
```

ADR-0022 remains the boundary for mutation.

## Acceptance gates

| | Gate |
|---|---|
| **C1** | A supported natural-language path produces a typed, inspectable `CognitiveAct` rather than passing raw prose as execution authority |
| **C2** | An unresolved referent remains explicitly unresolved; a mutating request cannot silently choose it through the intended path |
| **C3** | A correction is linked to the prior interpretation instead of rewriting prior accepted history |
| **C4** | The resulting `CognitiveAct` remains inspectable after the parser/backend that produced it is stopped or replaced |
| **C5** | A `ResponsePlan` expresses claims, evidence references, and qualifications before language realization |
| **C6** | A renderer cannot add an authoritative claim that is absent from the supplied `ResponsePlan` through the intended interface |
| **C7** | At least two surface-language realizations can map to or render the same canonical semantic object in test fixtures |
| **C8** | No generative model is required to satisfy the core meaning-interface gates |

## Alternatives Considered

### Raw prompt as protocol

Rejected because prompt text has no stable ownership, reference, ambiguity, epistemic, or
authorization semantics.

### Let a language model own dialogue state

Rejected because changing or removing the model would then change what Mind believes the person is
referring to or has already committed to.

### Treat highest-score reference as resolved

Rejected because ranking uncertainty is not permission to mutate the wrong target.

### Store only final generated prose

Rejected because explanation wording is not a stable representation of the underlying claims,
qualifications, or intended communicative act.

## Related documents

- `../MIND_MODEL.md`
- `../ROADMAP.md`
- `ADR-0021-language-models-are-optional-faculties.md`
- `ADR-0022-authorized-action-boundary.md`
- `ADR-0027-local-epistemic-projection-owner.md`
- `ADR-0029-associative-context-projection.md`
- `ADR-0030-transparent-context-delivery.md`
- `ADR-0032-layered-lifelong-learning.md`
