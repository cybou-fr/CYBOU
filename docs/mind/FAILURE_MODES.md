<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Failure Modes

## Current M5 isolation and continuity guarantees

| Failure | Current behavior |
|---|---|
| QML Presence destroyed | cognitive services remain independent |
| presenced restarts | organ processes and identity session remain |
| identityd restarts in same login | persistent identity resumes without incrementing session |
| workspaced restarts | bounded attention can rehydrate from Event1 history |
| predictord fails | other organ processes remain alive; prediction calls fail |
| eventd write rejected | no accepted event enters Workspace |
| duplicate D-Bus owner | second daemon instance fails service-name acquisition |
| lifecycled split commit | deterministic Event1 effect is reused after process restart or reboot |
| Plasma recreation | lifecycle run and Event1 count remain unchanged |

## Not yet M6

M4 creates failure domains and a minimal Ready/Health contract. It does not yet define the complete
capability-deficit UI, retry/backoff policy, dependency reconciliation, or user-facing degraded
state machine.

## Future lifecycle and epistemic failures

| Failure | Required future representation |
|---|---|
| consolidation interrupted | accepted partial work plus `Interrupted`, followed by reconciliation |
| required maintenance owner unavailable | run fails or remains degraded; never reports completion |
| stale perception source | affected claims become `Stale` or `Unknown` |
| contradictory observations | explicit `Disputed` projection; no silent last-write-wins |
| erasure cannot reach a replica | incomplete retention obligation and visible deficit |
| value policy unavailable | planning/priority capability unavailable; no fallback authorization |
| language faculty unavailable | typed Mind remains available without natural-language capability |

Recovery policy must distinguish unavailable capability, uncertain result, and verified failure.
See [Lifecycle](LIFECYCLE.md) and [Epistemic Governance](EPISTEMIC_GOVERNANCE.md).
