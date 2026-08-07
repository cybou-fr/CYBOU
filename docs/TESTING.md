<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Testing Strategy

## M4 suite

The Mind package builds twelve CTest suites:

```text
protocol
journal
identity
intentions
predictor
selfmodel
workspace
presence-proxy
m1-runtime
fabric-codec
eventd-integration
m4-process-integration
```

## Process integration

`m4-process-integration` runs inside an isolated `dbus-run-session`, launches seven executables,
and verifies:

```text
seven distinct process IDs
all organ D-Bus services become ready
two QML Presence proxies do not create another identity session
Promise crosses presenced -> intentiond -> eventd
Observe/Predict crosses presenced -> predictord -> eventd
Workspace receives accepted events
restarting identityd does not increment the same login session
restarting presenced leaves the cognitive organs alive
one organ failure does not kill the remaining processes
```

## VM gate

`vm-smoke` starts `cybou-presenced.service` through the user manager and asserts that all seven
Mind user services become active as separate processes.

## Build order

```bash
nix build .#packages.x86_64-linux.cybou-mind --print-build-logs
nix build .#packages.x86_64-linux.cybou-presence-applet --print-build-logs
nix build .#nixosConfigurations.cybou-vm.config.system.build.vm --print-build-logs
nix build .#checks.x86_64-linux.vm-smoke --print-build-logs
```

Do not mark M4 complete from compilation alone.
