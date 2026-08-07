<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Workspace and Attention

Workspace owns bounded active context, not biography.

## Current implementation

Workspace keeps a bounded in-memory moment and can reconstruct it from recent Journal history.

Current methods include:

```text
publish(envelope)
rehydrate()
coalitions()
focus()
momentState()
```

`publish()` currently:

```text
Journal append
→ prepend contribution to bounded moment
→ emit contributed
→ reevaluate focus
```

`rehydrate()` replaces the bounded moment with recent Journal contributions and reevaluates focus.

## Current coalition model

Contributions are grouped by `correlationId`; if a correlation ID is null, `messageId` is used as
the fallback key.

Members are ordered as a story from older to newer contributions.

## Current salience

The implemented score is deterministic and combines:

- contribution-kind weight;
- confidence;
- recency with a 120-second half-life;
- square-root corroboration from distinct contributing organs.

Current kind weights prioritize `NeedSignal` and `Objection`, then `Decision` and `Intention`.

## Current limitation

Not every current write flows through `Workspace::publish()`.

Presence and several organ objects write directly with `Journal::append()`. Therefore Workspace
can be correct after `rehydrate()` while still becoming stale relative to new Journal writes during
the same live session.

This is an explicit open M1 issue, not an implemented global-attention guarantee.

## Target admission

```text
organ proposal
→ eventd validation and durable append
→ accepted-contribution signal
→ workspaced admission
→ focus change / Presence projection
```

The Target Workspace must update on every accepted contribution without polling SQLite and without
becoming the durable owner of biography.

## Recovery

Current: bounded recent state can be reconstructed from Journal on rehydrate.

Target: the isolated `workspaced` process reconstructs bounded state after restart and then follows
the accepted-contribution stream live.
