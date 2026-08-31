<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0033: Learned Artifact Provenance, Promotion, Rollback, and Erasure

## Status

Proposed

## Context

ADR-0032 permits Cybou to learn linguistic behavior, preferences, reusable procedures, rankings,
and optional neural parameters. Those outputs affect future behavior, so they cannot be treated as
anonymous cache files or mutable model blobs.

Learning creates a new class of derived state with harder lifecycle requirements than an ordinary
reconstructible projection:

- some artifacts are deterministic and cheaply rebuildable;
- some are produced by optimization/training and may not be bit-identical on rebuild;
- a candidate can regress even when its training run succeeds;
- a promoted artifact can later need rollback;
- erasing source evidence must invalidate dependent learned behavior;
- neural parameters may encode influence from source examples in ways that cannot be surgically
  removed or proven absent.

Cybou therefore needs lineage, immutable generations, evaluation, promotion, invalidation, and
rebuild semantics before learned behavior becomes load-bearing.

## Decision

### Anything that changes future behavior because of past experience is a learned artifact

The category includes, but is not limited to:

```text
lexical/semantic mappings
reference-ranking state
preference/ranking models
planner heuristics or policies
verified skills
small classifiers or encoders
embeddings/rankers whose parameters were adapted from personal evidence
LoRA or other parameter-efficient adapters
fully fine-tuned personal neural models
```

An implementation may store some of these as reconstructible projections and others as explicit
versioned artifacts. The governance rules apply to both where they affect future behavior.

### Learned artifacts have lineage

Conceptually:

```text
LearnedArtifact {
    artifactId,
    kind,
    parentArtifact,
    sourceHighWaterMark,
    sourceEvidence[],
    derivationVersion,
    buildOrTrainingConfig,
    evaluationRecord,
    privacy,
    retention,
    state
}
```

with lifecycle states equivalent to:

```text
Candidate
Active
Rejected
Invalidated
Retired
```

The exact schema is deferred, but an active opaque artifact with unknown source lineage is not an
acceptable Cybou learned artifact.

### Active generations are immutable

An active learned artifact is never trained or edited in place.

The required shape is:

```text
P0041 ACTIVE
  
  - experience / consolidation
  ↓
  P0042 CANDIDATE
  ↓
  evaluation
  /       \
  reject     promote
  ↓
  P0042 ACTIVE
  P0041 RETIRED
```

This applies to neural and non-neural behavior-changing artifacts when they are not trivially
reconstructible projections.

An implementation may compact old generations under explicit retention policy, but promotion is
never an overwrite of the active bytes/state.

### A clean ancestor remains available

For neural adaptation, the foundation artifact is immutable. Personal learning may produce a fully
fine-tuned descendant; full-weight training is not prohibited.

```text
Foundation F0
  
  - Personal P001
  
  - Personal P002
```

F0 remains a clean rebuild point until retention policy explicitly and safely replaces that role
with another verified ancestor.

The architecture therefore permits:

```text
full fine-tuning
parameter-efficient adaptation
small specialized learned models
non-neural learned artifacts
no neural training at all
```

without making any one technique normative.

### Training/build input is a bounded projection, not a second biography

Opaque learning must consume a captured, provenance-bearing input derived from accepted Mind state.

Conceptually:

```text
Journal / owned projections
        ↓
LearningCandidate selection
        ↓
TrainingProjection / BuildSnapshot
        ↓
candidate artifact
```

The training/build dataset is not a new canonical memory. Reconstructible portions should be
rebuildable from accepted evidence and versioned derivation logic.

At minimum the snapshot records:

- Journal high-water mark;
- included evidence identifiers or a verifiable source manifest;
- derivation/version configuration;
- inherited privacy and retention constraints.

### Successful training is not promotion

A candidate becomes active only after evaluation.

The promotion path must be distinguishable from generation/training completion:

```text
candidate produced
      ↓
quality/regression evaluation
      ↓
privacy/safety checks
      ↓
promotion decision
```

The exact evaluation suite depends on artifact kind, but failure must leave the prior active
artifact untouched.

Promotion of a behavior-changing opaque artifact is auditable. The final protocol may use durable
records such as `ArtifactPromotionRequested`, `ArtifactPromoted`, `ArtifactRejected`, or equivalent
versioned types; this ADR fixes the semantic requirement, not the final message names.

### Rollback is first-class

If a newly promoted artifact regresses, violates policy, or is later invalidated, the system can
return to a prior valid generation without reconstructing history from prose or model memory.

Rollback changes which artifact is active; it does not rewrite the fact that a prior generation was
once promoted.

### Erasure invalidates dependent artifacts

ADR-0028 owns source erasure. Learned artifacts participate in its dependency semantics.

If erased evidence contributed to a learned artifact in a way that cannot be proven removable, that
artifact becomes `Invalidated` before it may be used again.

Required shape:

```text
ErasureApplied(E3)
      ↓
find dependent learned artifacts
      ↓
P0042 → INVALIDATED
      ↓
stop selecting P0042
      ↓
rebuild/retrain from clean ancestor + surviving evidence
      ↓
P0043 candidate
```

