<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0010: Journal v2 Schema and Canonical Hashing

## Status

Proposed

## Context

Journal v1 lacks explicit schema/hash versions and does not hash every semantic field.

## Decision

Introduce database, envelope, and hash versions; canonical binary encoding; evidence relations; transactional migration; version-aware verification; and no rewriting of v1 rows.

## Consequences

Mutation of privacy, confidence, evidence, origin node, and capability scope becomes detectable.

## Alternatives Considered

Rehashing old history was rejected.
