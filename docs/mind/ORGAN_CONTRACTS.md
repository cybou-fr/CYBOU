<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Organ Contracts

Every organ should have one narrow cognitive responsibility. Process isolation is the Target
architecture; current components are in-process C++ objects/libraries.

## Current implementation

### Identity

Maintains subject identity and session state in the current Presence-owned object graph.

### Intentions

Creates and resolves commitments using prior Journal contributions.

### Predictor

Records numeric observations, creates measurable predictions from persisted history, and can
settle a prediction with a terminal Outcome.

### SelfModel

Measures and narrates self-state from persisted/domain facts.

### Workspace

Maintains bounded transient attention and reconstructs it from recent Journal history.

### Presence

Is the normal UI boundary and exposes projections/commands to the Plasma surface. Today it also
constructs the current Mind object graph, which is a temporary lifecycle responsibility.

### Journal

Provides durable append, validation, ordering, migration, and verification. It is currently a
library object, not `cybou-eventd`.

## Target process contracts

### `cybou-eventd`

Exclusively owns the Journal, validates proposals, assigns durable order, appends, and publishes
accepted contributions.

### `cybou-identityd`

Owns identity/session state. It must not overwrite damaged identity state or rewrite biography.

### `cybou-intentiond`

Owns commitment lifecycle projections and commands derived from accepted contributions.

### `cybou-predictord`

Owns prediction/calibration behavior and settles each prediction at most once.

### `cybou-selfd`

Produces self-assessment only from measured facts and available organ health.

### `cybou-workspaced`

Owns transient bounded attention, not biography, and consumes the accepted-contribution stream.

### `cybou-presenced`

Creates stable UI projections and commands without owning cognition or opening organ-owned
persistent stores directly.

## Process-boundary rule

Source-directory names ending in `d` are not evidence that the daemon exists. A Target process is
implemented only when an executable/service, IPC contract, lifecycle behavior, and tests exist.
