#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT

from __future__ import annotations

import re
import sys
from pathlib import Path

OBJECT_START = re.compile(
    r"^\s*(?:[A-Za-z_][A-Za-z0-9_]*\s*:\s*)?"
    r"([A-Za-z_][A-Za-z0-9_.]*)\s*\{"
)
PROPERTY = re.compile(r"^\s*([A-Za-z_][A-Za-z0-9_.]*)\s*:")


def strip_comment(line: str) -> str:
    quote = None
    escaped = False
    for i, ch in enumerate(line):
        if escaped:
            escaped = False
            continue
        if ch == "\\" and quote is not None:
            escaped = True
            continue
        if ch in {'"', "'"}:
            quote = None if quote == ch else ch if quote is None else quote
            continue
        if ch == "/" and quote is None and i + 1 < len(line) and line[i + 1] == "/":
            return line[:i]
    return line


def braces(line: str) -> tuple[int, int]:
    quote = None
    escaped = False
    opens = closes = 0
    for ch in line:
        if escaped:
            escaped = False
            continue
        if ch == "\\" and quote is not None:
            escaped = True
            continue
        if ch in {'"', "'"}:
            quote = None if quote == ch else ch if quote is None else quote
            continue
        if quote is None:
            opens += ch == "{"
            closes += ch == "}"
    return opens, closes


def validate_layouts(path: Path, text: str) -> list[str]:
    errors = []
    stack: list[tuple[str, int]] = []
    depth = 0

    for number, raw in enumerate(text.splitlines(), 1):
        line = strip_comment(raw)
        obj = OBJECT_START.match(line)
        if obj:
            stack.append((obj.group(1), depth))

        prop = PROPERTY.match(line)
        if prop and stack:
            owner = stack[-1][0]
            name = prop.group(1)
            if owner == "GridLayout" and name == "spacing":
                errors.append(
                    f"{path}:{number}: GridLayout has no spacing property; "
                    "use rowSpacing and columnSpacing"
                )
            if owner in {"RowLayout", "ColumnLayout"} and name in {"rowSpacing", "columnSpacing"}:
                errors.append(
                    f"{path}:{number}: {owner} has no {name} property; use spacing"
                )

        opened, closed = braces(line)
        depth += opened - closed
        while stack and depth <= stack[-1][1]:
            stack.pop()

    return errors


def main(argv: list[str]) -> int:
    root = Path(argv[1] if len(argv) > 1 else
                "packages/cybou-presence-applet/org.cybou.presence")
    if not root.is_dir():
        print(f"validate-qml-api: {root} is not a directory", file=sys.stderr)
        return 1

    errors = []
    files = sorted(root.rglob("*.qml"))

    for path in files:
        text = path.read_text(encoding="utf-8")
        errors.extend(validate_layouts(path, text))
        for pattern, message in (
            (r"(?m)^\s*toolTip\s*:", "use ToolTip.text/visible/delay"),
            (r"(?m)^\s*Icon\s*\{", "qualify Kirigami.Icon or PlasmaCore.Icon"),
            (
                r"(?m)^\s*Plasmoid\.preferredRepresentation\s*:",
                "Plasma 6 PlasmoidItem owns preferredRepresentation directly; "
                "use preferredRepresentation and plasmoid.formFactor",
            ),
        ):
            for match in re.finditer(pattern, text):
                line = text.count("\n", 0, match.start()) + 1
                errors.append(f"{path}:{line}: {message}")

    card = root / "contents/ui/utils/StatCard.qml"
    if not card.is_file():
        errors.append(f"{card}: missing")
    else:
        text = card.read_text(encoding="utf-8")
        for label, pattern in {
            "title property": r"(?m)^\s*property\s+string\s+title\s*:",
            "value property": r"(?m)^\s*property\s+var\s+value\s*:",
            "icon property": r"(?m)^\s*property\s+string\s+icon\s*:",
            "qualified Icon": r"\b(?:Kirigami|PlasmaCore)\.Icon\s*\{",
        }.items():
            if re.search(pattern, text) is None:
                errors.append(f"{card}: missing {label}")

    for error in errors:
        print(f"error: {error}", file=sys.stderr)
    print(f"validate-qml-api: {len(files)} QML file(s), {len(errors)} error(s)")
    return 1 if errors else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
