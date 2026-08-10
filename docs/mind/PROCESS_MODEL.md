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

The narrow organs retain minimal `Ready()` and `Health()` probes. Health1 owns the component graph,
persistent health snapshot, capability availability, typed deficits, recovery progress, and raw
homeostatic measurements. Presence1 projects that state without owning it and gates each command
by its declared capability dependencies.

All compound Presence reads and mutations use one monotonic request budget. Optional-owner loss
therefore yields bounded partial data and capability-specific unavailability; required-owner loss
fails dependent mutations closed. Lifecycle mode and aggregate health remain independent axes.
