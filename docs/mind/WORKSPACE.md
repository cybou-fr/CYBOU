<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Workspace and Attention

Workspace owns bounded active context, not biography.

## Current process

The authoritative live Workspace now runs in `cybou-workspaced`.

```text
Event1 Accepted
→ EventClient accepted
→ Workspace::accept
→ bounded moment update
→ focus reevaluation
→ Workspace1 Changed
```

`Workspace::accept()` remains idempotent by `messageId`.

## Recovery

On workspaced startup:

```text
Event1 Recent(capacity)
→ Workspace::rehydrate
→ deterministic coalition/focus reconstruction
```

Normal live operation then follows Accepted signals rather than polling/re-reading history after
each contribution.

## Presentation

presenced listens to Workspace1 Changed and only then emits Presence1 Changed. This preserves the
ordering:

```text
durable
→ admitted to global attention
→ shown
```

## Ownership

No Workspace copy is owned by presenced or the QML proxy. Tests may still construct a local
Workspace against a temporary EventStore as a unit-test seam.

## Direction

During consolidation, workspaced may rebuild a bounded moment, decay salience, or close an episode,
but only from an accepted input high-water mark and without becoming biography owner.

Executive attention extends current admission/focus with interruption, deferral, return, competing
intentions, and resource budgets. Typed value constraints may influence priority; they do not grant
external execution authority. See [Epistemic Governance](EPISTEMIC_GOVERNANCE.md).
