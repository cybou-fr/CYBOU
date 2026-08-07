<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0009: One Presence Instance per User Session

## Status

Accepted

## Context

Multiple QML components can instantiate Presence independently. Before M1 each instance could
construct its own Journal/Identity/organ graph against the same persistent state and increment the
Identity session merely because another surface was opened.

## Decision

There is one Presentation backend for the current user-session runtime.

For the current in-process architecture:

- Presence QObjects are lightweight surface wrappers;
- wrappers for the same canonical data root share one `PresenceRuntime`;
- that runtime owns one Journal, Identity, Intentions, Predictor, SelfModel, and Workspace object;
- its lifetime lasts until the last wrapper releases it;
- a later runtime after all wrappers are gone begins the next Identity session normally.

For the Target process architecture, surfaces connect to `presenced`; they do not construct Mind
objects.

## Consequences

Opening another current Presence surface no longer creates a new Identity session or another
normal Journal object graph.

All wrappers receive notifications from the shared runtime's accepted-contribution stream.

The current guarantee is process-local because Mind still lives inside `plasmashell`. M4 moves the
same ownership rule into `presenced` so it becomes a true session-level process boundary.

## Evidence

The `m1-runtime` test suite verifies:

- two Presence wrappers share identity/session state;
- a command through one wrapper is immediately visible through another;
- the runtime stays shared until all wrappers leave;
- the next runtime begins exactly one new Identity session.

## Alternatives Considered

One Presence backend per QML component or tab was rejected because presentation lifecycle must not
create cognition/session lifecycle.
