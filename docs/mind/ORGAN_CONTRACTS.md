<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Organ Contracts

M4 makes the source-level organ boundaries real process boundaries.

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

It talks to the organ interfaces and Event1. It does not construct domain organ objects or open
persistent cognitive stores.

## QML Presence proxy

Not an organ. It caches Presence1 Snapshot data and forwards commands for Plasma/QML.

Opening another surface creates another proxy, not another Mind.
