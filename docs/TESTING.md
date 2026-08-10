<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Testing Strategy

## Mind CTest suite

The Mind package builds twenty CTest suites:

```text
protocol
health-protocol
homeostasis-protocol
health-service
healthd-integration
lifecycle
lifecycle-scheduling-policy
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
rpc-resilience
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
presenced consumes its shared server deadline before Lifecycle1 validation, then rejects without
touching Lifecycle1. A client heartbeat proves the QML event loop remains responsive until the
async transport reports `unknown-outcome`; the focused KVM gate also byte-compares persistent
lifecycle state before and after the delayed command and proves recovery succeeds.

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

The process suite delays predictor consolidation for two seconds after an automatic cycle returns
`started`. During that in-flight owner RPC, `NotifyUserActivity` must answer within one second and
persist `Interrupted`; after the delayed owner reply, the same run must remain interrupted and its
consumer backlog must remain unadvanced.

Optional-organ process coverage inspects the UI-ready `capabilityDetails` record during predictor
loss and recovery. It verifies typed cause, operational impact, dependency, verification time, and
the `waiting → verifying → ready` recovery progression while independent commands remain usable.
The same scenario validates `commandAvailability`, backend command enforcement, and the independent
state combinations `Awake + Limited` and `Recovering + Limited`.

P6.6 optional-organ coverage separately stops selfd and workspaced. Reflection and attention fail
before Event1 mutation, only their declared capabilities/commands become unavailable, unrelated
commands stay enabled, and two owner-backed refreshes prove verifying then ready recovery.

Boundary coverage stops lifecycled and presenced independently. Lifecycle control fails before an
Event1 mutation while identity and commitments remain usable. An already-awake QML proxy marks
runtime reachability false when presenced disappears, then reconnects with identical identity,
session count, Event1 count, and owner PIDs.

Scheduled timeout coverage keeps predictord registered on D-Bus but delays consolidation beyond a
200 ms test deadline. The required run becomes `Failed/Recovering`, backlog remains pending, late
retries produce no duplicate deterministic effect, and a post-recovery run completes the backlog.
Production keeps the five-second deadline.

P6.7 command-budget coverage runs the complete process matrix after RpcClient and EventClient adopt
strict timed pending calls. Promise, Reflect, Observe, Predict, Fulfill, and Abandon share one
monotonic budget per command rather than receiving a fresh timeout at every owner boundary. The
scheduled-owner scenario gives its explicit Health1 refresh orchestration call ten seconds because
the refresh itself owns a five-second production deadline; this changes only the test caller's
outer wait, not the production policy.

P6.7 snapshot coverage keeps selfd registered on D-Bus but suspends its process, restarts presenced
with a 500 ms command budget, and requests the full projection. The response must arrive in under
1.5 seconds, retain mandatory projection keys, expose empty self/lifecycle fields after exhaustion,
and avoid accumulating later owner deadlines. The test resumes selfd and restores normal presenced
configuration before continuing the fault matrix.

Two scoped `CYBOU_RPC_FAILPOINT` process cases terminate lifecycled after the first retryable
failure and after circuit-open. Each restart must project the original run ID in `Recovering`,
resume it, complete once, and leave zero consumer backlog with a stable Event1 count.

Required-owner coverage stops eventd while the other eight processes remain alive. It proves
Presence reachability with fail-closed biography/identity/commitment gates, rejects Promise before
acceptance, then verifies identical Journal count, UUID, session count, existing commitments, and
absence of the rejected description after restart.

`m6-recovery-boundary` is the focused P6.6 KVM exit gate. It boots the shipped Plasma session,
checks D-Bus/systemd activation of presenced without replacing plasmashell, proves a delayed
interruption cannot invent a lifecycle transition, then verifies successful recovery. Finally it
suspends eventd, refreshes Health1, and restarts presenced with a one-second compound-command
budget. Promise must fail closed inside a three-second external client deadline; the same owner is
then resumed and Event1 count and Plasma PID must remain unchanged. The current bounded transport
and remaining-budget implementation returns in under one second in this gate. Run it
with `nix build .#checks.x86_64-linux.m6-recovery-boundary --print-build-logs` on a KVM host.

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
`cybou-presenced.service` through the user manager and asserts that the eight services in its
activation graph (`eventd`, `lifecycled`, the five domain organs, and `presenced`) become active as
separate processes. Healthd behavior has its own process integration and focused M6 KVM coverage.
The gate repeats the continuity assertions in the desktop system composition, but remains
primarily the heavy service-graph and renderer gate.

## Build order

```bash
nix build .#packages.x86_64-linux.cybou-mind --print-build-logs
nix build .#packages.x86_64-linux.cybou-presence-applet --print-build-logs
nix build .#nixosConfigurations.cybou-vm.config.system.build.vm --print-build-logs
nix build .#checks.x86_64-linux.lifecycle-continuity --print-build-logs
nix build .#checks.x86_64-linux.vm-smoke --print-build-logs
```

Do not mark a milestone complete from compilation alone. `CURRENT_STATE.md` and `ROADMAP.md` may
claim completion only when the corresponding unit, process, VM/KVM, and documentation gates are
recorded and green.
