<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Journal Scale Baseline and Budgets

## Purpose

The 2026-08-10 project checkpoint recorded "no performance envelope for
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
| `Intentions::open`, first read | 86 ms | — | — | ~8.6 µs |
| `Intentions::open`, second read | **0 ms** | — | — | independent of history |
| Consolidation backlog count | 1 ms | 13 ms | 130 ms | ~0.13 µs |
| Indexed lookup (oldest / newest) | 5 / 5 ms | 5 / 4 ms | 5 / 5 ms | flat |
| Journal size | 3.6 MiB | 34.6 MiB | 347 MiB | **~364 bytes** |

Every growth-sensitive path is linear across two orders of magnitude. Nothing is quadratic, and
indexed lookup is flat — reaching the newest contribution costs the same as the oldest at every
size, so the indexes are doing their job.

## The Rust writer, measured against the same paths

`cybou-journal-scale` is the Rust counterpart of `journal-scale`: the same deterministic fixture
shape, the same batched build, the same separate one-commit-each append sample. Until it existed,
the Rust stack had budgets it had never been held to.

Recorded 2026-08-19 on the development host (x86_64, Windows/NTFS, NVMe), release profile. Payload
is ~108 bytes per contribution.

| Measure | 10k | 100k | Per contribution |
|---|---:|---:|---:|
| Fixture build (batched, 1000/commit) | 160 ms | 1,621 ms | ~16 µs |
| Append, one commit each (50 sample) | — | — | ~330 µs |
| Full verification | 39 ms | 367 ms | ~3 µs |
| Paged verification, 1000/page | 46 ms | 466 ms | ~4.6 µs |
| Journal size | 5.1 MiB | 51 MiB | ~534 bytes |

**Linear, and that is the transferable result.** Build, verification and size per contribution are
flat across an order of magnitude, and paged verification tracks full verification within noise —
the same finding the C++ measurement produced, reproduced through an independent implementation.

**The absolute numbers are not comparable to the table above, and must not be read as a
comparison.** Different machine, different filesystem, different fsync semantics, and a different
payload size, any one of which would account for the gap on its own. That the Rust append sample
came in at ~330 µs where the predecessor measured ~1.2 ms says something about the two hosts, not
about the two writers. A real comparison needs both binaries on the same Debian host against the
same fixture, and that run has not happened.

**Not yet measured on this side:** a million contributions, concurrent read/write pressure, RSS, and
cold organ reconstruction end to end — the last of which has no Rust owner to reconstruct yet.

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

## What a connected biography costs

Every measurement above uses a fixture of root observations: no causation, no evidence, nothing to
look up on the way in. Mind never writes that. A second fixture builds five-contribution episodes -
root observation, prediction citing it, intention citing both, outcome settling the prediction,
prediction citing the rest - so each append pays the reference lookups and privacy inheritance that
Event1 actually performs.

Measured in the same run, at 10k, so the two are comparable:

| Measure | Flat | Connected |
|---|---:|---:|
| Fixture build (batched, 1000/commit) | 435 ms | **1,017 ms** |
| Full replay `recent(0)` | 92 ms | 101 ms |
| Full `Verify` | 122 ms | 140 ms |

**The cost of connection is paid on the way in, not on the way out.** Building costs 2.3x more,
because each non-root contribution resolves its cause and every evidence id and then checks that its
privacy equals the most restrictive of them. Reading costs 10-15% more, which is the wider rows and
the evidence join.

Not reported as a result: per-contribution append came out at 0.935 ms connected against 1.790 ms
flat in the same run. That ordering is backwards from what the build number implies, and both are
fsync-bound at a scale where run-to-run variance exceeds the difference. It is noise, and recording
it as a finding would be inventing one.

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
