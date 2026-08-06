<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Current State

Status date: 2026-08-07.

## Implemented

- NixOS 26.05 configuration;
- KDE Plasma 6 and Wayland desktop foundation;
- Cybou Horizon branding;
- VM, ISO, and Hyper-V development targets;
- in-process C++ Mind prototype;
- Plasma Presence applet;
- Qt tests for domain components.

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

Daemon-like directory names do not yet mean independent services.

## Not implemented

- process-isolated organs;
- `cybou-eventd`;
- stable D-Bus contracts;
- Journal schema migrations;
- canonical full-envelope hashing;
- complete cause/evidence validation;
- terminal-outcome uniqueness;
- process-level degraded modes;
- inter-node distribution;
- language-model faculties;
- autonomous operating-system mutation.

## Documentation rule

Target architecture must be labeled **Target**. Current behavior must be supported by code and passing tests.
