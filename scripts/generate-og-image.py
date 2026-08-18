#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT
"""Render the social preview from the same geometry the SVG mark declares.

docs/03-design-system.md, restated in assets/cybou-mark.svg:
  viewBox 0 0 64 64 · centre 32,32 · outer radius 22 · stroke 6 · round caps
  opening 42 degrees centred toward the upper-right, gap edges at 24 and 66
  degrees, so the drawn stroke spans the remaining 318 degrees.

Screen coordinates put y downward, and PIL measures arc angles clockwise from
three o'clock, so the SVG's gap edges are -24 and -66 here: the stroke starts at
-24 and sweeps 318 degrees clockwise back to -66.
"""
import math
import pathlib
import sys

from PIL import Image, ImageDraw, ImageFilter, ImageFont

OUT = pathlib.Path(__file__).resolve().parent.parent / "www" / "assets" / "cybou-og.png"

# The site declares system typography, so the preview borrows a system grotesque
# rather than vendoring a font file. Regular and bold, first pair that exists.
FONT_CANDIDATES = [
    ("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
     "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"),
    ("/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
     "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf"),
    ("C:/Windows/Fonts/segoeui.ttf", "C:/Windows/Fonts/segoeuib.ttf"),
    ("C:/Windows/Fonts/arial.ttf", "C:/Windows/Fonts/arialbd.ttf"),
]


def font_pair():
    for regular, bold in FONT_CANDIDATES:
        if pathlib.Path(regular).exists() and pathlib.Path(bold).exists():
            return regular, bold
    sys.exit("no usable system font found; install dejavu or liberation fonts")


REGULAR, BOLD = font_pair()

W, H = 1200, 630
BG = (10, 13, 18)          # --bg-0 / theme-color #0A0D12
ARC = (242, 245, 248)      # the SVG stroke #F2F5F8
ACCENT = (112, 225, 200)   # #70E1C8
INK2 = (150, 165, 178)
SS = 4                     # supersample factor, resolved away at the end

base = Image.new("RGB", (W * SS, H * SS), BG)

# --- atmospheric field, echoing the page's two glows -------------------------
field = Image.new("RGB", base.size, BG)
fd = ImageDraw.Draw(field)
fd.ellipse([-260 * SS, -300 * SS, 640 * SS, 420 * SS], fill=(15, 30, 36))
fd.ellipse([720 * SS, 320 * SS, 1520 * SS, 940 * SS], fill=(13, 22, 32))
field = field.filter(ImageFilter.GaussianBlur(90 * SS))
base = Image.blend(base, field, 0.5)

CX, CY, R = 300 * SS, 315 * SS, 150 * SS
STROKE = R * 6 / 22          # stroke-width 6 against radius 22

# --- centre glow: a real radial falloff, not a disc with an edge --------------
glow = Image.new("L", base.size, 0)
gd = ImageDraw.Draw(glow)
gr = R * 6.7 / 22
gd.ellipse([CX - gr, CY - gr, CX + gr, CY + gr], fill=255)
glow = glow.filter(ImageFilter.GaussianBlur(gr * 1.2))
base = Image.composite(Image.new("RGB", base.size, ACCENT), base,
                       glow.point(lambda v: int(v * 0.18)))

d = ImageDraw.Draw(base)

# --- the aperture arc --------------------------------------------------------
box = [CX - R, CY - R, CX + R, CY + R]
d.arc(box, start=-24, end=-24 + 318, fill=ARC, width=int(STROKE))
# PIL grows an arc's width inward from the bounding box, so the stroke centreline
# sits at R - STROKE/2; the caps must be placed there or they step off the curve.
CAP_R = R - STROKE / 2
for ang in (-24, -66):       # round caps at both gap edges
    ax = CX + CAP_R * math.cos(math.radians(ang))
    ay = CY + CAP_R * math.sin(math.radians(ang))
    d.ellipse([ax - STROKE / 2, ay - STROKE / 2, ax + STROKE / 2, ay + STROKE / 2], fill=ARC)

pr = R * 4.5 / 22
d.ellipse([CX - pr, CY - pr, CX + pr, CY + pr], fill=ACCENT)

# --- wordmark and one line of positioning ------------------------------------
def font(size, bold=False):
    return ImageFont.truetype(BOLD if bold else REGULAR, int(size * SS))

X = 520 * SS
d.text((X, 214 * SS), "Cybou", font=font(92, True), fill=ARC)
d.text((X, 336 * SS), "An agent-native", font=font(36), fill=INK2)
d.text((X, 384 * SS), "operating environment", font=font(36), fill=INK2)
d.rectangle([X, 452 * SS, X + 110 * SS, 452 * SS + 3 * SS], fill=ACCENT)

base.resize((W, H), Image.LANCZOS).save(OUT, optimize=True)
print(f"wrote {OUT}")
