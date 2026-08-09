<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Grounding, Epistemics, and Cognitive Governance

## Scope

This document refines the M6/M7 target from ADR-0025. It does not claim that current M5 implements
perception adapters, an epistemic owner, retention propagation, or a value model.

## From environment to accepted observation

```text
Body / user / sensor
        │ raw signal or testimony
        ▼
perception adapter
        │ candidate + provenance + privacy
        ▼
protocol validation → Event1 acceptance → Journal
```

An adapter is a replaceable faculty. Acceptance means an observation is durably recorded, not that
its payload is objective truth.

Minimum provenance includes source identity/kind, acquisition and observation times, freshness,
transformation chain, integrity evidence, and privacy.

## Epistemic projection

| Status | Meaning |
|---|---|
| `Observed` | accepted direct machine/sensor observation |
| `Reported` | testimony from a user or external source |
| `Inferred` | derived from explicit causes/evidence |
| `Assumed` | used provisionally with explicit uncertainty |
| `Disputed` | supporting and contradicting evidence coexist |
| `Superseded` | newer scoped evidence replaces current applicability |
| `Stale` | freshness policy no longer supports current use |
| `Unknown` | required evidence is absent or unavailable |

Status is scoped by node, time, and task. Confidence never removes provenance or status.

## Contradiction and reconciliation

Contradiction is first-class projection state, not an exception hidden by last-write-wins.
Reconciliation records conflicting evidence, scope/time, applied rule or user decision, resulting
status, remaining uncertainty, and privacy/retention consequences.

Original evidence remains biography unless separate retention/erasure policy applies.

## Retention and forgetting

Retention distinguishes active projection, durable source history, archive, expired material,
user-requested erasure, replicated copies, and derived summaries/embeddings/projections.

An erasure request identifies affected derived and replicated material. A tombstone may preserve
causal shape only when it does not retain the sensitive content it claims to erase. Cryptographic
erasure, key scope, backups, and peer acknowledgement require explicit implementation decisions.

## Homeostasis and metacognition

Typed pressure includes storage growth, backlog, RPC latency, stale projections, unresolved
contradictions, calibration drift, overdue intentions, failed consolidation, and resource budgets.
Pressure requests attention or maintenance; it does not authorize deletion or mutation.

Explanations and decisions expose evidence, epistemic status, freshness, assumptions, missing
capabilities, applied constraints, and what new observation could change the result. A future
language faculty formulates this typed state but cannot replace it with model confidence.

## Executive attention and values

Workspace evolves into bounded interruption, deferral, return, and competition policy. Priorities
are criticized against user authority, safety, privacy, reversibility, cost, urgency, evidence
quality, and resource budget.

Value constraints can reject or defer a proposal. They cannot grant execution permission; M9
authorization remains separate.

## M7 minimal vertical slice

1. One system-state perception adapter.
2. Provenance-bearing Observation acceptance.
3. One scoped projection with `Observed`, `Disputed`, `Stale`, and `Unknown`.
4. Contradiction and user-assisted reconciliation.
5. One retention/erasure rule propagated into a derived projection.
6. Capability-aware explanation without an LLM.
7. Replication of only permitted projection/evidence to one trusted node.

## Related documents

- [Cognitive Protocol](COGNITIVE_PROTOCOL.md)
- [Journal](JOURNAL.md)
- [Workspace](WORKSPACE.md)
- [Lifecycle](LIFECYCLE.md)
- [Privacy Model](../security/PRIVACY_MODEL.md)
- [ADR-0025](../adr/ADR-0025-grounding-epistemics-and-cognitive-governance.md)
