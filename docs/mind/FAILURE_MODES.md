<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Failure Modes

This document distinguishes behavior already provided by the in-process prototype from Target
process-level requirements.

## Current fail-closed behavior

| Failure | Current behavior |
|---|---|
| structurally invalid envelope | append rejected |
| missing cause/evidence | append rejected |
| weaker derived privacy | append rejected |
| duplicate terminal Outcome for one cause | rejected; also backed by a unique SQLite index |
| append fails inside transaction | transaction rolled back |
| v2 hashed field is mutated | Journal verification reports the first broken row |
| v1 migration input is malformed | migration fails closed |
| v1 hash chain is damaged | migration fails closed |
| v1 migration is attempted | a `.v1.bak` backup is retained before schema migration |
| database schema is newer than supported | Journal refuses to open it as current state |

## Current architectural deficits

| Failure | Current limitation |
|---|---|
| `plasmashell` crashes/restarts | current in-process Presence/Mind object graph also stops |
| second Presence applet is created | one backend per session is not yet enforced |
| direct Journal write bypasses Workspace | live attention may be stale until rehydrate |
| host application identity/path changes | Mind persistence is not yet on the stable target XDG state path |
| one organ object fails independently | there is no process-isolated health/degraded-mode protocol |

## Target required behavior

| Failure | Target behavior | Status |
|---|---|---|
| Plasma crashes | Mind remains alive; Presence reconnects | Pending |
| `eventd` unavailable | no durable writes; read-only/degraded projection where safe | Pending |
| Journal fails verification | stop durable writes and report integrity failure | Partly present in library, process behavior pending |
| identity state missing/damaged | do not overwrite silently | Partly present, full process recovery pending |
| `intentiond` unavailable | commitment operations unavailable, other capabilities remain explicit | Pending |
| `workspaced` unavailable | no attention projection; biography remains intact | Pending |
| disk full | reject append atomically and expose the deficit | Atomic rejection expected; runtime deficit reporting pending |
| architecture migration fails | preserve recoverable state/backup and report degraded continuity | Journal migration present; architecture migration pending |
| protocol mismatch | reject and report unavailable capability | Local rejection partly present; IPC reporting pending |
| network partition | use only locally authorized behavior and reconcile explicitly | Pending |

A component failure must cause a specific capability deficit, never invented success.

Target rows are requirements and must not be described elsewhere as implemented until runtime
tests prove them.
