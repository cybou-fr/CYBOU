#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
"""Generate the Horizon Field wallpapers from spec/design-tokens.json.

Derived, not drawn: every number below comes from the tokens file or from
docs/03-design-system.md, so the wallpaper cannot quietly drift from the design system.

    generate-wallpaper.py spec/design-tokens.json out/

Composition (docs/03-design-system.md):
  1. base vertical gradient
  2. horizon at 61% of canvas height
  3. one 1 px mint-white line on the horizon, 18-25% opacity
  4. three very broad arcs emerging below the horizon, each under 10% opacity
  5. no text, stars, mountains, planets, mascots or AI symbolism

Safe zone: a 21:9 centre crop of the 16:9 canvas removes 257 px top and bottom. Every arc apex
must stay inside that band, or an ultrawide screen loses the composition.
"""

import json
import pathlib
import sys

W, H = 3840, 2160


def mix(a, b, t):
    a, b = a.lstrip("#"), b.lstrip("#")
    return "#%02x%02x%02x" % tuple(
        round(int(a[i : i + 2], 16) + (int(b[i : i + 2], 16) - int(a[i : i + 2], 16)) * t)
        for i in (0, 2, 4)
    )


def wallpaper(t, wp, dark):
    horizon = round(H * wp["horizonAtHeightRatio"])  # 0.61 -> 1318
    line_op = sum(wp["horizonLineOpacityRange"]) / 2  # midpoint of 0.18-0.25
    arc_max = wp["arcMaxOpacity"]

    if dark:
        top, bottom = mix(t["canvas"], "#000000", 0.35), mix(t["canvasRaised"], t["information"], 0.10)
        line = mix(t["accent"], "#ffffff", 0.55)
    else:
        top, bottom = mix(t["canvas"], "#ffffff", 0.45), mix(t["canvasRaised"], t["information"], 0.06)
        line = mix(t["accentStrong"], "#ffffff", 0.35)

    # Three broad arcs. Centres sit far below the canvas so only a shallow cap shows; the apex
    # of each is placed inside the 21:9 safe band (y between 257 and 1903).
    arcs = [
        # (apex y, radius, colour, opacity)
        (horizon + 90, 5200, t["information"], arc_max * 0.55),
        (horizon + 250, 3400, t["accent"], arc_max * 0.45),
        (horizon + 430, 2600, t["border"], arc_max * 0.9),
    ]

    parts = [
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width="{W}" height="{H}">',
        "  <defs>",
        '    <linearGradient id="sky" x1="0" y1="0" x2="0" y2="1">',
        f'      <stop offset="0" stop-color="{top}"/>',
        f'      <stop offset="1" stop-color="{bottom}"/>',
        "    </linearGradient>",
        '    <linearGradient id="edge" x1="0" y1="0" x2="1" y2="0">',
        f'      <stop offset="0" stop-color="{line}" stop-opacity="0"/>',
        f'      <stop offset="0.5" stop-color="{line}" stop-opacity="1"/>',
        f'      <stop offset="1" stop-color="{line}" stop-opacity="0"/>',
        "    </linearGradient>",
        "  </defs>",
        f'  <rect width="{W}" height="{H}" fill="url(#sky)"/>',
    ]

    for apex, r, colour, op in arcs:
        cy = apex + r
        parts.append(
            f'  <circle cx="{W // 2}" cy="{cy}" r="{r}" fill="{colour}" fill-opacity="{op:.3f}"/>'
        )

    parts += [
        f'  <rect x="0" y="{horizon}" width="{W}" height="1" fill="url(#edge)" '
        f'fill-opacity="{line_op:.2f}"/>',
        "</svg>",
    ]
    return "\n".join(parts) + "\n"


def metadata(name, mode):
    return (
        "[Desktop Entry]\n"
        f"Name=Cybou Horizon {mode}\n"
        "X-KDE-PluginInfo-Name=" + name + "\n"
        "X-KDE-PluginInfo-Author=Cybou contributors\n"
        "X-KDE-PluginInfo-License=CC-BY-SA-4.0\n"
        "X-KDE-PluginInfo-Version=0.1\n"
    )


def main(argv):
    tokens = json.loads(pathlib.Path(argv[1]).read_text(encoding="utf-8"))
    out = pathlib.Path(argv[2])
    wp = tokens["wallpaper"]
    horizon = round(H * wp["horizonAtHeightRatio"])

    # 21:9 centre crop of a 16:9 canvas: cropped height, and the band that survives.
    crop_h = round(W / (21 / 9))
    band = ((H - crop_h) // 2, (H + crop_h) // 2)
    assert band[0] < horizon < band[1], f"horizon {horizon} outside 21:9 safe band {band}"

    for mode, name in (("dark", "CybouHorizonDark"), ("light", "CybouHorizonLight")):
        d = out / name / "contents" / "images"
        d.mkdir(parents=True, exist_ok=True)
        (d / f"{W}x{H}.svg").write_text(
            wallpaper(tokens["colors"][mode], wp, dark=(mode == "dark")), encoding="utf-8"
        )
        (out / name / "metadata.desktop").write_text(
            metadata(name, mode.capitalize()), encoding="utf-8"
        )
        print(f"{name}: horizon at y={horizon} ({wp['horizonAtHeightRatio']:.0%}), "
              f"21:9 safe band {band[0]}-{band[1]}")

    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
