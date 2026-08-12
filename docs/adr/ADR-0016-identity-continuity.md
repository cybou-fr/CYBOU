<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0016: Identity Continuity Across Sessions and Upgrades

## Status

Accepted

Enforced by identityd and proven by the headless reboot gate: the persisted identity and logical
session semantics survive a real system transition.

## Context

A stable UUID alone does not prove continuity after reboot or migration.

## Decision

Continuity requires identity state, verified Journal, active commitments, architecture version, migration record, and explicit session transitions. Failure yields degraded continuity.

## Consequences

Cybou avoids falsely claiming seamless identity.

## Alternatives Considered

Creating a new identity over damaged state was rejected.
