<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Testing Strategy

## M4 suite

The Mind package builds fourteen CTest suites:

```text
protocol
lifecycle
lifecycled-integration
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

`lifecycle` verifies atomic persistence, in-memory rollback after a failed save, persistent deficit
causes, and fail-closed recovery at the service-object boundary. `protocol` rejects unknown
lifecycle status values and round-trips the expanded run metadata.
`lifecycled-integration` runs a real daemon in an isolated D-Bus session, starts an active run,
restarts the process, verifies recovery with the same run identity, and rejects a duplicate D-Bus
owner. It also launches Event1, Predictor1, and Workspace1, dispatches a bounded consolidation run
to both owners, verifies two evidence-linked durable Event1 effects, repeats dispatch to prove the
event count is unchanged, and completes only after both typed receipts
are persisted. Completion adds exactly one terminal Event1 `Outcome`; its ID is retained in the
persistent Lifecycle1 run state.

The same process test uses `CYBOU_LIFECYCLE_FAILPOINT` to terminate lifecycled in the two
split-commit windows: `after-owner-commit` and `after-terminal-commit`. It restarts the daemon,
resumes the persisted run, repeats the operation, and asserts that Event1 count is unchanged by
replay. These failpoints are test instrumentation and are unset in the normal service environment.

`lifecycle-continuity` promotes both split-commit scenarios to KVM: it reboots after the owner
commit window and again after the terminal commit window, resumes the same persistent run, and
asserts that replay adds no Event1 contribution. The same test retains its original identity ID,
exact run blob, `Recovering`, and logical-session continuity proof.

`m4-process-integration` now launches all eight Mind daemons. It changes Lifecycle1 from `awake` to
`idle` and back, then proves the signal-driven update reaches Presence1 and the QML-facing Presence
proxy, including lifecycled health, without turning lifecycle mode into runtime availability.

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

`lifecycle-continuity` boots one headless NixOS node, creates an identity and active lifecycle run
through the real user D-Bus, reboots the machine, and proves the identity ID and exact persisted
run survive while the logical session advances and lifecycle enters `Recovering`.

`vm-smoke` boots the full Plasma session and SDDM greeter nodes. It starts
`cybou-presenced.service` through the user manager and asserts that all eight Mind user services
become active as separate processes. It repeats the continuity assertions in the desktop system
composition, but remains primarily the heavy service-graph and renderer gate.

## Build order

```bash
nix build .#packages.x86_64-linux.cybou-mind --print-build-logs
nix build .#packages.x86_64-linux.cybou-presence-applet --print-build-logs
nix build .#nixosConfigurations.cybou-vm.config.system.build.vm --print-build-logs
nix build .#checks.x86_64-linux.lifecycle-continuity --print-build-logs
nix build .#checks.x86_64-linux.vm-smoke --print-build-logs
```

Do not mark M4 complete from compilation alone.
