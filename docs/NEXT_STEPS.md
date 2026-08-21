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

1. **An inspector, so the record is readable by the person it is about.** Deliveries are recorded
   now: every supply of a projection across a boundary writes a `ContextDisclosed` naming the
   consumer, the contributions the supplied items came from, and what was held back and why. What
   does not exist is the surface ADR-0030 asks for — the person cannot see the gap between what was
   available and what was delivered, which is the interesting part and the one invisible in every
   system that assembles context silently (B1, B6). The records are in the Journal and only a
   developer with `busctl` can read them.

   Two smaller gaps behind it. A concept does not carry what it was derived from, so a delivery
   that supplied one says it supplied something it cannot account for; the count and the provenance
   are deliberately separate so that shows rather than hides. And `retains` is false for every
   consumer today because there are no learning consumers yet — when there are, ADR-0033's A6 needs
   these records to find what an erased payload influenced.

   Do not reach for `CYBOU_PUBLISHABLE_SENSITIVITY` to solve anything. It was raised on 2026-08-20
   for one stated reason — 1252 rows in the first Journal carried a constant sensitivity their
   content did not justify — with a comment saying to remove it once those rows were gone. The rows
   were discarded that same day and the raise outlived them, so the next thing above ordinary was
   published without anyone deciding to. A temporary permission that survives its reason is the same
   failure as a claim that survives its evidence.

2. **Erasure beyond the live database.** The executor exists: `Event1.RequestErasure` records a
   durable request, destroys the keys, redacts the payload of the target and everything derived
   from it, advances the epoch, and records that it happened. An interrupted erasure finishes on
   the next start, because the request is on record precisely so nobody has to remember. What is
   not covered is what ADR-0028 says plainly: a backup taken before an erasure still holds the
   ciphertext, and only a destroyed key reaches it. Nothing in this tree yet takes a backup, so
   nothing yet declares which rotation is inside the guarantee (E11, E12).

3. **ADR-0029 completeness for `Context1`.** Bounded by node and edge budgets, invalidated by an
   erasure epoch, and inheriting privacy and retention from evidence. What remains needs the
   activation session this version does not have — depth, time and token budgets, and seeds that
   are not only words. Until it exists the capability is partial and is described that way.

4. **A Debian-native desktop launcher, or an honest absence.** The Plasma packaging that stood in
   for the shell is gone. Either a small Cage/Chromium launcher lands in this tree, or the README
   keeps calling the desktop a target.

5. **The rest of M8.** `cybou-meaningd` owns the boundary and is gated: an utterance becomes a
   typed act or none, a reference stays unresolved rather than being guessed, corrections append,
   and prose comes from a `ResponsePlan` and nothing else. What is missing is the middle. Nothing
   builds a `ResponsePlan` from Mind state, so `Realize` is reachable and unused and ADR-0031's C5
   has no assertion behind it. There are no composition operators and no dialogue state across
   turns. The vocabulary is a list of openings a person can read and disagree with, which is the
   point, and it is small.

## What not to do

Do not restore anything from the removed C++/Qt/Nix tree. Nothing installed those packages and no
Journal written by that implementation exists; the canonical byte fixtures it produced are checked
in and the tests verify against them.

Do not mark a milestone complete because the code exists. For most of this repository's life every
Mind owner existed, compiled in principle, and was connected to nothing.
