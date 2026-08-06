<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Presence API

Presence is the only normal UI interface to Mind.

## Read-only projections

```text
awake
status
narration
stats
identityState
obligations
activity
calibrations
coalitions
moment
organHealth
```

## Commands

```text
promise(description)
fulfill(intentionId)
abandon(intentionId)
requestPrediction(subject)
requestSelfAssessment()
```

Names must reveal side effects. A function that writes a Prediction is not a const getter.

`refreshSnapshot()` must update UI data without writing biography. Opening a panel or tab must not create SelfAssessment or a new session.
