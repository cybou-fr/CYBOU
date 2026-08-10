<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Cybou Mind Documentation

Before reading the implementation contracts, read the repository-level
[Mind Model](../MIND_MODEL.md). It explains how biography, identity, commitments, prediction,
self-model, attention, optional faculties, and future authorized action fit together.

Then read the implementation documents in this order:

1. [Cognitive Protocol](COGNITIVE_PROTOCOL.md)
2. [Journal](JOURNAL.md)
3. [Organ Contracts](ORGAN_CONTRACTS.md)
4. [Data Ownership](DATA_OWNERSHIP.md)
5. [Process Model](PROCESS_MODEL.md)
6. [IPC](IPC.md)
7. [Workspace](WORKSPACE.md)
8. [Presence API](PRESENCE_API.md)
9. [Continuity](CONTINUITY.md)
10. [Failure Modes](FAILURE_MODES.md)
11. [Cognitive Lifecycle and Consolidation](LIFECYCLE.md) — implemented M5/M6 contract
12. [Capability and Health Contract](HEALTH.md) — implemented P6.1/P6.2 protocol and owner boundary
13. [RPC Resilience](RPC_RESILIENCE.md) — implemented P6.3/P6.7 transport and compound-budget policy
14. [Homeostatic Measurements](HOMEOSTASIS.md) — implemented P6.4/P6.5 measurement and scheduling input
15. [Grounding, Epistemics, and Cognitive Governance](EPISTEMIC_GOVERNANCE.md) — future M7 contract

Documents 1–14 specify or explain the implemented M1–M6/P6.7 substrate. Document 15 is a
future-target contract and must not be used as evidence that M7 behavior exists.

For current implementation status, see [Current State](../CURRENT_STATE.md).
For milestone semantics, see [Roadmap](../ROADMAP.md).
