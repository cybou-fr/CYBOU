<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Failure Modes

## Current isolation, continuity, and degradation guarantees

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
| optional owner unavailable | only dependent capabilities and commands become unavailable |
| required eventd unavailable | dependent mutations fail before Journal acceptance; other processes remain alive |
| owner registered but unresponsive | compound Presence operation exhausts one shared deadline and returns bounded partial/typed failure |
| stale Health1 scheduling evidence | Lifecycle1 refuses execution without creating a run |
| lifecycled crashes after scheduled-run creation | restart enters `Recovering` and resumes the same deterministic run |
| late owner reply after interruption | reply is fenced and cannot advance or overwrite the terminal run |

## Current representation

Health1 distinguishes component observations, capability availability, deficits, and recovery
progress. Presence1 exposes that projection and command availability while keeping endpoint
reachability, aggregate health, and lifecycle mode separate. RPC outcomes distinguish timeout,
unavailable, rejected, and unknown non-idempotent outcomes; retries are limited to operations whose
semantics permit them.

The current gates cover optional predictor/self/workspace loss, required eventd loss, lifecycled
and presenced loss, owner timeouts, recovery, split commits, Plasma recreation, and reboot
continuity. They do not imply that distributed failure policies exist.

## Future lifecycle and epistemic failures

| Failure | Required future representation |
|---|---|
| governed epistemic reconciliation interrupted | accepted partial work plus explicit resumable state |
| stale perception source | affected claims become `Stale` or `Unknown` |
| contradictory observations | explicit `Disputed` projection; no silent last-write-wins |
| erasure cannot reach a replica | incomplete retention obligation and visible deficit |
| value policy unavailable | planning/priority capability unavailable; no fallback authorization |
| language faculty unavailable | typed Mind remains available without natural-language capability |

Recovery policy must distinguish unavailable capability, uncertain result, and verified failure.
See [Lifecycle](LIFECYCLE.md) and [Epistemic Governance](EPISTEMIC_GOVERNANCE.md).

The resilient transport boundary implements this distinction. Timeout of a non-idempotent
mutation is `UnknownOutcome`, while explicit refusal is `Rejected`; only safe operations may retry.
