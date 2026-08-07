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


def require(path: Path, text: str, label: str, pattern: str, errors: list[str]) -> None:
    if re.search(pattern, text, re.MULTILINE) is None:
        errors.append(f"{path}: missing {label}")


def main(argv: list[str]) -> int:
    root = Path(
        argv[1]
        if len(argv) > 1
        else "packages/cybou-presence-applet/org.cybou.presence"
    )
    if not root.is_dir():
        print(f"validate-qml-api: {root} is not a directory", file=sys.stderr)
        return 1

    errors = []
    qml_files = sorted(root.rglob("*.qml"))

    for path in qml_files:
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
            (
                r"The journal could not be opened",
                "the M4 QML proxy must not diagnose Journal ownership directly",
            ),
        ):
            for match in re.finditer(pattern, text):
                line = text.count("\n", 0, match.start()) + 1
                errors.append(f"{path}:{line}: {message}")

    required_files = (
        root / "contents/ui/MindDock.qml",
        root / "contents/ui/MindTabBar.qml",
        root / "contents/ui/MindHeader.qml",
        root / "contents/ui/MindUnavailable.qml",
        root / "contents/ui/utils/StatCard.qml",
        root / "contents/ui/utils/InfoCard.qml",
    )
    for path in required_files:
        if not path.is_file():
            errors.append(f"{path}: missing")

    main_qml = root / "contents/ui/main.qml"
    if main_qml.is_file():
        text = main_qml.read_text(encoding="utf-8")
        require(
            main_qml,
            text,
            "direct Plasma 6 preferredRepresentation",
            r"^\s*preferredRepresentation\s*:",
            errors,
        )
        if "PlasmaExtras.PlaceholderMessage" in text:
            errors.append(
                f"{main_qml}: obsolete applet-level unavailable overlay; "
                "use MindUnavailable inside MindDock"
            )

    tab_bar = root / "contents/ui/MindTabBar.qml"
    if tab_bar.is_file():
        text = tab_bar.read_text(encoding="utf-8")
        require(
            tab_bar,
            text,
            "icon-only navigation",
            r"display\s*:\s*AbstractButton\.IconOnly",
            errors,
        )
        require(
            tab_bar,
            text,
            "48px icon button contract",
            r"Layout\.preferredWidth\s*:\s*48",
            errors,
        )

    dock = root / "contents/ui/MindDock.qml"
    if dock.is_file():
        text = dock.read_text(encoding="utf-8")
        require(
            dock,
            text,
            "64px navigation rail",
            r"Layout\.preferredWidth\s*:\s*64",
            errors,
        )
        require(
            dock,
            text,
            "unavailable page inside the dock shell",
            r"\bMindUnavailable\s*\{",
            errors,
        )

    card = root / "contents/ui/utils/StatCard.qml"
    if card.is_file():
        text = card.read_text(encoding="utf-8")
        for label, pattern in {
            "title property": r"^\s*property\s+string\s+title\s*:",
            "value property": r"^\s*property\s+var\s+value\s*:",
            "icon property": r"^\s*property\s+string\s+icon\s*:",
            "qualified Icon": r"\bKirigami\.Icon\s*\{",
        }.items():
            require(card, text, label, pattern, errors)

    for error in errors:
        print(f"error: {error}", file=sys.stderr)

    print(
        f"validate-qml-api: {len(qml_files)} QML file(s), "
        f"{len(errors)} error(s)"
    )
    return 1 if errors else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
