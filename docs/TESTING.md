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

Until 2026-08-22 this section said the gate could not see most of the Mind, because everything
behind `cfg(target_os = "linux")` was "not compiled there". That was wrong, and wrong in the
direction that matters: the runner is Ubuntu, so `target_os = "linux"` holds, and the twelve
daemons and every D-Bus surface have been compiling and running their unit tests on every push all
along. The document understated its own evidence — which is the same failure as overstating it,
because a gate nobody believes in is a gate nobody reads.

What the workspace gate genuinely could not see was behaviour: a daemon compiles and its unit tests
pass without any of them ever having spoken to another daemon. That is what the job below now
covers.

## What proves the desktop is styled at all

```bash
python3 scripts/validate-desktop-styles.py
```

Living Canvas draws its own window chrome. There is no browser furniture behind it and no
user-agent stylesheet that makes an unstyled element look deliberate: a `div` whose class nobody
styled falls into the document flow, and the person sees controls stacked in a corner rather than a
toolbar. Neither the compiler, nor `cargo test`, nor the browser gate can see this, because CSS is
not code to any of them.

On 2026-08-22 sixty-five rendered classes had no rule at all — the entire topbar among them — while
the stylesheet still carried rules for a previous generation of components that nothing renders.
This check fails if a class a component renders has no rule. It is one direction only, deliberately:
"rendered but unstyled" is exact, while "a rule nothing renders" is not decidable from source, and a
check that guesses gets ignored.

## What proves the desktop

```bash
cargo test -p living-canvas --target wasm32-unknown-unknown
```

Everything under `crates/living-canvas/src/components` is `cfg(target_arch = "wasm32")`, so
`cargo test --workspace` compiles none of it. That gap is not small: three separate faults found on
2026-08-22 lived entirely inside it and no existing test could see any of them. Clicking one Shell
card selected every Shell card. Collapsing a card destroyed the terminal session inside it. The
minimap drew cards that were docked inside decks, and the stylesheet rules for its own elements did
not exist at all.

`src/interaction_gate.rs` mounts real components against real signals in a headless Chromium and
asserts on the DOM a person would have seen. It needs `wasm-bindgen-cli` at exactly the version in
`Cargo.lock` — the runner and the generated bindings must agree — and a WebDriver:

```bash
cargo install wasm-bindgen-cli --version 0.2.126 --locked
sudo apt-get install chromium chromium-driver
CHROMEDRIVER=/usr/bin/chromedriver cargo test -p living-canvas --target wasm32-unknown-unknown
```

`.cargo/config.toml` names `wasm-bindgen-test-runner` for that target, which is what makes `cargo
test` start a browser rather than fail to execute a `.wasm` file. The `desktop` job runs it on every
push.

The rule this gate exists to enforce is cheaper than the gate: **arithmetic over the layout belongs
in `layout/`, where it is tested natively, and components should only draw.** `layout::selection`
and `layout::minimap` were moved there for exactly that reason after their bugs were found.

## Checking the Linux half without a Linux machine

```bash
rustup target add x86_64-unknown-linux-gnu
cargo check -p cybou-web-gateway --all-targets --target x86_64-unknown-linux-gnu
cargo clippy -p cybou-web-gateway --all-targets --target x86_64-unknown-linux-gnu -- -D warnings
```

`cargo check` does not link, so the `cfg(target_os = "linux")` half — the zbus surfaces, the
daemons' service code — type-checks from any host with that target's standard library installed.
This is worth knowing before editing it: without it, a change to `presence_zbus` or a daemon is
written blind and found out on a push.

Two limits. `--workspace` does not work this way, because the crates that pull `rusqlite` build a
bundled SQLite and need a cross compiler; check those crates individually or leave them to CI. And
it type-checks only — the tests cannot run, because running them would mean running a Linux binary.
That is what the gates below are for.

## What proves the Mind as a system, on every push

```bash
bash scripts/test-multi-daemon-integration.sh
```

The `integration` job runs the same script the Debian builder runs. It re-executes itself under
`dbus-run-session`, starts all twelve owners, and asserts the properties listed under
[What proves the system](#what-proves-the-system) below. It needs `dbus-run-session`, `busctl` and
the PAM headers, all of which an Ubuntu runner has.

This is not a substitute for the builder. The script starts the owners by hand: no systemd units,
no unit ordering, no reboot. Those remain provable only on a deployed host.

## What proves the system

```bash
scripts/vps-checks.sh fast
scripts/vps-checks.sh release
```

This runs on the Debian 13 builder, compiles the whole workspace, and then runs the multi-daemon
integration gate against the distribution the system actually targets. `release` additionally
builds the release binaries and the frontend artifact that a deployment installs.

The integration gate is the only place the Mind is exercised as a system. Since 2026-08-22 it also
runs in CI on every push; the builder remains the authority on Debian 13 specifically, and on
everything a real `systemd` and a real reboot are needed to falsify. It re-executes itself
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

## What proves Desktop reliability and invariant-safe recovery

```bash
bash scripts/test-desktop-gate.sh
```

This gate runs 5 sequential verification stages covering the frontend and capability boundaries:

1. **Desktop and Living Canvas unit tests**: Verifies `DesktopLayout` v8-to-v9 migration, spatial geometry clamping, layout undo/redo history, and automatic self-healing normalization (`validate_and_normalize`) that recovers missing system cards and dissolves corrupt decks.
2. **Invariant-safe Deck model**: Verifies `DeckError` enforcement, preventing single-card decks, duplicate cards, and multi-deck conflicts.
3. **CYBOU Shelld confinement**: Verifies that `cybou-shelld` strictly executes only the ADR-0040 DemoReadOnly builtins — the set accepted in that ADR's Amendments 1 and 4 — and rejects everything else, including mutating and arbitrary commands, with code 127. This line said six until 2026-08-22, while the engine recognised thirteen; the set is enumerated in the ADR and this document points at it rather than restating it from memory.
4. **Web Gateway security boundaries**: Verifies that Public Preview mode strictly forbids shell access (HTTP 403) and serves only safe read-only projections.
5. **WASM32 target compilation and workspace Clippy**: Proves clean, zero-warning compilation for the browser runtime.

## What is not covered

- **The native Wayland desktop session manager**, which has no full compositor implementation in this tree yet (Living Canvas currently runs as a browser/PWA and web workstation surface).

That is a real gap. It is recorded here rather than left to be inferred from a green run that was
answering a different question.

## Judging a milestone

A milestone is complete when the gates prove it, not when the code exists. The distinction is not
pedantic: for most of this repository's life every Mind owner existed, compiled in principle, and
was connected to nothing.
