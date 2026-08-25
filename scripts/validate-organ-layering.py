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
    # Above the Journal, because what a host is doing right now is upstream of what happened to it:
    # telemetry proposes findings that may become contributions, and never reads one.
    ("telemetry", {"cybou-telemetryd"}),
    ("journal", {"cybou-storage", "cybou-eventd"}),
    ("epistemic", {"cybou-epistemicd"}),
    ("associative", {"cybou-contextd"}),
    ("attention", {"cybou-workspaced"}),
    ("meaning", {"cybou-meaning"}),
    # Last, and below meaning on purpose. What may be done is decided after what is known, what is
    # related, what has attention and what is being said — and nothing above it may read it, because
    # a layer that could consult the authorization gate could come to depend on being permitted.
    #
    # `cybou-capsule` is here for the same reason and answers the same kind of question about a
    # different subject: the gate decides what Cybou may do to its host, and the capsule decides what
    # an agent may do inside one. Neither may be read from above, and neither may reach anything that
    # could carry out what it permits.
    ("governance", {"cybou-remediation", "cybou-capsule"}),
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

#: Crates that carry out what a governance crate permits, and which it may therefore never name.
#:
#: The same split as `cybou-actiond` and `cybou-executord`, one layer down. `cybou-capsule` decides
#: what an agent may reach; `cybou-egressd` is the thing that connects it. A dependency from the
#: decider to the doer is how "deciding" quietly becomes "arranging", and it is the kind of edge
#: somebody adds for a good reason — the broker wanting a type, the decider wanting to test against
#: a real connection — which is why it is checked here rather than remembered.
#:
#: The other direction is fine and is the point: the broker reads a grant.
ENFORCERS = {"cybou-egressd"}

#: What each layer owns, for the error message. An operator reading a failure should not have to
#: open the ADR to know what was crossed.
OWNS = {
    "telemetry": "what the Body is doing right now",
    "journal": "what happened",
    "epistemic": "what is known, and with what epistemic force",
    "associative": "what is related, and what is relevant now",
    "attention": "what gets bounded attention",
    "meaning": "how a selected context is interpreted or expressed",
    "governance": "what may be done, and by whose permission",
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
        here = layer_of(crate)
        if here is not None and OWNS.get(here[1]) == OWNS["governance"]:
            checked += 1
            for dependency in sorted(path_dependencies(manifest)):
                if dependency in ENFORCERS:
                    print(
                        f"error: {crate} decides what may be done and depends on {dependency}, "
                        f"which does it. A decider that can name its enforcer is one refactor "
                        f"away from being the thing that acts."
                    )
                    violations += 1

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
