<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Desktop and browser gate

## The claim

The desktop surface is exercised where it runs — in a browser — rather than assumed to work because
it compiled.

## Why it constrains today

Three classes of code in this tree were once invisible to `cargo test`, and each produced a real
defect: components that only exist under wasm, CSS that nothing compiles, and Linux-gated daemons.
Living Canvas draws its own window chrome, so there is no user-agent stylesheet making an unstyled
element look deliberate — a class with no rule falls into the document flow and a person sees
controls stacked in a corner.

## The evidence

Browser tests, in real Chromium via `chromedriver` (`apt install chromium-driver` on Debian;
`scripts/gate.sh` looks for it at `/usr/bin/chromedriver`). Seven of them as of 2026-08-30:
the four that were there, the deck invariant, and two that could not be asserted anywhere else
— that a card with no component of its own is drawn with its own contents in it, and that a
card panned out of sight keeps its frame while dropping them:

```bash
cargo test -p living-canvas --target wasm32-unknown-unknown --locked
```

Every class the components render has a rule:

```bash
python3 scripts/validate-desktop-styles.py
```

Checked in one direction deliberately. *Rendered but unstyled* is exact; *a rule nothing renders* is
not decidable, because classes are also built at run time, and a check that guesses produces noise
that gets ignored.

## What this does not prove

That anything looks right. These catch a class with no rule and a component that panics; they say
nothing about whether the result is legible. Nor has the native Chromium/Wayland session ever run on
a machine with a seat — its unit ships disabled and that is stated wherever the desktop is claimed.

## Conditional desktop saves (2026-09-07)

Desktop layout reads now carry a content revision. A write supplies the revision it read;
SQLite checks it inside an immediate transaction, including across separate connections.
A stale write receives HTTP 409. Repeating the same payload after a lost acknowledgement
is idempotent. Existing stored layouts need no database migration; older clients cannot
unconditionally replace an existing layout.

The browser waits for its initial read before writing, sends at most one request at a time,
and acknowledges only the snapshot actually sent. Edits made during a read or write survive.
A conflict fetches the saved arrangement and offers **Keep this layout** or **Use saved layout**;
it does not merge geometry or resolve the choice automatically. Transient failures retry,
while malformed data and permanent refusals stop writes with a visible status. Navigation
warns when account layout changes are still pending.

Regression checks:

```bash
cargo test -p living-canvas --locked
cargo test -p cybou-web-gateway --lib --locked
CHROMEDRIVER=/usr/bin/chromedriver cargo test -p living-canvas --target wasm32-unknown-unknown --locked
```

Native tests cover late loads, edits during saves, stale writers, lost acknowledgements,
independent SQLite connections and reopening storage. The browser test exercises both conflict
buttons. This is not a full browser-to-gateway network-failure test, nor a proof of terminal
session restoration after reload.

## Files to Terminal (2026-09-07)

Files offers **Terminal here** only for a successfully listed host home directory. It creates
a new terminal instance and prepares its starting directory before mounting; existing terminals
keep their own state. The directory comes from the listing's `HostUserPath`, not a sandbox path
that happens to look like a host path. Navigation invalidates the launch context until it succeeds.

`OpenAt` carries an absolute host directory separately from keystrokes. Older owners refuse the
unknown variant instead of ignoring a new optional field. The owner resolves the directory and
passes it to the child command's `current_dir`; there is no shell interpolation or process-wide
chdir. Ordinary account filesystem permissions remain the boundary. Invalid directories start
nothing; accessible symlinks remain ordinary host paths. User shell profiles can still change cwd.

Verification: `cargo test -p cybou-ptyd --locked` exercises real shells in two concurrent
directories, including a name containing spaces, quotes and shell metacharacters; it reads back
`pwd -P` and checks the owner process's directory did not change. Gateway library tests carry an
`OpenAt` frame through a real WebSocket upgrade to a Unix socket without changing its bytes.
The Chromium gate clicks the Files button, verifies independent terminal context and refuses
launch context from an unread or sandbox directory. These are connected boundary checks, not a
single authenticated browser-to-shell deployment test. The starting directory lives in tool state;
terminal processes and that launch context are not restored across a full page reload yet.
