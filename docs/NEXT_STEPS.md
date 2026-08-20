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

1. **Authentication, when the tripwire says so.** Not now: the deployment holds facts about a
   machine, and a login would cost the demo and protect nothing. The gateway refuses to serve an
   unauthenticated public surface once the Journal holds anything above ordinary sensitivity, so
   the moment this stops being true is enforced rather than remembered. When it trips, this becomes
   the work — and it will trip on the first promise a person makes through the interface.

2. **Reboot integration.** `scripts/test-systemd-continuity.sh` now proves identity continuity, a
   noticed restart, an unshrinking Journal and recovery from the loss of a required owner — under
   the real units, against a deployed host. What it cannot prove is that the machine comes back:
   restarting a target is not a reboot. That needs a host that can be rebooted on demand.
   See [Testing](TESTING.md).

3. **ADR-0029 completeness for `Context1`.** Bounded by node and edge budgets, invalidated by an
   erasure epoch, and inheriting privacy and retention from evidence. What remains needs the
   activation session this version does not have — depth, time and token budgets, and seeds that
   are not only words. Until it exists the capability is partial and is described that way.

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
