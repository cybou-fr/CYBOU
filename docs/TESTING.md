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

The graphical `vm-smoke` asks systemd directly for a non-blocking reboot because the test-driver
implementation of `reboot()` sends Ctrl+Alt+Delete, which Plasma treats as an interactive logout.
This keeps shutdown non-interactive while preserving the normal state-flush path.

`m4-process-integration` now launches all nine Mind daemons. It changes Lifecycle1 from `awake` to
`idle` and back, then proves the signal-driven update reaches Presence1 and the QML-facing Presence
proxy, including lifecycled health, without turning lifecycle mode into runtime availability.
It also recreates the proxy three times around one active run and verifies unchanged run identity,
status, and Event1 count. `CYBOU_PRESENCE_INTERRUPT_DELAY_MS` is test-only fault injection:
presenced delays and rejects an interruption without touching Lifecycle1. A client heartbeat proves
the QML event loop remains responsive until the async transport timeout completes.

The same process test stops predictord, refreshes Health1, and verifies that Presence reports a
limited aggregate with only prediction unavailable. Identity, commitments, biography, attention,
and endpoint reachability remain usable; restarting predictord and refreshing Health1 restores the
prediction capability.

`lifecycle-scheduling-policy` covers invalid-evidence blocking, lifecycle deferral, required versus
optional capability loss, supported/unsupported trigger handling, and the 32/8 hysteresis boundary.
Process integration calls `EvaluateScheduling` through real D-Bus, verifies its reason reaches the
QML Presence proxy, and byte-compares Lifecycle1 state before and after the dry run.
It creates exactly enough external pressure to cross the 32-event threshold, verifies Health1
authorizes only `event-backlog-v1`, and observes `Run` through Lifecycle1 and Presence without a
state mutation. Protocol tests cover schema-v1 observation-only migration, invalid legacy true,
duplicate/invalid policy IDs, and future-schema rejection.

The same process scenario refreshes Health1 between evaluation and execution to prove stale
evidence fails without changing Lifecycle1 bytes. A current decision creates one deterministic
run; retries while active, after completion, and after a later run all return the same ID. Real
predictor/workspace dispatch and terminal completion close the path through Event1.

The production `RunSchedulingCycle` is exercised directly for both completed and quiet/deferred
outcomes. A failpoint crashes lifecycled after durable scheduled-run creation but before dispatch;
restart enters `Recovering`, resumes the same run, completes both owners, and leaves consumer
backlog at zero. Test environments disable timers/signals only to keep triggering deterministic.

Lifecycle unit and process tests also prove that Presence activity interrupts an automatic run,
persists its cooldown across lifecycled restart, and leaves a manual run active. Policy tests prove
that an otherwise authorized runnable decision defers until the cooldown expires.

`eventd-integration` proves consumer registration, exact backlog, idempotent/monotonic advancement,
rejection of backward and ahead-of-head offsets, invalid-ID rejection, and persistence across an
eventd restart. Lifecycle process integration proves a completed run advances its consumer while
its own owner/terminal contributions produce zero follow-up backlog. Health integration verifies
the backlog is a current typed measurement.

`p4-plasma-lifecycle` is the focused single-node VM gate for shell recreation. It restarts the
shipped Plasma user service around one active lifecycle run, requires a replacement PID and restored
D-Bus surface, and compares the exact run blob and Event1 count across the UI-only transition.

## Process integration

`m4-process-integration` runs inside an isolated `dbus-run-session`, launches nine executables,
and verifies:

```text
nine distinct process IDs
all organ D-Bus services become ready
two QML Presence proxies do not create another identity session
Promise crosses presenced -> intentiond -> eventd
Observe/Predict crosses presenced -> predictord -> eventd
Workspace receives accepted events
restarting identityd does not increment the same login session
restarting presenced leaves the cognitive organs alive
predictor loss limits only prediction and recovery restores it
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
