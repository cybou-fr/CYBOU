<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Privacy Model

From most restrictive to least restrictive:

```text
Local
Node
Household
Public
```

**Local** remains on the originating device.

**Node** is available to trusted components on that node.

**Household** may be replicated to explicitly trusted household nodes.

**Public** may be displayed or exported subject to user policy.

A derived contribution is at least as restrictive as every cause and evidence item. Unknown values are treated as Local. Reject incorrectly weak declarations rather than silently exposing data.

## Lifecycle and derivation

Consolidation does not weaken privacy. Summaries, calibrations, reconciliations, embeddings, and
language context inherit the most restrictive applicable source classification. Logs should refer
to identifiers rather than duplicate sensitive payloads.

## Retention and erasure — M7 target

Classification answers who may access data; retention answers how long and in which forms it may
exist. Policy must cover source history, active projections, summaries, caches, backups, and
replicas.

An erasure operation is not complete while prohibited derived or replicated content remains
reachable. If deletion cannot be verified, Mind represents an outstanding privacy obligation rather
than claiming success. Exact tombstone and cryptographic-erasure semantics require a dedicated
implementation ADR.
