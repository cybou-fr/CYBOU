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

1. **Authentication. The tripwire has said so.** It tripped on 2026-08-20, on the first sentence
   spoken to `Meaning1` on the deployed host: the utterance and its interpretation are sensitivity
   1, the surface refused to publish them, and the public site went to 502. That is the mechanism
   working, and it is also the end of the arrangement it was protecting. A public surface and a
   Mind anyone can talk to are mutually exclusive while there is no login, because everything a
   person says is about the person. Either the deployment gets authentication, or nobody speaks to
   the deployed Mind — and the second is not a stable answer once there is anything to say.

   Do not reach for `CYBOU_PUBLISHABLE_SENSITIVITY` again. It was raised on 2026-08-20 for one
   stated reason — 1252 rows in the first Journal carried a constant sensitivity their content did
   not justify — with a comment saying to remove it once those rows were gone. The rows were
   discarded that same day and the raise outlived them, so the next thing above ordinary was
   published without anyone deciding to. A temporary permission that survives its reason is the
   same failure as a claim that survives its evidence.

2. **An executor for erasure.** ADR-0028 defines erasure, `Kind` has `ErasureRequested` and
   `ErasureApplied`, `Context1` discards its projection when the epoch advances, and the storage
   layer can read the epoch. Nothing raises it, and nothing removes a row: there is no path, at any
   level, from a person asking for something to be gone to it being gone. This was found the way
   the rest of the list was — by needing it. One test sentence on the deployed host could not be
   taken back, and the only available remedy was discarding the whole Journal. A system that keeps
   an append-only biography of a person and cannot delete from it is not finished being designed.

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
