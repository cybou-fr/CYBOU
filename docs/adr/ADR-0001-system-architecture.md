<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0001: Body, Mind and Presence

## Status

Accepted

## Context

Cybou needs stable boundaries between the operating system, cognition, and user interface.

## Decision

Cybou consists of Body, Mind, and Presence. Body is the operating environment. Mind is the cognitive substrate. Presence is the presentation boundary. No single organ, model, database, or UI component is Cybou by itself.

## Consequences

The desktop presentation and future language models remain replaceable. New components must declare their domain.

## Alternatives Considered

A monolithic assistant process was rejected because it combines identity, memory, language, planning, and execution.
