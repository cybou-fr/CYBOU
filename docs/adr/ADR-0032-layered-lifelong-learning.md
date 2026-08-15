<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0032: Layered Lifelong Learning and Consolidation

## Status

Proposed

## Context

Cybou already accumulates accepted biography, predictions, intentions, epistemic state, associative
context, and lifecycle state. That accumulation is useful, but it is not by itself a complete
learning architecture.

A common shortcut is to define "learning" as fine-tuning a language model. That would make one
optional implementation technique the owner of every kind of adaptation. It would also mix rapidly
changing facts, user corrections, reusable procedures, preferences, and neural parameters into one
opaque state whose provenance and rollback are difficult to explain.

Cybou instead needs multiple learning speeds and representations. A new fact may become accepted
history immediately; a linguistic correction may update a reconstructible projection quickly; a
reusable skill should require repeated evidence and evaluation; neural adaptation, if used at all,
should be the slowest and most optional layer.

ADR-0024 already provides bounded, interruptible consolidation over a causally fixed Journal
high-water mark. Learning should use that lifecycle rather than create a second hidden scheduler or
a central service that owns every cognitive organ.

## Decision

### Learning is a system property

The following distinctions are normative:

```text
memory update       ≠ learning everything into weights
association         ≠ truth
repetition          ≠ truth
learned preference  ≠ permission
learned procedure   ≠ authorization
neural adaptation   ≠ learning as a whole
```

Cybou defines layered learning classes:

```text
L0  Episodic learning
    accepted biography and episode structure

L1  Epistemic learning
    what is currently known, disputed, superseded, stale, or unknown

L2  Associative learning
    what tends to relate to what, with provenance and bounded activation

L3  Linguistic and behavioral learning
    vocabulary, interpretation patterns, reference tendencies, preferences, ranking, interaction habits

L4  Procedural learning
    reusable verified skills, procedures, and policy-neutral task structure

L5  Neural learning
    optional learned parameters in local statistical/neural components
```

L5 is optional.

Core learning through L0-L4 MUST remain architecturally possible with no neural training backend
installed.

### Facts stay in Mind; learning may generalize from them

A volatile or authoritative fact is not made more durable by being encoded into learned parameters.

Examples:

```text
"the current server address is X"
    → epistemic state

"the person usually wants a concise technical explanation"
    → possible behavioral learning candidate

"three successful service-recovery episodes share this procedure"
    → possible procedural learning candidate
```

The learned representation may generalize behavior from evidence, but it does not become the source
of truth for the source facts.

### Learning begins as a candidate

A single event does not silently become a durable learned rule.

Conceptually:

```text
LearningCandidate {
    id,
    kind,
    sourceEvidence[],
    outcomeEvidence[],
    proposedGeneralization,
    scope,
    privacy,
    retention,
    derivationVersion
}
```

The exact wire type is deferred, but these semantics are required:

- source evidence is explicit;
- proposed scope is explicit;
- privacy and retention are inherited from evidence;
- the candidate is not yet an active skill, preference, policy, or model change;
- derived candidates never outrank the evidence that produced them.

### Fast learning and slow learning are separate

Some reconstructible learning can happen immediately after accepted evidence.

Example:

```text
user correction:
"when I say 'bring the database up', I mean start PostgreSQL"
      ↓
reconstructible lexical/reference learning state
```

This may improve the next interaction without training a large model.

More consequential adaptation requires consolidation and evaluation:

```text
candidate accumulation
        ↓
consolidation snapshot / high-water mark
        ↓
derivation or training
        ↓
evaluation
        ↓
promotion | rejection
```

Procedural skills and behavior-changing opaque learned artifacts MUST NOT become active merely
because generation/training completed.

### Consolidation schedules learning; it does not own learning

ADR-0024 remains the lifecycle authority.

`lifecycled` may determine that an idle/maintenance window is suitable, capture a bounded input
high-water mark, request typed learning work from the relevant owner, and observe completion,
interruption, or failure.

It MUST NOT become a hidden owner of vocabulary, preferences, skills, model weights, or every
learning projection.

This ADR deliberately does **not** introduce `learningd`. Before a new mutable learning service is
implemented, a dedicated owner/wire-contract decision must state exactly what it owns and how its
failure degrades Mind.

### Evidence quality matters, but learning score is not epistemic force

