<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0022: Authorized Action Boundary for NixOS Mutation

## Status

Proposed

## Context

Future autonomous action creates a high-risk path from uncertain cognition to system mutation.

## Decision

Mutation follows proposal, critics, decision, capability authorization, typed executor, Nix build/test, confirmation when required, switch, observation, outcome, and rollback. No model or UI component invokes arbitrary privileged shell commands.

## Consequences

Actions become traceable and reversible where possible.

## Alternatives Considered

Direct LLM-to-shell execution was rejected.
