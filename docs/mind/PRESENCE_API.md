<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Presence API

Presence is the only normal UI interface to the current Mind object graph.

This document separates the **Current C++/QML surface** from the **Target process API**.

## Current properties

`Presence.h` currently exposes these Qt properties:

```text
awake
narration
obligations
attention
contributions
stats
identityState
calibrations
coalitions
moment
```

They notify through the shared `changed` signal.

## Current read/projection methods

```text
recent(limit)                 # C++ helper
activity(limit)
detailedObligations()
stats()
identityState()
calibrations()
coalitions()
moment()
```

## Current commands with side effects

```text
promise(description)
reflect()
fulfillIndex(index)
abandonIndex(index)
observe(subject, value)
predict(subject)
```

Important semantics:

- `promise()` first records a user-request Observation and then forms an Intention;
- `reflect()` records an inspection-request Observation and then creates SelfAssessment;
- `observe()` writes a Predictor Observation;
- `predict()` writes a Prediction when enough persisted history exists;
- `fulfillIndex()` / `abandonIndex()` resolve the currently indexed open intention.

A method that writes biography must not be documented as read-only merely because it returns a
projection.

## Current lifecycle

Presence has C++ lifecycle/state methods including:

```text
wake()
isAwake()
lastError()
```

The current Plasma applet constructs Presence in-process. There is no independent `presenced`
service or reconnectable D-Bus API yet.

## Target API direction

The Target Presence process should expose stable read snapshots plus commands whose names make
side effects explicit, for example:

```text
status
organHealth
refreshSnapshot()

promise(description)
fulfill(intentionId)
abandon(intentionId)
requestPrediction(subject)
requestSelfAssessment()
```

Target rules:

- `refreshSnapshot()` must not write biography;
- opening a panel or switching a tab must not create SelfAssessment or a new session;
- command identity should use stable IDs rather than UI list indexes;
- Presence must survive/reconnect across UI lifecycle without becoming the owner of cognition.

Names in this Target section are design direction, not claims about the current `Presence.h`.