A learning system may distinguish stronger and weaker feedback, for example:

```text
uncorroborated system guess
< repeated occurrence
< explicit user correction or confirmation
< repeated successful observed outcome
< replay/evaluation across independent episodes
```

That ordering may guide whether a generalization is useful enough to promote. It MUST NOT silently
convert frequency or success into epistemic truth about the world.

### Procedural learning produces inspectable skills

Repeated successful episodes may yield a `SkillCandidate` or equivalent procedural artifact.

Example:

```text
service failure
  → inspect service state
  → inspect recent logs
  → inspect dependencies
  → inspect resource pressure
```

A promoted skill must remain inspectable enough to answer:

- what triggers it;
- what preconditions it assumes;
- what steps it proposes;
- what branches/failure states exist;
- what evidence/episodes caused it to be learned;
- what observations count as success.

A learned skill is a reusable procedure, not execution authority. ADR-0022 still decides whether an
instantiated action may execute.

### Neural adaptation is an optional implementation

Neural/statistical components may be trained for semantic parsing, ranking, prediction, response
realization, or other bounded capabilities.

This ADR does not choose:

- full fine-tuning versus parameter-efficient adaptation;
- Transformer versus another architecture;
- model size;
- optimizer;
- replay algorithm;
- embedding architecture.

Those are implementation choices constrained by ADR-0033's artifact lifecycle and provenance rules.

## Consequences

Cybou can improve continuously without rewriting one large model after every interaction.

Rapid factual change remains in explicit Mind state, while slower generalizations can accumulate
under evidence and evaluation.

User corrections become high-value learning evidence without retroactively rewriting the original
utterance or event.

Procedural learning can create reusable skills that are inspectable and independently authorized.

Neural training remains available as an optimization or specialization technique without becoming a
cognitive dependency.

The architecture requires explicit candidate, evaluation, ownership, and promotion semantics.

## Relationship to erasure

Every learning candidate and promoted learned artifact inherits retention dependencies from its
source evidence. ADR-0033 defines artifact invalidation and rebuilding; ADR-0028 remains the
canonical erasure contract for source material.

Erasure must not leave a learned behavior active when its continued use depends on evidence that the
system has committed to forget.

## Relationship to meaning

ADR-0031 corrections, interpretations, and reference resolutions may produce linguistic learning
candidates.

The parser/meaning layer may propose learning. It does not authorize its own durable adaptation.

## Acceptance gates

| | Gate |
|---|---|
| **L1** | A user correction can improve a reconstructible linguistic/behavioral projection without neural model training |
| **L2** | Every non-trivial `LearningCandidate` cites accepted source evidence and inherited privacy/retention metadata |
| **L3** | Replaying the same accepted history can reconstruct equivalent reconstructible learning state without relying on hidden model memory |
| **L4** | Contradictory examples remain representable as conflict; frequency alone does not silently become epistemic truth |
| **L5** | A procedural candidate is evaluated/replayed before it becomes an active skill through the intended path |
| **L6** | A promoted skill remains inspectable and does not itself grant execution authority |
| **L7** | The learning architecture remains usable with no neural training backend installed |
| **L8** | Interrupted consolidation does not duplicate candidates or promote partial work |
| **L9** | Lifecycle orchestration can fail without becoming the owner of learned domain state |

## Alternatives Considered

### Continually fine-tune one LLM and call that learning

Rejected because facts, preferences, skills, language adaptation, and neural parameters have
different ownership, evidence, erasure, and update semantics.

### Treat repeated events as truth

Rejected because statistical regularity is not epistemic authority.

### Promote a skill after one successful episode

Rejected because one outcome is insufficient evidence for a reusable procedure and may encode an
accident of context.

### Let lifecycled own all learning state

Rejected because lifecycle is an orchestrator, not a second owner of every cognitive domain.

## Related documents

- `../MIND_MODEL.md`
- `../ROADMAP.md`
- `ADR-0024-cognitive-lifecycle-and-consolidation.md`
- `ADR-0025-grounding-epistemics-and-cognitive-governance.md`
- `ADR-0028-retention-and-erasure.md`
- `ADR-0029-associative-context-projection.md`
- `ADR-0031-structured-meaning-and-cognitive-acts.md`
- `ADR-0033-learned-artifact-governance.md`
