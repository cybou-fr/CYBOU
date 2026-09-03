#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
"""Refuse a card whose state outlives the person closing it.

`ToolCardStates` holds one map per kind of card, and `forget` is what a close runs. On 2026-09-03
the terminal map was not in it: the PTY subsystem added `terminals` beside the older `shells`, and
`forget` kept clearing only what it had always cleared. A closed terminal came back still saying
`Connected`, and its signals stayed alive under an owner that had been disposed — which the browser
gate caught, and which is only half the cost. The other half is quieter: twenty of those maps hold
what a person was looking at, including their mail, their notes and their contacts.

The rule is the one thing that makes this class of defect impossible to reintroduce silently: every
`StoredValue<HashMap<CardId, _>>` field of `ToolCardStates` must be released by `forget`. Adding a
card kind and forgetting this method now fails here rather than in whatever the next person happens
to notice.
"""

import pathlib
import re
import sys

SOURCE = pathlib.Path("crates/living-canvas/src/tool_state.rs")

FIELD = re.compile(r"^    (\w+): StoredValue<HashMap<CardId,", re.MULTILINE)
RELEASED = re.compile(r"self\.(\w+)\.update_value")


def main() -> int:
    if not SOURCE.exists():
        print(f"{SOURCE} is missing; run this from the repository root", file=sys.stderr)
        return 2
    text = SOURCE.read_text(encoding="utf-8")

    try:
        struct = text[
            text.index("pub struct ToolCardStates {") : text.index("impl ToolCardStates {")
        ]
        start = text.index("    pub fn forget(&self, card: CardId) {")
    except ValueError:
        print("ToolCardStates or its forget method could not be found", file=sys.stderr)
        return 2
    forget = text[start : text.index("\n    }", start)]

    declared = FIELD.findall(struct)
    released = set(RELEASED.findall(forget))
    missed = [field for field in declared if field not in released]

    if missed:
        print("Card state that a close does not release:", file=sys.stderr)
        for field in missed:
            print(f"  - ToolCardStates::{field}", file=sys.stderr)
        print(
            "\nAdd each one to ToolCardStates::forget. A card whose state survives being closed\n"
            "comes back saying what it used to say, and holds what the person was looking at.",
            file=sys.stderr,
        )
        return 1

    print(f"card release ok: {len(declared)} card state maps, all released on close")
    return 0


if __name__ == "__main__":
    sys.exit(main())
