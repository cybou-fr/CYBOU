#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
"""Refuse a colour a card invented for itself.

The stylesheet defines twenty tokens and says what each is for: `--ok` is something that is working,
`--danger` something that failed, `--info` something to know, and four steps of text from bright to
faint. Nineteen card files still painted twenty-nine literal colours between them, seventy-three
times — a green that was not `--ok`, three greys for the same "quieter than the text", and a purple
that was in no palette at all. Read together on one screen they are what "the colours are not ours"
looks like.

A literal here is not a small inconsistency. It is a colour a theme cannot change: the light theme
redefines the tokens and every literal ignores it, which is how a panel comes out unreadable on a
white ground. The same is true of any future theme, and of anybody's contrast requirements.

The check is deliberately narrow. It reads the card components — the surfaces a person looks at —
and says nothing about the stylesheet, where defining a colour is the point.
"""

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
CARDS = ROOT / "crates/living-canvas/src/components/cards"

# `#abc`, `#aabbcc`, `#aabbccdd`, and the `rgb()`/`rgba()` families spelled out in numbers.
LITERAL = re.compile(r"#[0-9a-fA-F]{3,8}\b|\brgba?\(\s*\d")

# The one place a number is the honest answer: a colour that came from somewhere else. The terminal
# renders what a program asked for, and an ANSI palette is not this desktop's palette.
ALLOWED_FILES = {"terminal.rs"}


def main() -> int:
    problems = []
    checked = 0

    for path in sorted(CARDS.glob("*.rs")):
        if path.name in ALLOWED_FILES:
            continue
        checked += 1
        for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
            match = LITERAL.search(line)
            if match:
                relative = path.relative_to(ROOT).as_posix()
                problems.append(
                    f"error: {relative}:{number} paints {match.group(0)!r} rather than a token, "
                    f"so no theme can change it"
                )

    for problem in problems[:40]:
        print(problem, file=sys.stderr)
    if len(problems) > 40:
        print(f"... and {len(problems) - 40} more", file=sys.stderr)

    if problems:
        print(f"validate-card-palette: {checked} card(s), {len(problems)} invented colour(s)")
        return 1

    print(f"validate-card-palette: {checked} card(s), every colour comes from the palette")
    return 0


if __name__ == "__main__":
    sys.exit(main())
