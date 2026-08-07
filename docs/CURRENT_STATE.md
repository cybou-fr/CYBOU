<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Current State

Status date: 2026-08-07.

Repository snapshot:
`0ce6074e8b4b24ded982cfccec1572db0e8f397a`.

This file describes implemented behavior only. Target architecture belongs in
`ARCHITECTURE.md`, `ROADMAP.md`, and ADRs and must be labeled **Target** or **Proposed**.

## Implemented

### Body and desktop

- NixOS 26.05 configuration;
- KDE Plasma 6 / Wayland desktop foundation;
- Cybou Horizon branding;
- development targets for VM, live ISO, and Hyper-V;
- first-login layout versioning;
- a dedicated right-side Mind Dock loaded as a Plasma layout template;
- static KDE package validation and QML API validation.

### Mind

- an in-process C++ Mind object graph hosted by Presence inside `plasmashell`;
- Identity, Intentions, Predictor, SelfModel, Workspace, and Presence components;
- typed `CognitiveEnvelope` contributions;
- Observation as the only v2 root kind;
- rejection of self-causation, self-evidence, duplicate/null evidence, and direct-cause duplication;
- existence validation for cause and evidence references;
- restrictive privacy inheritance across references.

### Journal v2

- SQLite `PRAGMA user_version = 2`;
- explicit envelope schema and row hash versions;
- canonical v2 hashing of all semantic envelope fields;
- normalized evidence relations in `contribution_evidence`;
- `BEGIN IMMEDIATE` before write-side validation and tail selection;
- database-level uniqueness of one terminal `Outcome` per cause;
- v1 → v2 migration with retained `journal.db.v1.bak`;
- v1 hash preservation rather than historical rehashing;
- fail-closed migration for malformed legacy evidence, duplicate legacy terminal Outcomes,
  broken v1 history, partial schemas, and unsupported newer schemas.

### Automated validation

The `cybou-mind` package currently runs nine QtTest/CTest suites:

```text
protocol
journal
identity
intentions
predictor
selfmodel
workspace
presence
presence-extended
```

The fast GitHub Actions job on this snapshot passes formatting, REUSE, package metadata,
`cybou-mind`, `cybou-presence-applet`, and the post-format diff check.

The full `nix flake check` / VM job is tag-only and is not executed by an ordinary push.

## Current process topology

```text
plasmashell
└── Presence
    ├── Journal
    ├── Identity
    ├── Intentions
    ├── Predictor
    ├── SelfModel
    └── Workspace
```

Daemon-like source-directory names do **not** mean independent services yet.

## Current persistence ownership

Presence constructs the current object graph and opens the Journal and identity state below a
path derived from Qt `QStandardPaths::AppDataLocation`, with a `cybou` child directory.

This is intentionally documented as a current limitation. The target persistent Mind location is
`$XDG_STATE_HOME/cybou`, but ADR-0017 has not yet been implemented for Mind persistence.

The desktop first-login marker already uses `$XDG_STATE_HOME/cybou`; that does not mean the Mind
Journal has moved there.

## Current write and Workspace behavior

Identity, Intentions, Predictor, SelfModel, and Presence can write through the shared in-process
Journal. SQLite serializes concurrent local connections, but there is no single owning `eventd`
process.

Workspace has a correct bounded `publish()` path and can rehydrate from recent Journal history.
However, current organ writes normally call `Journal::append()` directly rather than publishing
through Workspace. Therefore Workspace is **not yet guaranteed to reflect every newly accepted
contribution live**; a rehydrate reconstructs the recent state.

This is the open part of M1.

## Not implemented

- one Presence backend enforced per user session;
- a single accepted-contribution stream feeding Workspace and Presence;
- stable Mind persistence under `$XDG_STATE_HOME/cybou`;
- `cybou-eventd`;
- exclusive single-process Journal ownership;
- stable local D-Bus contracts;
- process-isolated organs;
- process-level health and degraded-mode reporting;
- survival of Mind across `plasmashell` restart;
- inter-node distribution and reconciliation;
- language-model faculties;
- authorized autonomous operating-system mutation.

## Milestone position

- **M0 — Green Build:** fast gates are green; heavy full/VM validation remains a separate gate.
- **M1 — One Presence, One Journal:** open.
- **M2 — Journal v2:** complete.
- **M3 — eventd:** not implemented.

Milestones are architectural dependencies, not a claim that implementation work always landed in
numeric order.

## Documentation rule

A current-behavior claim must be supported by code and/or a passing test. A future requirement
must be labeled **Target**, **Proposed**, or **Pending**.
