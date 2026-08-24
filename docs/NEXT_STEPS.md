<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Next Engineering Steps

[Roadmap](ROADMAP.md) defines the milestones. [Current State](CURRENT_STATE.md) is the
implementation authority. This document is the short list of what to do next, and nothing else.

## The objective

> **Make every claim the system makes about itself traceable to a source, before giving it a
> language.**

ADR-0041 says what it is *for*. Cybou is a cognitive Linux environment for a VPS, server, VM or
container — a machine that runs unattended, is reached through a browser, and is expected to look
after itself. Outside this repository: **Linux that understands and operates itself.** Not Linux
with a local model, and not an AI desktop.

Two gates decide whether that is true, and everything below is ordered by them:

> **S0.** Cut internet access and every external model API. On a minimal VPS, Cybou continues to
> observe its Body, answer basic questions about its own state, detect known problems, explain them
> through evidence, remember its open intentions, and form typed action proposals.

> **S0R.** Restore the network and connect a large model. Language, analysis and planning improve
> sharply. Identity, memory, epistemics, permissions and the ability to maintain minimum system
> control do not change owner.

S0R is held by construction: nothing in the substrate loads a model, the broker is a faculty rather
than an organ, and a model that answers can only return proposals. **S0 is held up to the last
stage.** The vertical is:

```text
observe → understand → remember → diagnose → explain → propose → authorize → act → observe outcome
```

Every stage exists and is tested except one. *Act* is missing, and only *act*: what an outcome is
and how it is judged were built before the executor deliberately, so an executor arrives to find its
own report is one of two fields and not the deciding one.

## In order

0. **The executor, and the consent it is waiting for.** *Not started, and deliberately so.* Every
   stage before it is built: `cybou-remediation` proposes over a closed set of typed operations,
   criticises each proposal against the finding it claims to relieve, and decides it against a
   standing policy that grants nothing by default. What does not exist is anything that can carry
   one out.

   This is the one item in this document that is not blocked on engineering. Writing code that can
   mutate the host is a decision about what this machine is allowed to do to itself, and it is not
   a decision this repository should take on its owner's behalf by inference from *the boundary is
   ready*. The specific thing needed is a per-operation grant: which operations may exist at all in
   the first executor. `package.cache.clean` is the obvious candidate — reversible in the sense
   that matters, since what it deletes is re-downloadable — and `service.restart` is the first one
   that is genuinely not.

   When it exists it is `Action1` on a `cybou-executord`, and the shape is fixed by ADR-0022: a
   typed operation, never shell text; a capability check at the boundary rather than at the caller;
   and an **independent re-observation** afterwards. The last part is the one worth naming twice.
   An executor that reported its own success would be the system grading its own homework, and the
   evidence that the disk has space is a telemetry reading taken afterwards, not an exit code.

1. **Erasure beyond the live database.** *Partly done.* An erasure reports a typed `BackupState`
   rather than implying completeness, and E11 is now held by a test against a real copy of the
   database: the file is copied, the erasure runs, and the restored copy cannot read what the
   destroyed key protected. The precondition is a test too — a backup that also took the key store
   reads everything, and the default layout puts the two side by side, which the daemon now says at
   every start.

   What remains needs software rather than a declaration. Nothing in this tree takes a backup, so
   no rotation is enforced by anything but a statement, and E12 is answered by what a deployment
   declares rather than by what it does (ADR-0028 E12).

2. **A registered worker for the broker.** The brokerage faculty is built and answers `NoModel` on
   an installation with nothing registered, which is what ADR-0021 means by that being a
   configuration rather than a failure. No inference runtime is implemented beside it, on purpose:
   `llama.cpp`, `mistral.rs` and an ONNX runtime are three shapes of process, and a broker whose
   only backend was written next to it would have that backend's assumptions built in. The next
   step is one real worker behind the existing interface, not a second interface.

3. **The desktop launcher on real hardware.** `scripts/cybou-desktop-session.sh` and
   `systemd/user/cybou-desktop.service` exist and are installed disabled. What is proven is narrow
   and listed in [Current State](CURRENT_STATE.md): the launcher refuses a silent gateway, refuses
   a missing browser, creates its profile directory, and can print what it would run. What is not
   proven is that Cage acquires a seat and shows the window, because no machine with a seat has
   been available. The README calls the desktop a target, and should keep doing so until someone
   runs the unit on one.

## What not to do

Do not restore anything from the removed C++/Qt/Nix tree. Nothing installed those packages and no
Journal written by that implementation exists; the canonical byte fixtures it produced are checked
in and the tests verify against them.

Do not mark a capability complete because the code exists. For most of this repository's life every
Mind owner existed, compiled in principle, and was connected to nothing.

Do not reach for `CYBOU_PUBLISHABLE_SENSITIVITY` to solve anything. It was raised for one stated
reason — rows in an early Journal carried a constant sensitivity their content did not justify —
with a comment saying to remove it once those rows were gone. The rows were discarded that same day
and the raise outlived them, so the next thing above ordinary was published without anyone deciding
to. The default is `Ordinary` again. A temporary permission that survives its reason is the same
failure as a claim that survives its evidence.
