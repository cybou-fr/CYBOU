<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0021: Language Models Are Optional Faculties

## Status

Proposed

## Context

Future language support must not become identity, memory authority, or executor.

## Decision

Models may parse requests, propose hypotheses, and formulate explanations. Outputs enter the typed protocol. Models cannot directly write the Journal or execute privileged actions.

## Consequences

Cybou remains alive without language and models remain replaceable.

## Alternatives Considered

A central LLM agent was rejected.
