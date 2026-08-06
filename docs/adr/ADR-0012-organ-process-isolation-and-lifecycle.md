<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0012: Organ Process Isolation and Lifecycle

## Status

Proposed

## Context

Current organs are objects inside Presence and share one failure domain.

## Decision

True organs become separate executables managed by systemd user services. Shared libraries contain types and utilities but no hidden mutable cognition.

## Consequences

Organs can fail and recover independently; IPC and health contracts become mandatory.

## Alternatives Considered

A permanent cybou-mindd monolith was rejected.
