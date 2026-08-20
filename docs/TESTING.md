<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Testing

Two gates, answering two different questions. Keeping them apart is the point: one is about the
code, the other about the system that runs.

## What proves the code

The GitHub workflow runs on every push, on a portable runner:

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo check -p living-canvas --target wasm32-unknown-unknown --locked
```

It also builds the frontend with `trunk`, lints licence headers with `reuse`, and runs
`scripts/validate-cognitive-docs.py`, which checks that the documents this repository declares
authoritative still say what they claim and that their links resolve.

This gate cannot see most of the Mind. Everything behind `cfg(target_os = "linux")` — the twelve
daemons and every D-Bus surface — is not compiled there, so a green run says the portable half is
sound and nothing about the daemons.

## What proves the system

```bash
scripts/vps-checks.sh fast
scripts/vps-checks.sh release
```

This runs on the Debian 13 builder, compiles the whole workspace including the Linux-only half, and
then runs the multi-daemon integration gate. `release` additionally builds the release binaries and
the frontend artifact that a deployment installs.

The integration gate is the only place the Mind is exercised as a system. It re-executes itself
under `dbus-run-session` so the daemons, the PID list and the cleanup trap share one process, starts
all twelve owners, and then asserts:

- every organ answers `Ready`, because an organ that does not is indistinguishable from one that is
  down, and one missing method once pinned the whole control plane at unavailable;
- an intention survives a restart of the organ that holds it;
- a promise made through `Presence1` reaches `Intention1` **and** appears in the Journal as an
  `Intention` contribution;
- fulfilling it closes the obligation the command created, not whichever one happens to be first;
- `Epistemic1` and `Context1` name the subject that was observed, and never name an organ as one;
- `Workspace1` answers with a momentary state;
- the key material `eventd` wraps data keys with survives a restart of `eventd`;
- the control plane settles on healthy;
- killing `selfd` degrades it, both `Changed` signals fire, and restoring `selfd` returns it to
  healthy.

Each of those assertions exists because the property it names was once broken while everything
looked fine.

## What proves continuity under systemd

```bash
bash scripts/test-systemd-continuity.sh
```

Run against a deployed host, after a deploy. The integration gate starts the owners by hand under
`dbus-run-session`: a different manager, a different startup order, no unit dependencies. This one
uses the units that actually run, and asserts what only a real restart can falsify:

- the subject is the same after `systemctl restart cybou-mind.target` — an identity that changes
  across a restart is a different subject wearing the same biography;
- the session count advanced, because a restart the system did not notice is a restart it cannot
  account for;
- the Journal did not shrink;
- the control plane returns to healthy;
- stopping a required owner makes it stop calling itself healthy, and starting that owner again
  brings it back without intervention.

It touches the live Mind — it restarts the target and stops one owner — so it is a deliberate
post-deploy check rather than part of every gate run, and it puts the owner back even when an
assertion fails.

## What is not covered

- **A real reboot.** Restarting the target proves the owners recover from process death; it does
  not prove the machine can come back. That needs a host that can be rebooted on demand, which the
  removed NixOS VM gates used to provide.
- **The desktop session**, which has no implementation in this tree at all.

That is a real gap. It is recorded here rather than left to be inferred from a green run that was
answering a different question.

## Judging a milestone

A milestone is complete when the gates prove it, not when the code exists. The distinction is not
pedantic: for most of this repository's life every Mind owner existed, compiled in principle, and
was connected to nothing.