For neural parameters, Cybou does **not** claim that deleting a row or training example magically
unlearns its influence. Where exact removal is not demonstrable, rebuilding from an uncontaminated
ancestor and surviving evidence is the honest recovery path.

Rebuild after erasure need not be bit-identical to the old artifact. It must be lineage-correct and
must exclude the erased evidence/dependency closure.

### Learned artifacts inherit privacy and retention obligations

A learned artifact's privacy and retention are at least as restrictive as required by its source
manifest and the policy governing that artifact class.

Opaque learned state is not exempt from retention because its contents are difficult to inspect.
Difficulty of extraction is not proof of forgetting.

### Secrets and low-entropy sensitive payloads are not deliberate training targets

Until a future ADR establishes a stronger, demonstrable unlearning mechanism, credentials, cryptographic
keys, tokens, passwords, and similarly low-entropy sensitive payloads MUST NOT be deliberately used
as supervised targets for opaque learned artifacts.

Personal behavioral generalizations remain permitted when policy allows, for example:

```text
"the person prefers concise technical explanations"
```

The authoritative personal facts that support such a generalization remain in Mind; the learned
artifact does not become their only copy or authority.

### Learned state never outranks evidence or authority

The following are normative:

```text
artifact learned X      ≠ X is true
artifact predicts X     ≠ X is observed
artifact prefers action ≠ action is authorized
skill proposes steps    ≠ steps may execute
```

Epistemic force remains with the epistemic architecture. Execution authority remains with
ADR-0022.

### Core training/evaluation is local

Personal learned artifacts required for core Cybou behavior must be buildable/trainable/evaluable
without a mandatory network inference or training service.

An external tool may be imported deliberately in the future, but no core artifact lifecycle may
silently depend on a remote provider.

## Consequences

Full local fine-tuning remains possible without making mutable model weights the new biography.

A failed experiment cannot silently damage the active personal model or learned policy.

Learning gains an auditable lineage that can be tied to retention and erasure.

Erasure semantics remain honest: Cybou invalidates and rebuilds opaque learned state rather than
claiming unverifiable machine unlearning.

The system pays storage and evaluation costs for multiple generations and source manifests.

Promotion criteria must be designed per artifact class; one universal metric is intentionally not
specified here.

## Relationship to reconstructible projections

A deterministic projection that can be deleted and rebuilt exactly from Journal may remain a
projection rather than a heavyweight versioned artifact. Once a derived state is opaque,
optimization-produced, behavior-changing, or promoted independently, the lineage/promotion rules in
this ADR apply.

The owner decision for any future learning service must make that boundary explicit.

## Acceptance gates

| | Gate |
|---|---|
| **A1** | An active non-trivial learned artifact is immutable; new learning produces a candidate generation |
| **A2** | Every candidate has traceable lineage to a bounded accepted source snapshot/high-water mark |
| **A3** | Candidate generation/training success alone cannot replace the active artifact |
| **A4** | A failed evaluation leaves the previous active generation usable |
| **A5** | A promoted generation can be rolled back to a prior valid generation without rewriting history |
| **A6** | Erasing source evidence invalidates every dependent opaque learned artifact before intended reuse |
| **A7** | A rebuild/retrain after erasure demonstrably excludes erased evidence and its retention dependency closure |
| **A8** | A clean valid ancestor remains available for rebuilding a personally fine-tuned neural artifact under the configured retention policy |
| **A9** | Secrets/keys/tokens cannot enter the intended opaque-training target path under default policy. Depends on the sensitivity axis of [ADR-0018](ADR-0018-privacy-classification-and-replication.md): until a payload can be typed as a credential, this gate is satisfiable by a refusal for any reason at all |
| **A10** | Learned state cannot act as epistemic or execution authority through the intended interfaces |
| **A11** | Core personal artifact training/evaluation does not require a remote service |

## Alternatives Considered

### Train the active model in place

Rejected because rollback, evaluation isolation, erasure lineage, and failure recovery become
unreliable.

### Keep only the latest model file

Rejected because there is no clean ancestor, auditable promotion history, or deterministic rollback
path.

### Treat neural weights as exempt from erasure

Rejected because opaque representation is still derived storage influenced by personal evidence.

### Claim targeted machine unlearning after deleting training rows

Rejected as the default contract because exact removal from opaque learned parameters is not
currently something the architecture can generally prove.

### Forbid full fine-tuning entirely

Rejected because full local adaptation may be useful and is compatible with Cybou when performed as
an immutable descendant with lineage, evaluation, promotion, and rebuild semantics.

## Related documents

- `../MIND_MODEL.md`
- `ADR-0021-language-models-are-optional-faculties.md`
- `ADR-0024-cognitive-lifecycle-and-consolidation.md`
- `ADR-0028-retention-and-erasure.md`
- `ADR-0018-privacy-classification-and-replication.md`
- `ADR-0032-layered-lifelong-learning.md`
