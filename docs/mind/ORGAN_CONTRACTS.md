<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Organ Contracts

Every organ declares purpose, inputs, outputs, owned state, capabilities, prohibited access, failure deficit, and recovery behavior.

## eventd

Owns the Journal, validates proposals, assigns order, appends, and publishes accepted contributions.

## identityd

Maintains identity and sessions. It must not overwrite damaged identity state or rewrite history.

## intentiond

Creates and resolves commitments from prior observations or decisions. It must not create self-caused intentions or close another lifecycle domain.

## predictord

Creates measurable predictions from evidence and settles each prediction at most once.

## selfd

Produces self-assessment only from measured facts and organ health.

## workspaced

Owns transient bounded attention, not biography.

## presenced

Creates stable UI projections. It must not own cognition or directly open organ databases.
