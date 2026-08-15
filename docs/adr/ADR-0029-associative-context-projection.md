<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0029: Associative Context Projection and Semantic Activation

## Status

Accepted

Accepted before implementation, for the same reason ADR-0028 was: this decision exists to fix who
owns what, and the alternative to fixing it now is discovering later that a vector index, a user
interface and a language model each believe they own "memory". By the time that is visible it is
load-bearing.

**Acceptance does not enable a language faculty.** ADR-0021 remains Proposed and no model is
consumed anywhere. What this makes possible without one is a Mind that can be asked what a word
brought to mind and answer from its own history.

Being Accepted, this outranks [Current State](../CURRENT_STATE.md): where an implementation and this
document disagree, the implementation is wrong.

## Context

Mind can now say what happened, what is known, and how sure it is. It cannot say what is *related*.

That gap is where a memory architecture gets decided by accident. A language model asked to answer a
question will happily retrieve whatever it can reach; a vector database dropped in beside the
Journal becomes a second memory with different rules; a context panel that assembles its own prompt
becomes the thing that decides what Mind knows. Each is a reasonable local decision, and together
they would leave no single answer to "where does Mind's memory live".

The substrate already refuses that pattern twice. Perception proposes and does not decide what is
true ([ADR-0027](ADR-0027-local-epistemic-projection-owner.md)); the epistemic owner derives and
never writes. This decision applies the same shape one layer up.

## Decision

### Association is not truth, and the vocabulary says so

Five distinctions are load-bearing, and collapsing any of them is the failure this ADR exists to
prevent:

```
association  ≠ truth
relevance    ≠ evidence
activation   ≠ attention
context      ≠ biography
embedding    ≠ knowledge
association  ≠ learning
```

The layers that follow from them:

```
Journal          what happened
epistemicd       what is known, and with what epistemic force
contextd         what is related, and what is relevant now
workspaced       what gets bounded attention
meaning/language how a selected context is interpreted or expressed
```

Each layer may read the one above it and may not overrule it.

### The owner is a separate process

`cybou-contextd`, the twelfth process, on the same terms as `cybou-epistemicd`.

It owns the `AssociativeProjection`, `ActivationSession` and `ContextBundle`. It owns **neither**
the Journal, nor truth, nor identity, nor attention, nor prompts, nor any language model. It never
writes to Event1 except to propose — and what it proposes is a candidate, not a fact.

Its index and checkpoint are entirely reconstructible. Deleting `$XDG_STATE_HOME/cybou/context/`
costs the speed of semantic recall and no memory at all, exactly as deleting the epistemic
checkpoint costs a replay and no knowledge. Where the index and the Journal disagree, the Journal is
right.

### What the projection holds

Not embeddings alone. An index of vectors can rank things and cannot explain itself, and a memory
that cannot say why it produced something is indistinguishable from one that made it up.

```cpp
struct ConceptNode {
    ConceptId id;
    ConceptKind kind;
    QList<QUuid> evidence;      // contributions this concept was derived from
    PrivacyClass privacy;       // inherited, most restrictive among evidence
    RetentionMetadata retention;
};

struct Association {
    ConceptId from;
    ConceptId to;
    RelationType type;
    double strength;
    AssociationOrigin origin;
    QList<QUuid> evidence;
};
```

`AssociationOrigin` is a closed set, and it is the field that keeps association from becoming
knowledge:

```
ObservedAssociation        seen in the biography
UserDeclaredAssociation    stated by the person
DerivedAssociation         inferred from other associations
ModelSuggestedAssociation  proposed by a language faculty
StatisticalAssociation     co-occurrence, nothing more
```

`lemon → yellow` and `lemon → makes people kinder` may both exist. They must never be
indistinguishable, and `contextd` does not adjudicate between them — for epistemic force it defers
to `epistemicd`.

### Activation is bounded, deterministic and inspectable

An `ActivationSession` spreads from seeds under an explicit budget:

```
seeds:  lemon
budget: 32 nodes, 64 edges, depth 3, 30 ms, 1800 tokens
```

Relevance combines seed similarity, association strength, task relevance, personal relevance,
freshness and recency. **The formula is deliberately not frozen here.** What is frozen is the set of
properties any formula must have:

- **deterministic** — the same snapshot, seeds and instant produce the same bundle;
- **bounded** — every dimension of the budget is enforced, not advisory;
- **inspectable** — every item can say why it was retrieved;
- **provenance-preserving** — every item names the contributions behind it;
- **privacy-aware** and **retention-aware** — an item carries the class and lifetime of its evidence.

Ranking is expected to improve. These properties are not.

### Seeds are not only words

