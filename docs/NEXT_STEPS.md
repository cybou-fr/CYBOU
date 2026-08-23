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

1. **An inspector, so the record is readable by the person it is about.** *Done 2026-08-22, and
   read the limits below before treating it as finished.* `GET /api/v1/disclosure` answers for the
   caller and nobody else, and the `Disclosure` system card shows it: how much was supplied against
   how much of it can be accounted for, and every refusal with its reason. That is the gap ADR-0030
   asks for (B1, B6) — the part invisible in every system that assembles context silently. Before
   this the records were in the Journal and only a developer with `busctl` could read them.

   One thing was found while building it and is worth keeping in mind, because it is the shape this
   whole document is about: the gateway remembered a delivery only if it could also write it to the
   Journal, so a deployment without a sink told the person nothing had been supplied while it was
   supplying things. Remembering and recording are now separate. Having nowhere durable to write a
   delivery is a reason to say the audit trail is incomplete, never a reason to answer as though the
   delivery did not happen.

   Two smaller gaps remain, and the surface shows both rather than hiding them. A concept does not
   carry what it was derived from, so a delivery that supplied one says it supplied something it
   cannot account for; the count and the provenance are deliberately separate, and the card names
   the difference in words. And `retains` is false for every consumer today because there are no
   learning consumers yet — the field is reported rather than assumed so that the first consumer
   which does retain something is visible on the day it appears; when there is one, ADR-0033's A6
   needs these records to find what an erased payload influenced.

   Still missing: the record shown is the *last* delivery to this consumer, not their history. A
   person can see what they were supplied; they cannot yet see what they were supplied last week.

   Do not reach for `CYBOU_PUBLISHABLE_SENSITIVITY` to solve anything. It was raised on 2026-08-20
   for one stated reason — 1252 rows in the first Journal carried a constant sensitivity their
   content did not justify — with a comment saying to remove it once those rows were gone. The rows
   were discarded that same day and the raise outlived them, so the next thing above ordinary was
   published without anyone deciding to. It was taken back the same day in `4fa5788`, and the
   default is `Ordinary` again; this paragraph is kept as the reason not to raise it a second time.
   A temporary permission that survives its reason is the same failure as a claim that survives its
   evidence.

2. **Erasure beyond the live database.** *Partly done 2026-08-23.* An erasure now reports a typed
   `BackupState` rather than implying completeness: a deployment declares its rotation in the unit
   and the terminal `ErasureApplied` record carries what the erasure actually reached, with
   `unknown` as the default because silence about backups is not evidence that none exist. What
   remains is the part that needs software rather than a declaration — nothing in this tree takes a
   backup, so no rotation is being enforced by anything but a statement. The original text follows.

   **Erasure beyond the live database.** The executor exists: `Event1.RequestErasure` records a
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

4. **A Debian-native desktop launcher, or an honest absence.** *Partly done 2026-08-22.* The
   launcher landed: `scripts/cybou-desktop-session.sh` with `systemd/user/cybou-desktop.service`,
   installed by a deployment and left disabled, with a durable Chromium profile under
   `$XDG_STATE_HOME/cybou/desktop`. What is proven is narrow and listed in
   [Current State](CURRENT_STATE.md): the launcher refuses a silent gateway, refuses a missing
   browser, creates its profile directory, and can print what it would run. What is not proven is
   the part that matters — that Cage acquires a seat and shows the window on real hardware — because
   no machine with a seat was available. The README therefore still calls the desktop a target, and
   should keep doing so until someone runs the unit on one.

5. **The rest of M8.** *Partly done 2026-08-23.* `meaning::plan_status` builds a `ResponsePlan`
   from typed capability facts, and `ResponsePlan` gained a closed set of qualifications, so C5 has
   assertions behind it and `Realize` is reachable and used. `meaning::compose` joins plans that
   share an intent, carrying every qualification from every part, and refuses the joins that cannot
   be made honestly. What remains of the original text below is dialogue state across turns.

   **The rest of M8.** `cybou-meaningd` owns the boundary and is gated: an utterance becomes a
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
