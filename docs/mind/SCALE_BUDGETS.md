<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Journal Scale Baseline and Budgets

## Purpose

The [2026-08-10 checkpoint](../PROJECT_CHECKPOINT_2026-08-10.md) records "no performance envelope for
Journal and compound projections" as a P0 risk: correctness tests can pass while a cost grows without
bound. This document is the first measurement against that risk.

It is a baseline, not a target. The numbers below describe what the substrate does today; the
budgets derived from them say when that stops being acceptable.

## Method

`journal-scale` (`mind/tests/tst_journal_scale.cpp`) builds a deterministic fixture of N
contributions and measures the Journal paths that grow with history. The same index always produces
the same envelope, so two runs build identical journals and a measured difference is a real one.

Size is set by `CYBOU_SCALE_CONTRIBUTIONS` and defaults to 10,000 so the suite runs in the ordinary
checks. 100,000 belongs in a separate scale gate; 1,000,000 is a manual release benchmark.

The fixture is built through `Journal::appendBatch`, which shares one transaction — and therefore
one fsync — across a batch. Append cost is measured separately, one contribution per transaction,
exactly as Event1 accepts them. Building the fixture at production durability would take hours at a
million rows and would measure nothing that the append sample does not already measure honestly.

## Measured baseline

Recorded 2026-08-11 on the development host (x86_64, NixOS, SSD). Absolute values are
machine-specific; the per-contribution costs and their linearity are the transferable part.

| Measure | 10k | 100k | 1m | Per contribution |
|---|---:|---:|---:|---:|
| Fixture build (batched, 1000/commit) | 379 ms | 4,739 ms | 61,568 ms | ~62 µs |
| Append, one fsync each | — | — | — | **~1.2 ms** |
| Full replay `recent(0)` | 81 ms | 832 ms | 8,927 ms | **~8.9 µs** |
| Paged replay, 1000/page | — | 728 ms | — | ~7.3 µs |
| Full `Verify` | 109 ms | 1,036 ms | 10,915 ms | **~10.9 µs** |
| Incremental `Verify`, 500 new | — | 4 ms | — | independent of history |
| Predictor `allCalibrations`, first read | 94 ms | — | — | ~9.4 µs |
| Predictor `allCalibrations`, second read | **0 ms** | — | — | independent of history |
| Consolidation backlog count | 1 ms | 13 ms | 130 ms | ~0.13 µs |
| Indexed lookup (oldest / newest) | 5 / 5 ms | 5 / 4 ms | 5 / 5 ms | flat |
| Journal size | 3.6 MiB | 34.6 MiB | 347 MiB | **~364 bytes** |

Every growth-sensitive path is linear across two orders of magnitude. Nothing is quadratic, and
indexed lookup is flat — reaching the newest contribution costs the same as the oldest at every
size, so the indexes are doing their job.

## What the numbers say

**Append is dominated by fsync, not by work.** At ~1.2 ms per contribution the cost is one disk
sync; the validation, hashing and insert are noise beside it. This is the price of the durability
guarantee raised in P6.8, and it is the right price — but it caps sustained write throughput at
roughly 800 contributions per second, which is a real ceiling for any future perception adapter that
wants to observe continuously.

**Full replay is the expensive path, and it is paid repeatedly.** `recent(0)` costs ~8.9 µs per
contribution, and intentiond, predictord and selfd each replay the entire history to rebuild their
state on every start. At a million contributions that is ~9 s *each*, plus serialising ~347 MiB
across D-Bus three times. This is the concrete form of the deferred A4 finding: the design is
correct and does not scale.

**Paging does not make replay faster, and it was never going to.** Measured at 100k, paged replay
costs 728 ms against 762 ms for `recent(0)` — the same work, within noise. The same rows are read
and decoded either way; paging adds one query per page and removes nothing.

What paging buys is different and still worth having: memory no longer scales with history, because
no caller holds the whole biography at once, and across D-Bus there is no single reply carrying
hundreds of megabytes. Those are real failure modes, not slowness.

