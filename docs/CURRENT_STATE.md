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

The P3 transaction substrate now includes deterministic per-capability operation keys,
high-water-mark-bound idempotent acknowledgements, optional capability deficits, required-work
completion gates, and explicit resume of the same run after recovery. Lifecycle1 automatically
dispatches bounded `Consolidate` requests to Predictor1 and Workspace1 and validates their typed
receipts before persisting acknowledgements. Each owner resolves the exact accepted Event1 envelope
at the run high-water mark, commits an evidence-linked `Learning` contribution with a deterministic
UUIDv5 operation identity, and returns its contribution ID. Redelivery is a durable no-op, and the
integration suite proves two first-delivery contributions and zero duplicate contributions.
Lifecycle1 persists the capability-to-contribution mapping in the run, verifies every reference
against Event1, and refuses `Completed` until it has committed a deterministic terminal `Outcome`
caused by all owner results. The process integration suite verifies the extra terminal append and
exposes its ID through Lifecycle1 state. P3 still lacks the full VM fault-injection matrix across
each owner/terminal persistence boundary and Presence projection of lifecycle status. Process-level
fault injection now kills lifecycled immediately after an owner Event1 commit and immediately after
the terminal Event1 commit. In both cases restart enters `Recovering`, replay reuses deterministic
contributions, and Event1 count proves that no duplicate durable effect was created.
Lifecycle mutations roll back their in-memory candidate when persistence fails; unknown status
values fail protocol validation; optional deficit causes persist in the run; and the preferred
`RequestRunAtCurrentHead` API captures its accepted boundary directly from Event1.

The larger cognitive model and future agency architecture are described in `MIND_MODEL.md`.
The current M1–M4 plus partial M5 implementation is the process-isolated substrate of that model;
it does not yet
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
session marker and restarting the seven M4 processes. A focused booted NixOS VM gate already proves
identity and exact active-run continuity across a real reboot. Upgrade reconciliation and the full
lifecycle fault-injection matrix remain for the broader M5/M6 continuity proof.

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

- full M5 upgrade reconciliation and lifecycle reboot fault-injection matrix;
- lifecycle status projection through Presence;
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
- stronger upgrade/reconciliation guarantees belong to M5/M6;
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
- M5: in progress; P1/P2 are complete and the P3 transaction core is implemented, with Presence
  projection and the reboot fault-injection exit gate remaining.

See `ROADMAP.md` for the capability meaning of M5–M9.
