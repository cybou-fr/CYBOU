<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Next Engineering Steps

[Roadmap](ROADMAP.md) defines the milestones. [Current State](CURRENT_STATE.md) is the
implementation authority. This document is the short list of what to do next, and nothing else.

The plan that carried the Rust cutover is finished and has moved to
[Historical Execution](history/M5-M6.md) and [M7 execution](history/M7-EXECUTION.md). The C++/Qt
and NixOS trees it describes were removed on 2026-08-20.

## The objective

> **Make every claim the system makes about itself traceable to a source, before giving it a
> language.**

The previous barrier was wiring: owners existed and were connected to nothing. That is closed. The
present barrier is different and harder to see, because nothing looks broken while it holds: a
value that satisfies its type and states something nobody established. A random UUID passed as a
contribution identity, a `complete` flag that was the literal `true`, a readiness answer that stayed
true while the chain underneath it was broken.

## In order

1. **Authentication before any real personal biography.** `/api/v1/mind` is read-only and
   unauthenticated by an explicit, dated decision recorded in [Deployment](DEPLOYMENT.md). It
   already exposes identity, Journal metadata, commitment descriptions, Self narration, beliefs and
   attention. Read-only is not privacy-safe. This is the gate before the deployment holds anything
   that matters to a person.

2. **Debian service and reboot integration.** The NixOS VM gates made continuity and recovery
   claims and were removed with the composition they booted. Identity and lifecycle continuity
   across a real reboot, and recovery when a required owner is lost and returns under systemd, are
   currently proven by nothing. See [Testing](TESTING.md).

3. **ADR-0029 completeness for `Context1`.** Live and reconstructible today; still missing node,
   edge, depth, time and token budgets, privacy and retention inheritance, epistemic status, and
   invalidation on an erasure epoch. Until those exist the capability is partial and is described
   that way.

4. **A Debian-native desktop launcher, or an honest absence.** The Plasma packaging that stood in
   for the shell is gone. Either a small Cage/Chromium launcher lands in this tree, or the README
   keeps calling the desktop a target.

5. **M8 runtime on the frozen vocabulary.** `meaning.rs` and its siblings are contract-only: types
   with no runtime owner. Building on them before the four items above would add a layer making
   claims nobody can check, which is the failure this list exists to stop repeating.

## What not to do

Do not restore anything from the removed C++/Qt/Nix tree. Nothing installed those packages and no
Journal written by that implementation exists; the canonical byte fixtures it produced are checked
in and the tests verify against them.

Do not mark a milestone complete because the code exists. For most of this repository's life every
Mind owner existed, compiled in principle, and was connected to nothing.
