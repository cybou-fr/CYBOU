<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Journal compatibility

## The claim

Contributions written by an earlier build still decode, hash identically, and verify as part of the
same chain. A change to canonical encoding that broke this would not corrupt anything visibly — it
would make an existing biography unreadable while every new write looked fine.

## Why it constrains today

The canonical form is what the hash chain is computed over. Two builds that disagree about a byte
disagree about every hash after it, so the chain stops verifying from that point and the Journal
reports itself broken. Nothing recovers that: the earlier contributions are still there and can no
longer be shown to be the ones that were written.

This is why the encoding is pinned to fixtures rather than to a round-trip test. A round trip proves
a build agrees with itself, which is exactly what a build that changed the encoding also does.

## The evidence

Checked-in byte fixtures, compared against what this build produces:

```text
fixtures/protocol/envelope-v2.hex            schema v2 envelope
fixtures/protocol/envelope-v2-sha256.hex     its hash
fixtures/protocol/journal-row-v2.hex         v2 row
fixtures/protocol/journal-row-v3.hex         v3 row
fixtures/protocol/journal-row-v3-sha256.hex  its hash
fixtures/protocol/nonerasable-v3.hex         the part of v3 erasure cannot touch
fixtures/protocol/payload-v3-sha256.hex      the payload digest an erased row keeps
fixtures/protocol/commitment-v3.hex          commitment encoding
fixtures/storage/journal-writer-v3.txt       what the writer emits
```

Run it:

```bash
cargo test -p cybou-protocol -p cybou-storage --locked
```

## What this does not prove

That a v1 Journal reads. There is no v1 fixture because no v1 data exists outside development.
