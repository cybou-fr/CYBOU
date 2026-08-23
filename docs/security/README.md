<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Security and Privacy Documentation

- [Threat Model](THREAT_MODEL.md) — assets, adversaries, boundaries, and controls
- [Privacy Model](PRIVACY_MODEL.md) — classification, derivation, retention, and replication
- [Authorized Action Boundary](../adr/ADR-0022-authorized-action-boundary.md)
- [Grounding and Cognitive Governance](../adr/ADR-0025-grounding-epistemics-and-cognitive-governance.md)

## Current boundary

The runtime has a single Journal writer, causal/privacy validation, and separate user-session
processes. Same-user D-Bus is not yet a capability-security boundary. Distributed trust,
retention/erasure propagation, a language faculty, and privileged action execution are not current
features.

Security claims must follow [Current State](../CURRENT_STATE.md), not future diagrams or website
copy.
