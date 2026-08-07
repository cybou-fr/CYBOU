<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Workspace and Attention

Workspace owns bounded active context, not biography.

## Current live admission

Workspace subscribes to the current Journal's post-COMMIT accepted stream:

```text
any current organ / Presence
→ Journal::append
→ durable COMMIT
→ Journal::accepted
→ Workspace::accept
→ contributed / focus reevaluation
```

This includes direct `Journal::append()` calls. They no longer bypass live attention.

`Workspace::accept()` is idempotent by `messageId`.

## Publish

`Workspace::publish(envelope)` remains available as a convenience submission method, but it no
longer has a second private admission path:

```text
publish
→ Journal::append
→ Journal::accepted
→ accept
```

Therefore a contribution cannot be admitted before it is durable or admitted twice simply because
it entered through `publish()`.

## Bounded moment

The moment is newest-first and capped by the configured capacity. Contributions leaving the
moment remain in the Journal.

## Coalitions

Contributions sharing a correlation episode form a coalition. Members are presented oldest-first
inside each coalition.

## Salience

The current deterministic score combines contribution kind, confidence, recency, and independent
organ corroboration.

## Recovery

`rehydrate()` reconstructs the bounded recent state at startup/recovery. It is no longer required
after every normal runtime action.

## Target

M3/M4 replace the in-process Journal signal with eventd/workspaced IPC while preserving the same
semantic order: only accepted durable contributions enter attention.
