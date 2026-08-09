<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Cybou Documentation

## Choose a reading path

### Product and status

- [Repository overview](../README.md)
- [Current State](CURRENT_STATE.md) — authoritative implementation boundary
- [Roadmap](ROADMAP.md) — capability progression, not a release promise
- [Next Engineering Steps](NEXT_STEPS.md) — executable work packages and exit gates
- [M5 Evaluation Evidence](M5_EVALUATION.md) — completed evaluation boundary, artifacts, and limitations
- [Installation](INSTALLATION.md) — development artifacts and safety constraints

### Architecture

- [Mind Model](MIND_MODEL.md) — conceptual model and invariants
- [Architecture](ARCHITECTURE.md) — topology, ownership, failure domains, ordering
- [Glossary](GLOSSARY.md) — normative vocabulary
- [ADR index](adr/README.md) — decisions and their acceptance status

### Build, test, and release

- [Building](BUILDING.md)
- [Testing](TESTING.md)
- [Development Workflow](DEVELOPMENT_WORKFLOW.md)
- [Next Engineering Steps](NEXT_STEPS.md)
- [Troubleshooting](TROUBLESHOOTING.md)
- [Upgrading](UPGRADING.md)
- [Release Process](RELEASE.md)

## Document roles

```text
MIND_MODEL.md     meaning, invariants, and long-term cognitive model
CURRENT_STATE.md  what the repository actually implements today
ARCHITECTURE.md   current topology plus explicit future boundaries
ROADMAP.md        sequencing and acceptance meaning of milestones
mind/*            protocol and component contracts
security/*        threat, privacy, and trust boundaries
adr/*             normative decisions; status matters
```

## Mind contracts

Read the [Mind index](mind/README.md), or open a specific contract:

- [Cognitive Protocol](mind/COGNITIVE_PROTOCOL.md)
- [Journal](mind/JOURNAL.md)
- [Organ Contracts](mind/ORGAN_CONTRACTS.md)
- [Data Ownership](mind/DATA_OWNERSHIP.md)
- [Process Model](mind/PROCESS_MODEL.md)
- [IPC](mind/IPC.md)
- [Workspace](mind/WORKSPACE.md)
- [Presence API](mind/PRESENCE_API.md)
- [Continuity](mind/CONTINUITY.md)
- [Failure Modes](mind/FAILURE_MODES.md)
- [Cognitive Lifecycle and Consolidation](mind/LIFECYCLE.md) — implemented M5 contract and future policy
- [Capability and Health Contract](mind/HEALTH.md) — P6.1/P6.2 protocol, owner, graph, and recovery
- [RPC Resilience](mind/RPC_RESILIENCE.md) — P6.3 outcomes, retry safety, backoff, and circuit breaker
- [Homeostatic Measurements](mind/HOMEOSTASIS.md) — P6.4 typed signals without scheduling authority
- [Grounding, Epistemics, and Cognitive Governance](mind/EPISTEMIC_GOVERNANCE.md) — future M6/M7 contract

## Security

- [Security index](security/README.md)
- [Threat Model](security/THREAT_MODEL.md)
- [Privacy Model](security/PRIVACY_MODEL.md)

## Documentation authority

When documents appear to conflict, use this order:

1. accepted ADRs and machine-enforced protocol/schema invariants;
2. `CURRENT_STATE.md` for implementation claims;
3. Mind contract documents for current interfaces;
4. `MIND_MODEL.md` and proposed ADRs for target architecture;
5. `ROADMAP.md` for planned sequencing;
6. website/marketing text only as a non-normative summary.

Proposed ADRs are design direction until accepted and implemented. A milestone becomes complete only
when repository gates and focused acceptance tests demonstrate it.

## Maintaining documentation

- Update `CURRENT_STATE.md` in the same change that alters implemented capability.
- Update the relevant contract before or with protocol/schema changes.
- Record cross-owner or security-boundary changes in an ADR.
- Label examples as current or future; do not present unfinished M6–M9 behavior as implemented.
- Add new canonical documents to `scripts/validate-cognitive-docs.py`.
- Keep relative Markdown links valid and SPDX metadata present.
