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

## What proves continuity across a real reboot

```bash
bash scripts/test-reboot-continuity.sh
```

Restarting the target proves the owners recover from process death. It cannot prove the machine can
come back: lingering, unit enablement, a user manager starting with nobody logged in, and state that
only exists because a directory happened to be warm are all untouched by restarting a target inside
a session that is already running.

This gate reboots the deployed host and then asserts that the Mind came back on its own, that the
identity is the same subject, that the session advanced and its start is a contribution Event1
holds, that the Journal did not shrink, that Event1 can still answer for its own chain, that the
control plane reaches healthy, and that the read-only surface serves again. The boot id is read on
both sides, because a gate that asserted continuity without establishing that the machine went down
would pass most convincingly when the reboot silently failed.

It takes the service down for as long as the host takes to come back, so it is run deliberately and
is not part of any other gate.

## What proves an account gets in and a stranger does not

```bash
sudo -E bash scripts/test-pam-access.sh
```

Every other gate here can run against fixtures. This one cannot: the point of `cybou-authd` is that
it consults the real shadow database through the real PAM stack, and a stub would prove only that
the stub agrees with itself. So it creates two throwaway accounts on the host it runs on, gives them
the same password, puts one in `cybou-access`, and checks what each gets. It refuses to run if those
accounts already exist, and removes them afterwards — run it on the disposable local builder, not on
a host anyone depends on.

It asserts that the permitted account is accepted, the same account with a wrong password is not,
a valid account outside the group is refused despite a correct password, `root` is refused, an empty
password is refused, an account that does not exist is refused, a locked account is refused, an
account removed from the group is refused, the socket is not world-reachable, and no password
reaches the helper's output.

The two that matter most are the ones a stub could not have told you: a correct password on a real
account outside the group is refused, and `usermod -L` closes the door.

## What is not covered

- **The desktop session**, which has no implementation in this tree at all.

That is a real gap. It is recorded here rather than left to be inferred from a green run that was
answering a different question.

## Judging a milestone

A milestone is complete when the gates prove it, not when the code exists. The distinction is not
pedantic: for most of this repository's life every Mind owner existed, compiled in principle, and
was connected to nothing.
