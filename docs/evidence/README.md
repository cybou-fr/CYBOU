<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Evidence

Each document here answers one question: **on what basis does Cybou claim this today?**

Not what was done, and not when. A record of past work belongs in `git log`, which is better at it
than prose is. What belongs here is the thing that would have to be re-run to check a claim that is
being made *now* — and every document below names the command.

That is the discipline the product itself is built on. Cybou distinguishes what happened from what
is currently held, and refuses to let a projection state what nobody established. Documentation that
narrated its own past while claiming to describe the present would be breaking, in the one place a
reader goes to find out what is true, the rule the system spends its whole design enforcing.

## Rule

> If the past does not constrain Cybou today, it does not belong in documentation.

What survives is never *we once did X*. It is the consequence of X that still binds: a schema that
existing data is written in, an invariant a change could destroy, a boundary that decides what is
allowed to be built.

## What is here

| Claim | Evidence |
|---|---|
| Data written by earlier builds still reads correctly | [Journal compatibility](journal-compatibility.md) |
| Erasure reaches derived state, not only the database | [Erasure gate](erasure-gate.md) |
| Identity and open commitments survive a restart | [Reboot continuity](reboot-continuity.md) |
| The browser surface is exercised, not assumed | [Desktop and browser gate](desktop-browser-gate.md) |
| Fourteen owners actually come up and answer together | [Debian integration](debian-integration.md) |

## What is not here

Claims nothing checks. If a document here could not name a command, the claim it was defending
should be withdrawn from wherever it is made rather than given a page.

Two claims are currently made with no evidence document, and both say so where they are made rather
than being quietly listed here: the native desktop session has never run on a machine with a seat,
and no model has ever been loaded through the brokerage faculty.
