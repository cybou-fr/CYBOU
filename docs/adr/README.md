<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Architecture Decision Records

| ADR | Title | Status |
|---|---|---|
| [0001](ADR-0001-system-architecture.md) | Body, Mind and Presence | Accepted |
| [0002](ADR-0002-cognitive-causality-and-journal-invariants.md) | Causality and Journal Invariants | Accepted |
| [0003](ADR-0003-ai-in-v0.1-none.md) | AI in v0.1 — None | Accepted |
| [0004](ADR-0004-ci-workflow.md) | CI Workflow | Accepted |
| [0005](ADR-0005-calamares-upstream-profile.md) | Calamares Upstream Profile | Accepted |
| [0006](ADR-0006-state-version-pinning.md) | State Version Pinning | Accepted |
| [0007](ADR-0007-reuse-3.x-compliance.md) | REUSE 3.x Compliance | Accepted |
| [0008](ADR-0008-mind-dock-with-tabs.md) | Mind Dock with Organ Tabs | Accepted |
| [0009](ADR-0009-one-presence-per-user-session.md) | One Presence per Session | Accepted |
| [0010](ADR-0010-journal-v2-schema-and-canonical-hashing.md) | Journal v2 | Accepted |
| [0011](ADR-0011-single-writer-event-journal.md) | Single-Writer Journal | Accepted |
| [0012](ADR-0012-organ-process-isolation-and-lifecycle.md) | Organ Process Isolation | Accepted |
| [0013](ADR-0013-local-cognitive-fabric-qt-dbus.md) | Qt D-Bus Fabric | Accepted |
| [0014](ADR-0014-workspace-admission-and-global-attention.md) | Workspace Admission | Accepted |
| [0015](ADR-0015-terminal-outcome-semantics.md) | Terminal Outcomes | Accepted |
| [0016](ADR-0016-identity-continuity.md) | Identity Continuity | Accepted |
| [0017](ADR-0017-cognitive-state-locations.md) | State Locations | Accepted |
| [0018](ADR-0018-privacy-classification-and-replication.md) | Privacy and Replication | Proposed |
| [0019](ADR-0019-degraded-modes-and-capability-deficits.md) | Degraded Modes | Accepted |
| [0020](ADR-0020-presence-surface-for-v0.1.md) | Presence Surface for v0.1 | Proposed |
| [0021](ADR-0021-language-models-are-optional-faculties.md) | Optional Language Faculties | Proposed |
| [0022](ADR-0022-authorized-action-boundary.md) | Authorized Action Boundary | Proposed |
| [0023](ADR-0023-mind-dock-discoverability-and-access.md) | Mind Dock Discoverability and Access | Accepted |
| [0024](ADR-0024-cognitive-lifecycle-and-consolidation.md) | Cognitive Lifecycle and Consolidation | Accepted |
| [0025](ADR-0025-grounding-epistemics-and-cognitive-governance.md) | Grounding, Epistemics, and Cognitive Governance | Proposed |
| [0026](ADR-0026-lifecycle-owner-and-wire-contract.md) | Lifecycle Owner and Wire Contract | Accepted |
| [0027](ADR-0027-local-epistemic-projection-owner.md) | Local Epistemic Projection Owner | Accepted |
| [0028](ADR-0028-retention-and-erasure.md) | Retention and Erasure in an Append-Only Journal | Accepted |

## Why five remain Proposed

Accepted outranks [Current State](../CURRENT_STATE.md), so a Proposed ADR describing behaviour the
code already enforces leaves the record contradicting itself. These five do not:

- **0021 Language Faculties**, **0022 Authorized Action** and **0025 Grounding and Epistemics**
  describe boundaries deliberately not implemented. Proposed is the accurate status, and accepting
  them would assert a commitment nothing yet honours. 0025 is now partly narrowed by the Accepted
  [0027](ADR-0027-local-epistemic-projection-owner.md), which settles the owner it left open.
- **0018 Privacy and Replication** is half enforced and half absent. The Journal rejects a
  contribution whose privacy is weaker than its references, so classification and inheritance are
  real; replication does not exist at all. It should be split before either half is accepted, rather
  than accepted as a whole and thereby overstating the second.
- **0020 Presence Surface for v0.1** describes a surface that has since changed substantially — the
  whole of it is now non-blocking, and its command set is declared in the capability registry. It
  needs revising against what Presence actually is before its status means anything.
