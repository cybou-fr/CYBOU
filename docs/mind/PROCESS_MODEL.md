<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Process Model

## Current

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

Each is a real executable and D-Bus service managed by `systemd --user`. `lifecycled` owns only
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

M4 exposes `Ready()` and `Health()` on organ interfaces. The richer
Available/Starting/Healthy/Degraded/Unavailable/Recovering model remains M6.
