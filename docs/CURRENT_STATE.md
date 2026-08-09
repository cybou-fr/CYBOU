<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Current State

Status date: 2026-08-09.

This document is intentionally limited to implemented behavior and current limitations.

## Repository gate status

The P0 baseline is green: formatting, REUSE 3.3, package metadata, cognitive documentation, Mind
access, QML API, UI polish, `cybou-mind`, and `cybou-presence-applet` pass through pinned Nix checks.
The Mind package runs fourteen CTest suites, including Event1, lifecycle persistence/recovery,
Lifecycle1 process restart, and seven-process M4 integration. The process suite also proves a
simulated new login preserves identity and an accepted open intention while incrementing the
logical session count.

The P2 lifecycle owner is present: lifecycle schema v1, legal mode transitions, atomic persistent
run state, `org.cybou.Mind.Lifecycle1`, D-Bus/systemd activation, D-Bus run requests, and restart
recovery of an active run into `Recovering`. Legacy v0 state is backed up and migrated to v1;
unknown future versions fail closed. The focused headless NixOS gate proves that a real reboot
preserves the exact persisted run and identity ID, enters `Recovering`, and increments the logical
session count. P2 is complete; owner-work dispatch and real consolidation remain P3.

The first P3 transaction substrate is implemented in Lifecycle1: deterministic per-capability
operation keys, high-water-mark-bound idempotent acknowledgements, optional capability deficits,
required-work completion gates, and explicit resume of the same run after recovery. Predictor and
Workspace owner-side consolidation handlers and automatic coordinator dispatch are not yet
implemented.

The larger cognitive model and future agency architecture are described in `MIND_MODEL.md`.
The current M4 implementation is the process-isolated substrate of that model; it does not yet
contain the planned M8 language faculty or M9 authorized executor.

## Process topology

Mind now has eight real user-session processes (the seven M4 processes plus the P2 lifecycle owner):

```text
cybou-eventd
cybou-lifecycled
cybou-identityd
cybou-intentiond
cybou-predictord
cybou-selfd
cybou-workspaced
cybou-presenced
```

`plasmashell` no longer constructs Identity, Intentions, Predictor, SelfModel, Workspace, Journal,
or EventClient. It loads a lightweight `Presence` QObject whose runtime job is Presence1 IPC and
QML property caching.

## Ownership

| Resource / responsibility | Owner |
|---|---|
| `journal.db` | `cybou-eventd` |
| lifecycle mode and run state | `cybou-lifecycled` under `$XDG_STATE_HOME/cybou/lifecycle` |
| `identity.json` | `cybou-identityd` |
| identity login marker | `cybou-identityd` under `$XDG_RUNTIME_DIR/cybou` |
| intention commands/projection | `cybou-intentiond` |
| prediction/calibration | `cybou-predictord` |
| self projection/assessment | `cybou-selfd` |
| bounded attention | `cybou-workspaced` |
| presentation aggregation | `cybou-presenced` |
| visual cache | Plasma Presence proxy |

There is currently no language-model process and no privileged action-executor process in this
ownership table.

## IPC

Versioned Qt D-Bus interfaces:

```text
org.cybou.Mind.Event1
org.cybou.Mind.Lifecycle1
org.cybou.Mind.Identity1
org.cybou.Mind.Intention1
org.cybou.Mind.Predictor1
org.cybou.Mind.Self1
org.cybou.Mind.Workspace1
org.cybou.Mind.Presence1
```

Complex organ projections use fabric CBOR version 1. Event1 CognitiveEnvelope encoding remains
separate from generic projection encoding and from canonical Journal hashing.

## Lifecycle

The NixOS module installs each organ as a `systemd --user` `Type=dbus` service. The services are
D-Bus activated.

They are intentionally not eagerly wanted by the graphical target: the Plasma-hosted proxy first
performs the one-time pre-M1 state-location migration, then its first Presence1 request can
activate the Mind graph.

Identity uses a volatile runtime-session marker. Restarting `identityd` inside the same user login
reloads the current identity without incrementing `sessionCount`.

The process integration suite additionally simulates a new login by removing only the volatile
session marker and restarting the seven M4 processes. A booted VM reboot proof remains required
before this becomes the full M5/M6 continuity proof.

## Durable-to-visible ordering

```text
command
→ owning organ process
→ Event1
→ eventd
→ Journal COMMIT
→ Event1 Accepted
→ workspaced admission
→ Workspace1 Changed
→ presenced Changed
→ QML proxy refresh
```

This is the implemented form of the `durable before visible` invariant.

## Current cognitive substrate

The present tree has implementation boundaries for:

- canonical durable causal history;
- identity ownership;
- intention state;
- prediction/calibration state;
- self projection/assessment;
- bounded Workspace attention;
- presentation aggregation;
- process-level health/failure isolation.

These components are intentionally useful without any language model.

## Not implemented yet

The current tree does **not** yet implement:

- full M5 restart/reboot/upgrade continuity proof and reconciliation;
- explicit cognitive lifecycle modes or a consolidation coordinator;
- background consolidation, retention, forgetting, or temporal freshness policy;
- M6 explicit degraded-Mind capability-deficit policy;
- homeostatic pressure signals or metacognitive uncertainty/freshness projection;
- M7 inter-node transport, replication, or partition handling;
- typed perception adapters, epistemic claims, contradiction reconciliation, or value constraints;
- M8 optional language faculty;
- M9 planning/authorization/executor pipeline for privileged external actions.

A UI or current organ method should not be described as providing those future capabilities unless
the corresponding milestone is implemented and gated.

## Current limitations

- health is a minimal `Ready()/Health()` contract, not the full M6 degraded-mode model;
- most local RPC is synchronous;
- same-user IPC authorization is not yet a capability security boundary;
- stronger restart/reconciliation guarantees belong to M5/M6;
- `awake` is currently a presentation/runtime property, not the lifecycle state machine proposed by
  ADR-0024;
- Journal history is not yet consolidated into a governed epistemic projection;
- privacy classification exists, but retention and erasure propagation are not implemented;
- no inter-node transport exists;
- no model-selection/context policy for M8 exists;
- no authorization policy or typed privileged executor for M9 exists.

## Milestones

- M1: complete.
- M2: complete.
- M3: complete after the M3 compile repair included by M4.
- M4: implementation present; repository gates remain the acceptance authority.
- M5: next; now includes continuity, lifecycle modes, and consolidation foundations.

See `ROADMAP.md` for the capability meaning of M5–M9.
