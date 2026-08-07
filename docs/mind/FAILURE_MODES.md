<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Failure Modes

## Current M3 behavior

| Failure | Current behavior |
|---|---|
| invalid proposal | eventd/Journal rejects it; no Accepted signal |
| failed Journal transaction | rollback; no Accepted signal |
| duplicate Outcome | rejected by Journal and SQLite constraint |
| eventd unavailable during default Presence wake | Presence stays not-awake; no local SQLite fallback |
| second eventd process | cannot own the same D-Bus service name |
| plasmashell restart | eventd can remain independent, but other organs are recreated |
| Journal verification failure | query exposes failure; explicit degraded UI state is not yet implemented |
| D-Bus timeout after wake | command/query fails; full health/capability projection is M6 |

## Pending

Process-specific degraded modes, automatic reconnect/reconstruction for every organ, and explicit
capability deficits are M4/M6 work.