A `ContextQuery` may be seeded by a concept, the current Workspace focus, an observation, a person,
a file, an application, a place, an intention, a prediction or an episode. Restricting seeds to text
would make the whole layer an accessory to a chat box, which is precisely the accident this ADR is
written to avoid.

### The prompt is not the primary object

```
UserInput → ContextQuery → activation → ContextBundle → [optional language faculty] → ModelRequest
```

Context selection is upstream of language, so it works without a model at all. Typing `lemon` and
being shown what came to mind is a complete feature of Mind, not a debugging view of a prompt
builder.

### ContextBundle is a protocol object, not a string

```cpp
struct ContextItem {
    QUuid id;
    ContextItemKind kind;
    QString subject;
    QCborValue value;
    double relevance;
    EpistemicStatus epistemicStatus;   // carried through, never upgraded
    QList<QUuid> evidence;
    PrivacyClass privacy;
    QString activationReason;
};

struct ContextBundle {
    QUuid requestId;
    QList<ContextItem> items;
    int tokenBudget;
    int estimatedTokens;
    ContextDestination destination;
    bool complete;
};
```

`complete` is the field that matters most. A retrieval that was cut short reports `complete = false`
rather than a short list, because an incomplete answer presented as a full one is the substrate's
oldest failure mode: **partial or unavailable is not empty truth.**

### Durable association and derived index are different things

If Mind genuinely learns that *the person prefers lemon with honey*, that is biography and must be a
durable typed contribution with evidence, privacy and retention. If `contextd` merely computed
`lemon ↔ honey 0.82` to make retrieval fast, that is a cache.

They behave differently under erasure, and [ADR-0028](ADR-0028-retention-and-erasure.md) already
says how:

```
durable association        → retention dependency closure (E7)
derived association/index  → erasure epoch invalidation and rebuild (E8)
```

## Consequences

- A twelfth process, with the capability registry, health graph and fault matrix following
  automatically from one declaration.
- Associative state is derived state, so every erasure invalidates it and it rebuilds. That cost is
  accepted for the same reason it was accepted for the epistemic projection.
- Explanations are structural rather than generated. "Why did you think of honey?" is answered from
  the graph — `lemon → UsedWith → honey`, strength 0.84, origin personal-history, evidence
  `contribution …` — without invoking a model to compose a plausible story. A generated explanation
  of a retrieval is not evidence about that retrieval.
- Ranking quality becomes an ordinary engineering problem, because the properties around it are
  fixed.

## Acceptance gates

| | Gate |
|---|---|
| **A1** | The same snapshot, seeds and instant produce the same `ContextBundle` |
| **A2** | Activation is bounded by node, edge, depth, time and token budgets |
| **A3** | Deleting the context checkpoint and rebuilding gives an observationally equivalent result |
| **A4** | A disputed epistemic state is still disputed after retrieval |
| **A5** | Association alone never creates durable knowledge |
| **A6** | A contextd failure reports unavailable or partial, never empty-known |
| **A7** | An erasure epoch invalidates the associative projection |
| **A8** | An erased sensitive payload cannot be reconstructed from surviving graph or index metadata |
| **A9** | A durable association inherits privacy and retention from its evidence |
| **A10** | Switching language models does not change canonical associative memory |
| **A11** | Workspace stays bounded even when activation returns thousands of associations |
| **A12** | Every `ContextItem` can answer "why was I retrieved?" |

A12 is the one to defend hardest. A memory that cannot explain a retrieval without asking a model to
invent a reason has already given away the property this whole layer exists to keep.

### Amendment: association may support learning; it does not promote it

`contextd` **may** offer associations, repeated co-occurrence, activation paths and source evidence
as inputs to a `LearningCandidate` under [ADR-0032](ADR-0032-layered-lifelong-learning.md). It
**must not** promote an association into an epistemic fact, a preference, a reusable skill, a policy
or a neural adaptation by itself.

`ModelSuggested` stays a candidate origin, not a shortcut into learned or epistemic authority. It is
already a closed set for exactly this reason: the field that keeps association from becoming
knowledge is the same field that keeps it from becoming learning.

## Related documents

- [ADR-0014: Workspace Admission and Global Attention](ADR-0014-workspace-admission-and-global-attention.md)
- [ADR-0021: Language Models Are Optional Faculties](ADR-0021-language-models-are-optional-faculties.md)
- [ADR-0025: Grounding, Epistemics, and Cognitive Governance](ADR-0025-grounding-epistemics-and-cognitive-governance.md)
- [ADR-0027: Local Epistemic Projection Owner](ADR-0027-local-epistemic-projection-owner.md)
- [ADR-0028: Retention and Erasure in an Append-Only Journal](ADR-0028-retention-and-erasure.md)
- [ADR-0030: Transparent Context Selection and Prompt Delivery](ADR-0030-transparent-context-delivery.md)
