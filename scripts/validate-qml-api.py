#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
"""Small fail-fast checks for QML mistakes that Plasma reports only at runtime."""

from __future__ import annotations

import re
import sys
from pathlib import Path


def line_number(text: str, offset: int) -> int:
    return text.count("\n", 0, offset) + 1


def main(argv: list[str]) -> int:
    root = Path(argv[1] if len(argv) > 1 else
                "packages/cybou-presence-applet/org.cybou.presence")
    if not root.is_dir():
        print(f"validate-qml-api: {root} is not a directory", file=sys.stderr)
        return 1

    errors: list[str] = []
    qml_files = sorted(root.rglob("*.qml"))

    forbidden = [
        (
            re.compile(r"(?m)^\s*toolTip\s*:"),
            "ToolButton has no toolTip property; use ToolTip.text/visible/delay",
        ),
        (
            re.compile(r"(?m)^\s*Icon\s*\{"),
            "Icon is not a Qt Quick Controls type; qualify Kirigami.Icon or PlasmaCore.Icon",
        ),
    ]

    for qml in qml_files:
        text = qml.read_text(encoding="utf-8")
        for pattern, message in forbidden:
            for match in pattern.finditer(text):
                errors.append(f"{qml}:{line_number(text, match.start())}: {message}")

    stat_card = root / "contents" / "ui" / "utils" / "StatCard.qml"
    if not stat_card.is_file():
        errors.append(f"{stat_card}: reusable StatCard component is missing")
    else:
        text = stat_card.read_text(encoding="utf-8")
        required = {
            "property string title": r"(?m)^\s*property\s+string\s+title\s*:",
            "property var value": r"(?m)^\s*property\s+var\s+value\s*:",
            "property string icon": r"(?m)^\s*property\s+string\s+icon\s*:",
            "qualified icon type": r"\b(?:Kirigami|PlasmaCore)\.Icon\s*\{",
        }
        for label, pattern in required.items():
            if re.search(pattern, text) is None:
                errors.append(f"{stat_card}: missing {label}")

    for message in errors:
        print(f"error: {message}", file=sys.stderr)

    print(f"validate-qml-api: {len(qml_files)} QML file(s), {len(errors)} error(s)")
    return 1 if errors else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
