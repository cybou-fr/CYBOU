#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
"""Every class the desktop renders must have a rule.

Living Canvas draws its own window chrome: there is no browser furniture behind it and no
user-agent stylesheet that makes an unstyled element look deliberate. A `div` with a class nobody
styled falls into the document flow, and the person sees controls stacked in the corner rather than
a toolbar. That is not a cosmetic slip for a surface claiming to be an operating system's GUI; it
is the same failure as a projection stating what nobody established, in the one layer a person
actually looks at.

On 2026-08-22 this check was written because sixty-five rendered classes had no rule at all — the
whole topbar among them — while the stylesheet still carried a previous generation of components
that nothing renders any more. Neither half could be seen by the compiler, by `cargo test`, or by
the browser gate, because CSS is not code to any of them.

Checked in one direction only, deliberately. "Rendered but unstyled" is exact: the class is a
literal in the source and either has a rule or does not. The reverse — a rule nothing renders — is
not decidable here, because classes are also built at runtime from variables, and a check that
guesses produces noise that gets ignored, which is worse than no check.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

COMPONENT_ROOT = Path("crates/living-canvas/src")
STYLESHEET = Path("crates/living-canvas/styles.css")

#: Classes that reach the DOM without ever appearing as a literal beside `class=`.
#:
#: Each is built from a variable at runtime, so the scanner cannot see it and the rule cannot be
#: matched to a render site. They are listed rather than ignored so that adding one is a decision
#: somebody wrote down.
BUILT_AT_RUNTIME = {
    # `format!("object {}", card.key())` — the wrapper every card shares.
    "object",
    # Chosen in Rust for a withheld subject that could not be named.
    "withheld-unnamed",
}


def rendered_classes() -> dict[str, set[str]]:
    """Every class literal the components render, and where from."""
    found: dict[str, set[str]] = {}

    def note(name: str, source: Path) -> None:
        found.setdefault(name, set()).add(str(source))

    for source in sorted(COMPONENT_ROOT.rglob("*.rs")):
        text = source.read_text(encoding="utf-8")
        # `class="a b c"` — a plain literal.
        for match in re.finditer(r'class="([^"{}]+)"', text):
            for name in match.group(1).split():
                note(name, source)
        # `class:name=...` — Leptos conditional classes.
        for match in re.finditer(r"class:([a-zA-Z0-9_-]+)\s*=", text):
            note(match.group(1), source)
    return found


def card_keys() -> set[str]:
    """The per-kind classes `format!("object {}", key)` produces."""
    card = (COMPONENT_ROOT / "card.rs").read_text(encoding="utf-8")
    body = card.split("pub const fn key(self)", 1)
    if len(body) < 2:
        return set()
    return set(re.findall(r'=> "([a-z-]+)"', body[1].split("}", 1)[0]))


def styled_classes() -> set[str]:
    """Every class a rule mentions."""
    css = STYLESHEET.read_text(encoding="utf-8")
    # Strip declaration blocks so decimals like `.5` and units are not read as selectors.
    selectors = re.sub(r"\{[^}]*\}", " ", css)
    return set(re.findall(r"\.([a-zA-Z][a-zA-Z0-9_-]*)", selectors))


def main() -> int:
    styled = styled_classes()
    rendered = rendered_classes()
    # The per-kind class is an identity hook, not a style hook: it exists so a card can be found by
    # a test or a script, and deliberately does not make one card look different from another. It is
    # therefore not required to have a rule — cards look alike on purpose.
    identity_only = card_keys()

    unstyled = {
        name: sources
        for name, sources in rendered.items()
        if name not in styled
        and name not in BUILT_AT_RUNTIME
        and name not in identity_only
    }

    for name in sorted(unstyled):
        where = ", ".join(sorted(unstyled[name]))
        print(f"error: .{name} is rendered but has no rule in {STYLESHEET} ({where})")

    total = len(rendered)
    if unstyled:
        print(
            f"validate-desktop-styles: {total} rendered class(es), "
            f"{len(unstyled)} with no rule"
        )
        return 1

    print(f"validate-desktop-styles: {total} rendered class(es), all styled")
    return 0


if __name__ == "__main__":
    sys.exit(main())
