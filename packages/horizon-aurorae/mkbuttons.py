#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT
"""Generate the Aurorae button SVGs from the design tokens.

docs/04-desktop-layout.md: simple line geometry; the close button turns danger coloured on
hover only; maximize and minimize stay neutral; nothing animates.

Each file holds four states side by side in 24x24 cells, and each state is a plain <path>
carrying the id Aurorae looks up. Not a <g> wrapper with a transform: Aurorae measures an
element's bounding box, and that indirection renders nothing while logging nothing.
"""
import json
import pathlib
import sys


def glyph(name, x):
    """Return path data for one glyph, offset into its cell."""
    if name == "close":
        return f"M{x + 8} 8 L{x + 16} 16 M{x + 16} 8 L{x + 8} 16"
    if name == "minimize":
        return f"M{x + 8} 15 H{x + 16}"
    if name == "maximize":
        return f"M{x + 8.5} 8.5 H{x + 15.5} V15.5 H{x + 8.5} Z"
    if name == "restore":
        return (
            f"M{x + 9.5} 11 H{x + 15.5} V16.5 H{x + 9.5} Z "
            f"M{x + 10.5} 11 V8.5 H{x + 16.5} V14"
        )
    raise KeyError(name)


NAMES = ["close", "minimize", "maximize", "restore"]
HEADER_TAG = "SPDX-License" + "-Identifier: CC-BY-SA-4.0"
HEADER_COPY = "SPDX-FileCopyright" + "Text: 2026 Stanislav Saveliev"


def svg(name, states):
    cells = []
    for i, (state, colour, opacity) in enumerate(states):
        suffix = "" if state == "normal" else "-" + state
        cells.append(
            '  <path id="%s%s" d="%s" fill="none" stroke="%s" stroke-width="1.6" '
            'stroke-linecap="round" stroke-linejoin="round" opacity="%s"/>'
            % (name, suffix, glyph(name, i * 24), colour, opacity)
        )
    return (
        "<!--\n"
        + HEADER_COPY
        + "\n"
        + HEADER_TAG
        + "\n\n"
        + "Aurorae button: %s. Four states, one 24x24 cell each.\n" % name
        + "-->\n"
        + '<svg xmlns="http://www.w3.org/2000/svg" width="96" height="24" viewBox="0 0 96 24">\n'
        + "\n".join(cells)
        + "\n</svg>\n"
    )


def main(argv):
    tokens = json.loads(pathlib.Path(argv[1]).read_text(encoding="utf-8"))["colors"]["dark"]
    out = pathlib.Path(argv[2])
    out.mkdir(parents=True, exist_ok=True)

    text, muted, danger = tokens["text"], tokens["textMuted"], tokens["danger"]

    for name in NAMES:
        hover = danger if name == "close" else text
        states = [
            ("normal", muted, "1"),
            ("hover", hover, "1"),
            ("pressed", hover, "0.75"),
            ("deactivated", muted, "0.35"),
        ]
        (out / (name + ".svg")).write_text(svg(name, states), encoding="utf-8")
        print(name + ".svg")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
