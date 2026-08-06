<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0003: AI in v0.1 - None

## Status
Accepted

## Context
Cybou v0.1 is the Visual Foundation phase, establishing the core architecture and cognitive framework. During this phase, we need to decide whether to integrate AI/ML components for features like prediction, natural language processing, or automated decision-making.

The Mind architecture is designed to be measurable and testable. ADR-0003 establishes the principle that "nothing is shown that is not measured" — every piece of information displayed must have a traceable origin in the journal.

## Decision
**No AI/ML components in v0.1.**

All cognitive functions must be:
1. **Deterministic** - Same input always produces same output
2. **Measurable** - Every decision can be traced back to its source
3. **Testable** - Can be verified with unit tests
4. **Transparent** - No "black box" decision making

The Predictor organ uses simple arithmetic (rolling mean) rather than ML models. This is deliberate:
- It can be wrong in measurable ways
- It can state its confidence level
- It can be replaced later without changing the protocol
- It maintains the fail-closed principle

## Consequences

### Positive
- **Reproducibility**: Every build produces identical results
- **Debuggability**: All decisions can be traced through the journal
- **Testability**: Full QtTest coverage is possible
- **Trust**: Users can understand exactly how decisions are made
- **Upgrade Path**: AI can be added in future versions without breaking existing architecture

### Negative
- Limited predictive capabilities in v0.1
- No natural language processing
- Simpler predictions than ML could provide

## Alternatives Considered

### Alternative: Simple ML Models
- Lightweight ML for prediction
- **Rejected**: Violates determinism requirement, harder to test

### Alternative: External AI Services
- Call to external AI APIs
- **Rejected**: Network dependency, privacy concerns, non-deterministic

### Alternative: Hybrid Approach
- AI for some features, deterministic for others
- **Rejected**: Creates inconsistency in architecture, harder to reason about

## Related
- Presence.h:7 - "the only class the surface talks to" invariant
- Predictor organ - Uses rolling mean instead of ML
- ADR-0008 - Mind Dock implementation maintains this principle
