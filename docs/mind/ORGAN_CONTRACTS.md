<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Organ Contracts

Source-level organ boundaries are real process boundaries.

## eventd

Owns the canonical Journal and Event1.

Responsibilities:

```text
validate proposal
serialize append
assign durable sequence/hash
commit
publish Accepted
answer Journal queries
```

## healthd

Owns the capability dependency graph and persistent Health1 snapshot. It probes components,
derives capability availability and deficits, publishes recovery progress and homeostatic
measurements, and never transitions lifecycle mode or executes maintenance.

## lifecycled

Owns lifecycle mode, persistent consolidation run state, deterministic owner dispatch, recovery,
user-activity cooldown, and evidence-bound scheduling. It delegates work to state owners and
records accepted effects through Event1; it does not own their projections or write Journal
directly.

## identityd

Owns `identity.json` and the volatile same-login session marker.

It begins one logical identity session per user runtime and resumes that session after an
identityd process restart.

## intentiond

Owns intention operations/projection:

```text
Form
Close
Open
```

Durable facts remain in Event1/Journal.

## predictord

Owns measurement/prediction/calibration operations:

```text
Observe
Predict
Settle
Calibrations
```

## selfd

Builds self projection from Identity1, Intention1, Predictor1, and Event1. It records
SelfAssessment contributions through Event1.

It does not instantiate local copies of those organs.

## workspaced

Owns bounded transient attention.

It rehydrates from Event1 on startup, follows Event1 Accepted live, and emits Workspace1 Changed
only after admission.

## presenced

Owns presentation aggregation and user-facing command routing.

It talks to the organ interfaces and Event1. It projects Health1 capability state, gates commands
by dependency, and applies one monotonic deadline to every compound read or mutation. It does not
construct domain organ objects or open persistent cognitive stores.

## Living Canvas Presence proxy

Not an organ. It caches Presence1 Snapshot data and forwards commands for Living Canvas.

Opening another surface creates another proxy, not another Mind.
