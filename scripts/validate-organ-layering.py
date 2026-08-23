#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
"""No organ may reach down into a layer that answers to it.

ADR-0029 fixes an order and one rule over it:

    Journal          what happened
    epistemicd       what is known, and with what epistemic force
    contextd         what is related, and what is relevant now
    workspaced       what gets bounded attention
    meaning          how a selected context is interpreted or expressed

    Each layer may read the one above it and may not overrule it.

The rule is what stops a memory architecture from being decided by accident. Association reaching
into knowledge makes co-occurrence into fact (A5); attention reaching into association lets
relevance take a focus (A11); either one is a reasonable local decision that leaves no single
answer to "where does Mind's memory live".

Until 2026-08-23 the rule held because the wiring did not exist yet — contextd had no way to reach
epistemicd because contextd could not reach anything. That is not the rule holding; it is the rule
being untested, and it would have been discovered the first time somebody added a dependency for a
good reason. This makes the edge itself the thing that fails.

Checked at the manifest, deliberately. A `path` dependency is the only way one organ can name
another's types, and it is a fact in a file rather than an inference about code.

What this deliberately does not claim to see: an organ writing to a layer above it through the
fabric rather than by naming its crate. Reading upward is allowed by the rule, so contextd naming
epistemicd's types would pass here and should — what A5 forbids is contextd making something known,
not contextd knowing what is known. That direction is held by `protocol::promotion` instead, where a
candidate's evidence has to be episodes rather than associations. A check that guessed at the rest
would produce noise that gets ignored, which is worse than no check.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

CRATES = Path("crates")

#: The layers of ADR-0029, most authoritative first.
#:
#: A crate in a layer may depend on crates in layers above it. Depending on one below is the
#: failure this file exists to name.
LAYERS: list[tuple[str, set[str]]] = [
    ("journal", {"cybou-storage", "cybou-eventd"}),
    ("epistemic", {"cybou-epistemicd"}),
    ("associative", {"cybou-contextd"}),
    ("attention", {"cybou-workspaced"}),
    ("meaning", {"cybou-meaning"}),
]

#: Crates that are faculties rather than organs, and what each of them must not depend on.
#:
#: ADR-0035 gives the model broker a different bus namespace — `Faculty`, not `Mind` — and the
#: namespace is a claim: an organ of Mind owns part of what Mind is, and a faculty owns none of it.
#: A claim like that survives exactly as long as nothing makes it false by accident, and the way it
#: becomes false is a dependency added for a good reason. So it is checked rather than asserted.
#:
#: The rule is stronger than the layering one below: a faculty may not depend on *any* organ, in
#: either direction. Reading upward is what organs are allowed to do because they are part of the
#: same Mind; a faculty is not, and a faculty that could name an organ's types is one refactor away
#: from holding a piece of what it was built to stay outside of.
FACULTIES = {"cybou-model-brokerd"}

#: What each layer owns, for the error message. An operator reading a failure should not have to
#: open the ADR to know what was crossed.
OWNS = {
    "journal": "what happened",
    "epistemic": "what is known, and with what epistemic force",
    "associative": "what is related, and what is relevant now",
    "attention": "what gets bounded attention",
    "meaning": "how a selected context is interpreted or expressed",
}


def layer_of(crate: str) -> tuple[int, str] | None:
    """Which layer a crate belongs to, if any."""
    for index, (name, members) in enumerate(LAYERS):
        if crate in members:
            return index, name
    return None


def path_dependencies(manifest: Path) -> set[str]:
    """Every sibling crate this manifest names as a path dependency."""
    text = manifest.read_text(encoding="utf-8")
    return set(re.findall(r'^\s*([a-z0-9-]+)\s*=\s*\{[^}]*path\s*=\s*"\.\./', text, re.MULTILINE))


def organ_names() -> set[str]:
    """Every crate that is an organ of Mind."""
    return {crate for _, members in LAYERS for crate in members}


def main() -> int:
    violations = 0
    checked = 0
    organs = organ_names()

    for manifest in sorted(CRATES.glob("*/Cargo.toml")):
        crate = manifest.parent.name
        if crate in FACULTIES:
            checked += 1
            for dependency in sorted(path_dependencies(manifest)):
                if dependency in organs:
                    print(
                        f"error: {crate} is a faculty and depends on the organ {dependency}. "
                        f"ADR-0035: a faculty owns no part of Mind, which is why it is exported "
                        f"under org.cybou.Faculty and not org.cybou.Mind."
                    )
                    violations += 1
            continue

        here = layer_of(crate)
        if here is None:
            continue
        checked += 1
        depth, name = here

        for dependency in sorted(path_dependencies(manifest)):
            there = layer_of(dependency)
            if there is None:
                continue
            other_depth, other_name = there
            if other_depth > depth:
                print(
                    f"error: {crate} ({name}: {OWNS[name]}) depends on "
                    f"{dependency} ({other_name}: {OWNS[other_name]}), which answers to it. "
                    f"ADR-0029: each layer may read the one above it and may not overrule it."
                )
                violations += 1

    if violations:
        print(
            f"validate-organ-layering: {checked} checked, {violations} forbidden dependency(ies)"
        )
        return 1

    print(
        f"validate-organ-layering: {checked} organ(s) and faculty(ies), "
        f"no layer reaches downward and no faculty holds an organ"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
