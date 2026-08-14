<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0021: Language Models Are Optional Faculties

## Status

Proposed

## Context

Future language support must not become identity, memory authority, authorization authority, or
executor.

A central LLM agent would collapse multiple independent concerns into one replaceable model:
identity, biography, intention ownership, explanation, planning, and privileged action. That would
make model replacement equivalent to personality/memory replacement and would make hidden model
context an accidental source of authority.

The M1–M4 architecture instead establishes explicit owners for durable history, identity,
intentions, prediction/calibration, self projection, bounded attention, and presentation.

Language should attach to that substrate as an optional faculty.

## Decision

A language model MAY:

- parse or classify user requests;
- transform natural language into candidate typed observations or proposals;
- retrieve selected typed Mind context through explicit APIs;
- propose hypotheses;
- propose plans for later criticism/authorization;
- formulate explanations from typed Mind state;
- summarize or translate state without becoming its owner.

A language model MUST NOT become:

- identity authority;
- biography owner;
- canonical Journal writer;
- intention owner;
- authorization authority;
- privileged executor;
- the only location where commitments or continuity exist.

Model output that should influence durable cognition enters the typed protocol and is subject to the
same causal/evidence/privacy rules as other derived contributions.

A model does not directly write `journal.db`.

A model does not directly invoke arbitrary privileged shell commands.

Replacing, upgrading, disabling, or switching a model MUST NOT, by itself:

- create a new Cybou identity;
- erase accepted biography;
- erase open commitments;
- invalidate continuity that does not depend on that faculty.

Selected context should be provided to a model deliberately. The model is not granted unrestricted
database ownership merely because it can reason over text.

## Faculty relationship

Target relationship:

```text
typed Mind context
      │
      ▼
language faculty
      │
interpretation / hypothesis / proposal / explanation
      │
      ▼
typed protocol
      │
      ▼
Mind
```

Language is therefore replaceable capability rather than the cognitive substrate itself.

The first implementation is planned for M8.

## Consequences

Cybou remains architecturally alive without language.

Different local or remote models can be evaluated or replaced without redefining identity and
biography.

Natural-language fluency can improve independently from the storage/causal model.

Model hallucination is prevented from automatically becoming authoritative biography: derived state
still crosses typed contribution boundaries and their invariants.

The design requires explicit context-selection and protocol adaptation rather than passing the
entire persistence layer directly to a model.

## Relationship to future action

A language model may propose an action or plan, but authorization/execution belongs to ADR-0022.

The prohibited shortcut is:

```text
language model → privileged shell
```

## Amendment: a faculty consumes a ContextBundle and retrieves nothing

The interface is fixed:

```
ContextQuery → contextd → ContextBundle → ContextPolicy → LanguageFaculty
```

A `LanguageFaculty` **must not** perform unrestricted associative retrieval against the Journal or
context storage on its own, and the context supplied to a model **must** be representable as a
`ContextBundle` and inspectable independently of that model.

This is what makes a model replaceable. If a faculty could retrieve for itself, then swapping
Mistral for llama.cpp — or removing the model entirely — would change what Mind remembers and how it
recalls it, and the memory architecture would be a property of the current model rather than of
Mind. With this rule, the model changes and the memory does not.

Inspectable *independently of the model* also means an explanation of a retrieval is never generated
by the thing being explained. See [ADR-0029](ADR-0029-associative-context-projection.md) gate A12.

## Acceptance direction

M8 should demonstrate at least:

- Mind starts and remains usable with the language faculty absent;
- switching the configured model does not create a new identity/session by itself;
- the model cannot directly open/write canonical Journal storage through the faculty interface;
- model-derived durable contributions use the typed protocol;
- model output does not bypass the future authorization boundary for privileged action.

## Alternatives Considered

### Central LLM agent

Rejected because it makes one probabilistic replaceable model the de facto owner of identity,
memory, planning, and action.

### LLM with direct Journal database access

Rejected because it bypasses canonical write ownership and typed causal validation.

### LLM with direct privileged shell access

Rejected because interpretation/planning uncertainty must not equal execution authority.

## Related documents

- `../MIND_MODEL.md`
- `ADR-0001-system-architecture.md`
- `ADR-0002-cognitive-causality-and-journal-invariants.md`
- `ADR-0022-authorized-action-boundary.md`
