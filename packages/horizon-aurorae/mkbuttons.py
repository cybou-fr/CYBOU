#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT
"""Generate the Aurorae button SVGs from the design tokens.

docs/04-desktop-layout.md: buttons are simple line geometry; the close button turns danger
coloured on hover only; maximize and minimize stay neutral.

Each button SVG carries four element ids that Aurorae looks up by name - normal, hover,
pressed and deactivated - laid out side by side in one 96x24 canvas.
"""
import json
import pathlib
import sys

GLYPHS = {
    # 24x24 cell, stroke geometry centred on the cell
    "close": "M8 8 L16 16 M16 8 L8 16",
    "minimize": "M8 15 H16",
    "maximize": "M8.5 8.5 H15.5 V15.5 H8.5 Z",
    "restore": "M9.5 11 H15.5 V16.5 H9.5 Z M10.5 11 V8.5 H16.5 V14",
}


def svg(name, colours):
    cells = []
    for i, (state, colour, opacity) in enumerate(colours):
        suffix = "" if state == "normal" else f"-{state}"
        cells.append(
            f'  <g id="{name}{suffix}" transform="translate({i * 24},0)">\n'
            f'    <path d="{GLYPHS[name]}" fill="none" stroke="{colour}" stroke-width="1.4" '
            f'stroke-linecap="round" stroke-linejoin="round" opacity="{opacity}"/>\n'
            f"  </g>"
        )
    body = "\n".join(cells)
    return (
        "<!--\n"
        "SPDX-FileCopyrightText: 2026 Stanislav Saveliev\n"
        "SPDX-License-Identifier: CC-BY-SA-4.0\n\n"
        f"Aurorae button: {name}. Four states side by side, each in its own 24x24 cell, with the\n"
        "element ids Aurorae looks up. Generated from spec/design-tokens.json.\n"
        "-->\n"
        '<svg xmlns="http://www.w3.org/2000/svg" width="96" height="24" viewBox="0 0 96 24">\n'
        f"{body}\n"
        "</svg>\n"
    )


def main(argv):
    tokens = json.loads(pathlib.Path(argv[1]).read_text(encoding="utf-8"))["colors"]["dark"]
    out = pathlib.Path(argv[2])
    out.mkdir(parents=True, exist_ok=True)

    text, muted, danger = tokens["text"], tokens["textMuted"], tokens["danger"]

    for name in GLYPHS:
        # Only close reacts with colour; the rest brighten. Nothing animates.
        hover = danger if name == "close" else text
        states = [
            ("normal", muted, "1"),
            ("hover", hover, "1"),
            ("pressed", hover, "0.75"),
            ("deactivated", muted, "0.35"),
        ]
        (out / f"{name}.svg").write_text(svg(name, states), encoding="utf-8")
        print(f"{name}.svg")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
