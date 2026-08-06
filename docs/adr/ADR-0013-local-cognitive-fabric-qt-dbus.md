<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0013: Local Cognitive Fabric Uses Qt D-Bus

## Status

Proposed

## Context

Independent local organs need typed IPC integrated with Qt.

## Decision

Use versioned Qt D-Bus interfaces and versioned CBOR where extensibility is needed. Service logic depends on a transport abstraction.

## Consequences

Local integration is inspectable and can later be bridged to a network transport.

## Alternatives Considered

Free-form natural-language organ chat was rejected.
