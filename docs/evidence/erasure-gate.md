<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Erasure gate

## The claim

Erasing something removes it from the derived projections as well as from the database, and every
projection that was derived from erased evidence is discarded and rebuilt rather than answered from.

## Why it constrains today

An append-only Journal cannot delete a row, so erasure works by removing a payload and raising an
epoch. Everything derived — beliefs, associations, attention, context — was computed from what is
now gone, and a projection that kept answering would be reporting content that was erased, from
memory, indefinitely. The erasure would have removed the record and left the consequence.

This is the invariant a well-meaning optimisation destroys. Caching a projection across an epoch
change looks like an obvious win and is the exact failure.

## The evidence

Each derived owner checks the epoch and discards, and each has a test that it does:

```bash
cargo test -p cybou-eventd -p cybou-epistemicd -p cybou-contextd --locked
```

The multi-daemon gate raises a real epoch against running processes and checks that the projections
came back rebuilt rather than stale, and that `ErasureRequested` without `ErasureApplied` — an
erasure interrupted by a process dying — is not treated as an erasure that happened:

```bash
bash scripts/test-multi-daemon-integration.sh
```

The overflow case is tested too: an absurd retention window must not wrap into *already complete*.

## What this does not prove

That erased bytes are unrecoverable from the filesystem. Erasure here is a property of the Journal
and its projections, not of the storage medium underneath them.
