<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Architecture Decision Records

| ADR | Title | Status |
|---|---|---|
| [0001](ADR-0001-system-architecture.md) | Body, Mind and Presence | Accepted |
| [0002](ADR-0002-cognitive-causality-and-journal-invariants.md) | Cognitive Causality and Journal Invariants | Accepted |
| [0004](ADR-0004-ci-workflow.md) | CI Workflow | Accepted |
| [0007](ADR-0007-reuse-3.x-compliance.md) | REUSE 3.x Compliance | Accepted |
| [0010](ADR-0010-journal-v2-schema-and-canonical-hashing.md) | Journal v2 Schema and Canonical Hashing | Accepted |
| [0011](ADR-0011-single-writer-event-journal.md) | Single-Writer Event Journal | Accepted |
| [0012](ADR-0012-organ-process-isolation-and-lifecycle.md) | Organ Process Isolation and Lifecycle | Accepted |
| [0014](ADR-0014-workspace-admission-and-global-attention.md) | Workspace Admission and Global Attention | Accepted |
| [0015](ADR-0015-terminal-outcome-semantics.md) | Terminal Outcome Semantics | Accepted |
| [0016](ADR-0016-identity-continuity.md) | Identity Continuity Across Sessions and Upgrades | Accepted |
| [0017](ADR-0017-cognitive-state-locations.md) | Cognitive State Locations and Ownership | Accepted |
| [0018](ADR-0018-privacy-classification-and-replication.md) | Privacy Classification and Replication | Proposed |
| [0019](ADR-0019-degraded-modes-and-capability-deficits.md) | Degraded Modes and Capability Deficits | Accepted |
| [0021](ADR-0021-language-models-are-optional-faculties.md) | Language and Models Are Optional Faculties | Accepted |
| [0022](ADR-0022-authorized-action-boundary.md) | Authorized Action Boundary for System Mutation | Accepted |
| [0024](ADR-0024-cognitive-lifecycle-and-consolidation.md) | Cognitive Lifecycle and Consolidation | Accepted |
| [0025](ADR-0025-grounding-epistemics-and-cognitive-governance.md) | Grounding, Epistemics, and Cognitive Governance | Proposed |
| [0026](ADR-0026-lifecycle-owner-and-wire-contract.md) | Lifecycle Owner and Wire Contract | Accepted |
| [0027](ADR-0027-local-epistemic-projection-owner.md) | Local Epistemic Projection Owner | Accepted |
| [0028](ADR-0028-retention-and-erasure.md) | Retention and Erasure in an Append-Only Journal | Accepted |
| [0029](ADR-0029-associative-context-projection.md) | Associative Context Projection and Semantic Activation | Accepted |
| [0030](ADR-0030-transparent-context-delivery.md) | Transparent Context Selection and Delivery | Accepted |
| [0031](ADR-0031-structured-meaning-and-cognitive-acts.md) | Structured Meaning and Cognitive Acts | Accepted |
| [0032](ADR-0032-layered-lifelong-learning.md) | Layered Lifelong Learning and Consolidation | Proposed |
| [0033](ADR-0033-learned-artifact-governance.md) | Learned Artifact Provenance, Promotion, Rollback, and Erasure | Proposed |
| [0034](ADR-0034-governed-agents-workers-and-tools.md) | Governed Agents, Workers, and Tool Use | Proposed |
| [0035](ADR-0035-governed-model-brokerage.md) | Governed Model Brokerage and External Inference | Proposed |
| [0036](ADR-0036-autonomous-security-control-plane.md) | Autonomous Security and Operations Control Plane | Proposed |
| [0037](ADR-0037-web-first-presence-and-desktop.md) | Web-First Presence and Chromium Desktop | Accepted |
| [0038](ADR-0038-rust-first-codebase.md) | Rust-First Product Codebase | Accepted |
| [0039](ADR-0039-debian-13-base-system.md) | Debian 13 Base System | Accepted |
| [0040](ADR-0040-spatial-card-desktop-and-bounded-body-capabilities.md) | Spatial Card Desktop (CYBOU Desktop vNext) and Bounded Body Capabilities (CYBOU Shell) | Accepted |
| [0041](ADR-0041-server-first-deployment.md) | Cybou Is a Server-Side Cognitive System | Accepted |
| [0042](ADR-0042-agent-capsule-platform.md) | Agent Capsules and the Agent Platform | Proposed |
| [0043](ADR-0043-model-gateway-for-external-agents.md) | A Model Gateway for External Agents | Proposed |

Numbers have gaps, and the gaps are deliberate. An ADR whose decision no longer constrains how Cybou
may be designed is deleted rather than kept as an entry nobody should read — the number stays retired
so an old reference is unambiguous, and `git log docs/adr/` still has the file. Eight went that way
on 2026-08-23: Plasma dock layouts, a Calamares profile, Nix state pinning, an in-process Qt
presentation architecture. What any of them still bound was moved into the ADR that superseded it
before the file was removed.

This table is checked against the directory. An ADR that exists and is not listed, a row that points
at nothing, or a status that disagrees with the document fails `scripts/validate-cognitive-docs.py`.
## Why some ADRs remain Proposed

Accepted outranks [Current State](../CURRENT_STATE.md), so a Proposed ADR describing behaviour the
code already enforces leaves the record contradicting itself. These do not:

- **0022 Authorized Action** and **0025 Grounding and Epistemics** describe boundaries deliberately
  not implemented. Proposed is the accurate status, and accepting them would assert a commitment
  nothing yet honours. 0025 is now partly narrowed by the Accepted
  [0027](ADR-0027-local-epistemic-projection-owner.md), which settles the owner it left open.
  0021 was in this group until 2026-08-23, when the meaning path it asked for was finished and it
  became Accepted.
- **0032 Layered Lifelong Learning**, **0033 Learned Artifact Governance**, **0034 Governed Agents
  and Tools**, **0035 Governed Model Brokerage**, **0036 Autonomous Security**, **0042 Agent
  Capsules** and **0043 Model Gateway** describe boundaries that deliberately precede their
  implementation. Writing them first is the point: they exist so the code can be reviewed against
  explicit invariants rather than the invariants being reconstructed afterwards from whatever the
  code turned out to do. 0031 was in this group until 2026-08-24, when the meaning boundary it asked
  for was finished in both directions and it became Accepted.

  0042 and 0043 are the newest of these and the largest. They rest partly on facts about an external
  ecosystem — a protocol, a registry, a multi-provider proxy — recorded with the date they were
  researched, because a decision quietly resting on a stale fact is the failure this repository keeps
  finding in itself. Both are arranged so that being wrong about one of those facts costs an adapter
  rather than an architecture.
- **0018 Privacy and Replication** is half enforced and half absent. The Journal rejects a
  contribution whose privacy is weaker than its references, so classification and inheritance are
  real; replication does not exist at all. It should be split before either half is accepted, rather
  than accepted as a whole and thereby overstating the second.
