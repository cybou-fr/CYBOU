<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Process Model

## Current

```text
cybou-eventd
cybou-healthd
cybou-lifecycled
cybou-identityd
cybou-intentiond
cybou-predictord
cybou-selfd
cybou-workspaced
cybou-presenced
```

Each is a real executable and D-Bus service managed by `systemd --user`. `healthd` owns only the
capability dependency graph and current persistent health snapshot. `lifecycled` owns only
lifecycle/run orchestration state; it does not own organ projections or Journal.

## Dependencies

```text
eventd
├── identityd
├── intentiond
├── predictord
├── workspaced
├── selfd
│   ├── identityd
│   ├── intentiond
│   └── predictord
└── presenced
    ├── identityd
    ├── intentiond
    ├── predictord
    ├── selfd
    └── workspaced
```

The services are D-Bus activated and stop with the graphical session.

## QML

`plasmashell` owns only a Presence proxy. Recreating the visual surface does not construct or
destroy cognitive organ processes.

## Health

M5 retains the minimal `Ready()` and `Health()` organ interfaces and adds lifecycle-level deficits.
P6.1/P6.2 add separate component-health and capability-availability contracts, an explicit
dependency graph, persistent Health1 ownership, and verified recovery. P6.3 adds bounded async RPC
for required non-blocking paths; broader migration and the read-only Health1 Presence projection
remain later work.