The consequence matters for sequencing: **paged replay alone does not move the cold-reconstruction
budget.** Nine seconds per organ at a million contributions is the cost of reading and decoding a
million rows, and no cursor changes that. Moving it requires organs to stop replaying everything —
that is what projection checkpoints are for, and this measurement is the evidence that they are
required rather than merely desirable.

**`Verify` reaches a user-visible path.** selfd calls it during ordinary self-assessment, which
presenced reaches through `Reflect` under a 5 s command budget. At ~10.9 µs per contribution,
verification alone consumes that entire budget at roughly **460,000 contributions**. Past that,
`Reflect` cannot succeed regardless of how healthy everything else is.

Incremental verification removes that scaling: anchored at a checkpoint, checking 500 new
contributions against a 100k journal costs 4 ms rather than 1,060 ms, because the cost follows what
has arrived since the last check rather than the whole history, and selfd uses it.

**That claim was previously stated too strongly here.** Verification stopped scaling with the
biography, but `Reflect` did not: `SelfModel::measure` built its subject list with `recent(0)` and
then called `Predictor::calibration` per subject, each doing its own `recent(0)` — roughly O(N x S),
worse than the verification cost it replaced. Fixing one cost on a path and assuming it was the only
one is how the wrong claim was reached.

That multiplication is now removed. `allCalibrations` accumulates every subject in a single pass and
selfd consumes that projection instead of scanning Event1 itself. `Reflect` is linear in the
biography rather than linear in the product — still not independent of it, which would need an
incremental subject index in predictord.

The optimisation has an honest limit, and it is the reason the result is typed. Incremental
verification trusts the prefix its checkpoint covers, so corruption inside that prefix is invisible
to it — a test tampers with an early contribution and confirms the incremental check still reports
intact while the full walk finds it. That is not a defect to be fixed but a property to be reported:
`VerifiedThrough` says a suffix was checked, `FullyVerified` says the history was rebuilt, and a
caller needing the stronger claim can tell which it received. A periodic full verification remains
the heavy integrity gate.

**Storage growth is modest.** ~364 bytes per contribution means a million contributions is ~350 MiB.
That is not the pressing constraint; time is.

## Budgets

These are the thresholds at which the current design stops being adequate. They are stated in
contributions rather than seconds so they can be checked against a real journal.

| Budget | Threshold | Consequence of crossing it |
|---|---:|---|
| `Verify` within the Presence command budget | ~~**~460k**~~ | Closed for verification. selfd verifies incrementally against a persisted checkpoint |
| `Reflect` within the Presence command budget | ~~**not measured**~~ | Closed. predictord answers from a cursor-carrying projection, so a read costs what arrived since the last one rather than the length of a life |
| Organ cold reconstruction within a plausible session start | **~500k** | Still binding at start, but paid once per process rather than once per question. intentiond replays paged, predictord and epistemicd carry cursors; only epistemicd persists a checkpoint across restarts |
| Sustained ingestion | **~800/s** | Above this, Event1 acceptance becomes the bottleneck and a perception adapter must batch or drop |
| Journal size on a normal disk | not binding below ~10m | ~3.5 GiB; time budgets bind long before storage does |

The two threshold numbers are close together, and both sit around half a million contributions. That
is the point at which paged replay and incremental verification stop being architectural preferences
and become required.

## What this does not measure

- RSS and per-organ cold reconstruction end-to-end. The Journal-level replay cost is measured; the
  organ-level cost of turning those envelopes into state is not, and will be larger.
- Presence snapshot and lifecycle consolidation under a large journal. Both are bounded by their own
  deadlines, so the question is not how slow they get but what they *stop* including — a correctness
  question that needs its own coverage.
- Concurrent read and write pressure. Every measurement here is single-threaded and uncontended.
- Growth rate in real use. Without knowing how fast a real biography accumulates, the thresholds
  above cannot be turned into a date. That is the most useful next measurement.
