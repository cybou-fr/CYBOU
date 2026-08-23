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

Browser tests, in real Chromium via `chromedriver`:

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
