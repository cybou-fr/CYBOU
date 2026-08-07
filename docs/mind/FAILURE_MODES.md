<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Failure Modes

## M4 isolation guarantees

| Failure | Current M4 behavior |
|---|---|
| QML Presence destroyed | cognitive services remain independent |
| presenced restarts | organ processes and identity session remain |
| identityd restarts in same login | persistent identity resumes without incrementing session |
| workspaced restarts | bounded attention can rehydrate from Event1 history |
| predictord fails | other organ processes remain alive; prediction calls fail |
| eventd write rejected | no accepted event enters Workspace |
| duplicate D-Bus owner | second daemon instance fails service-name acquisition |

## Not yet M6

M4 creates failure domains and a minimal Ready/Health contract. It does not yet define the complete
capability-deficit UI, retry/backoff policy, dependency reconciliation, or user-facing degraded
state machine.
