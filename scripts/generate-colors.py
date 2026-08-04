#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
"""Generate KDE colour schemes from spec/design-tokens.json.

The schemes are derived, never hand-edited: the tokens file is authoritative (AGENTS.md), and
a hand-maintained .colors file would drift from it silently. Run:

    generate-colors.py spec/design-tokens.json out/

KDE reads these from share/color-schemes/*.colors. Every Colors:* group must be complete -
a missing key falls back to Breeze and produces a half-themed application.
"""

import json
import pathlib
import sys


def rgb(h):
    h = h.lstrip("#")
    return ",".join(str(int(h[i : i + 2], 16)) for i in (0, 2, 4))


def mix(a, b, t):
    """Blend two hex colours; t=0 gives a, t=1 gives b."""
    a, b = a.lstrip("#"), b.lstrip("#")
    out = []
    for i in (0, 2, 4):
        ca, cb = int(a[i : i + 2], 16), int(b[i : i + 2], 16)
        out.append(round(ca + (cb - ca) * t))
    return "#%02x%02x%02x" % tuple(out)


def scheme(name, t, dark):
    """Build one .colors file body from a token dict."""
    # Accent is not a text colour (see the contrast rules in docs/03); accentStrong is.
    accent_text = t["accentStrong"]
    # A dim variant for alternating rows and disabled text.
    alt = mix(t["surface"], t["canvas"], 0.5 if dark else 0.35)
    dim = mix(t["textMuted"], t["surface"], 0.45)

    def group(bg, bg_alt, fg, fg_inactive):
        return {
            "BackgroundNormal": bg,
            "BackgroundAlternate": bg_alt,
            "DecorationFocus": t["accent"],
            "DecorationHover": t["accent"],
            "ForegroundNormal": fg,
            "ForegroundInactive": fg_inactive,
            "ForegroundActive": accent_text,
            "ForegroundLink": accent_text,
            "ForegroundVisited": mix(accent_text, t["textMuted"], 0.4),
            "ForegroundNegative": t["danger"],
            "ForegroundNeutral": t["warning"],
            "ForegroundPositive": t["success"],
        }

    groups = {
        "Colors:Window": group(t["surface"], alt, t["text"], t["textMuted"]),
        "Colors:View": group(t["canvas"], alt, t["text"], t["textMuted"]),
        "Colors:Button": group(t["surfaceElevated"], alt, t["text"], t["textMuted"]),
        "Colors:Selection": group(t["accentStrong"], t["accentStrong"], t["canvas"] if dark else "#ffffff", dim),
        "Colors:Tooltip": group(t["surfaceElevated"], t["surface"], t["text"], t["textMuted"]),
        "Colors:Complementary": group(t["canvasRaised"], t["canvas"], t["text"], t["textMuted"]),
        "Colors:Header": group(t["surface"], t["canvasRaised"], t["text"], t["textMuted"]),
        "Colors:Header][Inactive": group(t["canvasRaised"], t["canvas"], t["textMuted"], dim),
    }

    lines = [f"[General]", f"ColorScheme={name}", f"Name={name}", "shadeSortColumn=true", ""]
    for gname, keys in groups.items():
        lines.append(f"[{gname}]")
        for k, v in keys.items():
            lines.append(f"{k}={rgb(v)}")
        lines.append("")

    lines += [
        "[ColorEffects:Disabled]",
        "Color=" + rgb(dim),
        "ColorAmount=0",
        "ColorEffect=0",
        "ContrastAmount=0.65",
        "ContrastEffect=1",
        "IntensityAmount=0.1",
        "IntensityEffect=2",
        "",
        "[ColorEffects:Inactive]",
        "ChangeSelectionColor=true",
        "Color=" + rgb(t["border"]),
        "ColorAmount=0.025",
        "ColorEffect=2",
        "ContrastAmount=0.1",
        "ContrastEffect=0",
        "Enable=false",
        "IntensityAmount=0",
        "IntensityEffect=0",
        "",
        "[WM]",
        "activeBackground=" + rgb(t["surface"]),
        "activeForeground=" + rgb(t["text"]),
        "activeBlend=" + rgb(t["accent"]),
        "inactiveBackground=" + rgb(t["canvasRaised"]),
        "inactiveForeground=" + rgb(t["textMuted"]),
        "inactiveBlend=" + rgb(t["border"]),
        "",
        "[KDE]",
        "contrast=4",
        "",
    ]
    return "\n".join(lines)


def luminance(h):
    h = h.lstrip("#")
    out = []
    for i in (0, 2, 4):
        c = int(h[i : i + 2], 16) / 255
        out.append(c / 12.92 if c <= 0.03928 else ((c + 0.055) / 1.055) ** 2.4)
    return 0.2126 * out[0] + 0.7152 * out[1] + 0.0722 * out[2]


def contrast(a, b):
    la, lb = luminance(a), luminance(b)
    return (max(la, lb) + 0.05) / (min(la, lb) + 0.05)


def check_contrast(mode, t, thresholds):
    """Fail the build rather than ship an illegible scheme.

    Catching this here means a token edit that breaks legibility stops at `nix build`,
    instead of being discovered by a human squinting at a screenshot in Phase 7.
    """
    text, non_text = thresholds["text"], thresholds["nonText"]
    surfaces = [("surface", t["surface"]), ("canvas", t["canvas"]), ("surfaceElevated", t["surfaceElevated"])]
    problems = []

    for sname, s in surfaces:
        for fname, f, need in (
            ("text", t["text"], text),
            ("textMuted", t["textMuted"], text),
            ("accentStrong", t["accentStrong"], text),
            ("accent", t["accent"], non_text),
            ("border", t["border"], 1.0),  # separators only, no threshold
        ):
            r = contrast(f, s)
            if r < need:
                problems.append(f"  {mode}: {fname} on {sname} is {r:.2f}:1, needs {need}")

    return problems


def main(argv):
    tokens = json.loads(pathlib.Path(argv[1]).read_text(encoding="utf-8"))
    out = pathlib.Path(argv[2])
    out.mkdir(parents=True, exist_ok=True)

    thresholds = tokens.get("contrast", {}).get("thresholds", {"text": 4.5, "nonText": 3.0})
    problems = []
    for mode in ("dark", "light"):
        problems += check_contrast(mode, tokens["colors"][mode], thresholds)

    if problems:
        print("contrast check failed:", file=sys.stderr)
        print("\n".join(problems), file=sys.stderr)
        return 1

    for mode, name in (("dark", "CybouHorizonDark"), ("light", "CybouHorizonLight")):
        body = scheme(name, tokens["colors"][mode], dark=(mode == "dark"))
        (out / f"{name}.colors").write_text(body, encoding="utf-8")
        print(f"{name}.colors")

    print("contrast: all token pairs pass")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
