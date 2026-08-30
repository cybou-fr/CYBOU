#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
"""Refuse a card that says something about itself where nobody can hear it.

A panel that cannot reach the gateway writes why into a signal and renders it. Two of them wrote and
never rendered: the Editor put twenty-eight messages there — a draft autosave that failed, a save
the host refused, a conflict re-read — and the System Monitor put "Failed to load telemetry" there,
and no view read either. Both panels drew empty, which is the one answer this system is built never
to give: a projection that could not be read reports unknown, never nothing.

The check is per file, and that is the whole of why it works. An earlier version collected every
name read anywhere in the crate and asked whether the written one was among them — but every card
has a signal called `status_msg`, so the name was always read somewhere and the check passed on the
two defects it was written for. A rule that cannot fail is not a rule, and this one is tested by
removing the fixes and watching it fail.

A file that writes into another card's state is a different thing and is listed below with its
reason, because the alternative is a check nobody can make green honestly.
"""

import pathlib
import re
import sys

# Signals whose whole purpose is to be shown.
SPOKEN = ("status_msg", "error_msg", "refusal")

# Whitespace between the parts, because rustfmt breaks a long call across lines and the first
# version of this check quietly missed the System Monitor for exactly that reason — the defect it
# was written to catch, written across three lines instead of one.
_SPOKEN_ALTERNATION = "|".join(SPOKEN)
WRITE = re.compile(
    rf"(?:(\w+)\s*\.\s*)?(\w*(?:{_SPOKEN_ALTERNATION}))\s*\.\s*set\(\s*Some\("
)
READ = re.compile(
    rf"(?:(\w+)\s*\.\s*)?(\w*(?:{_SPOKEN_ALTERNATION}))\s*\.\s*get(?:_untracked)?\(\)"
)

# What makes a rendered message reach somebody who is not looking at it. `role="alert"` is the
# assertive form and is deliberately allowed as an alternative rather than required: most of these
# answer something the person just did, and interrupting them to say a listing refreshed is worse
# than waiting for a pause.
LIVE_REGION = re.compile(r'aria-live=|role="status"|role="alert"')

# Files that legitimately write a message another card renders. The receiver is named, so this
# cannot quietly widen into "this file is exempt".
CROSS_CARD_WRITERS = {
    ("components/cards/editor.rs", "diff"): (
        "a conflict is explained by the Diff panel the Editor opens to show it, which renders its "
        "own status line"
    ),
    ("components/cards/file_manager.rs", "editor_state"): (
        "admission of a file into the Editor is reported by the Editor, which is where the person "
        "is about to be looking"
    ),
}


def spoken(name: str) -> bool:
    return name.endswith(SPOKEN)


def main() -> int:
    root = pathlib.Path(__file__).resolve().parent.parent
    components = root / "crates" / "living-canvas" / "src" / "components"
    if not components.is_dir():
        print(f"validate-card-signals: {components} is not a directory", file=sys.stderr)
        return 2

    problems: list[str] = []
    written_total = 0

    for path in sorted(components.rglob("*.rs")):
        text = path.read_text(encoding="utf-8")
        relative = path.relative_to(components.parent).as_posix()

        writes: dict[tuple[str, str], int] = {}
        for receiver, name in WRITE.findall(text):
            if not spoken(name):
                continue
            written_total += 1
            key = (receiver, name)
            writes[key] = writes.get(key, 0) + 1

        reads = {(receiver, name) for receiver, name in READ.findall(text) if spoken(name)}
        # A split signal writes through `set_x` and is read through `x`; treating the setter as its
        # own name would report every one of them as unheard.
        reads |= {(receiver, "set_" + name) for receiver, name in reads}

        for (receiver, name), count in sorted(writes.items()):
            if (receiver, name) in reads:
                continue
            if (relative, receiver) in CROSS_CARD_WRITERS:
                continue
            where = f"{receiver}.{name}" if receiver else name
            problems.append(
                f"error: {relative} writes `{where}` {count} time(s) and never reads it, so this "
                f"panel says it where nobody can hear it"
            )

        # Visible is half of it. A panel that has just refused a write, lost its connection or
        # finished a replace has changed, and a person not looking at it is told by the live region
        # or not at all. This crate carried none until 2026-08-30.
        if reads and not LIVE_REGION.search(text):
            problems.append(
                f"error: {relative} renders a message and has no live region, so a screen reader "
                f"is never told the panel said anything"
            )

    for problem in problems:
        print(problem, file=sys.stderr)

    if problems:
        print(
            f"validate-card-signals: {written_total} message(s) written, "
            f"{len(problems)} of them unheard"
        )
        return 1

    print(
        f"validate-card-signals: {written_total} message(s) written, every one of them is rendered"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
