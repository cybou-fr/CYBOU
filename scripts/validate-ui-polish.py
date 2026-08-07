#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT

from __future__ import annotations

import re
import sys
from pathlib import Path

errors: list[str] = []


def error(path: Path | str, message: str) -> None:
    errors.append(f"{path}: {message}")


def require(path: Path, label: str, pattern: str) -> None:
    if not path.is_file():
        error(path, "missing")
        return

    text = path.read_text(encoding="utf-8")
    if re.search(pattern, text, re.MULTILINE) is None:
        error(path, f"missing {label}")


def main(argv: list[str]) -> int:
    if len(argv) != 3:
        print(
            "usage: validate-ui-polish.py PRESENCE_PKG HANDLE_PKG",
            file=sys.stderr,
        )
        return 2

    presence = Path(argv[1])
    handle = Path(argv[2])

    ui = presence / "contents/ui"
    tabs = ui / "tabs"
    utils = ui / "utils"

    tab_bar = ui / "MindTabBar.qml"
    header = ui / "MindHeader.qml"
    unavailable = ui / "MindUnavailable.qml"
    stat_card = utils / "StatCard.qml"
    info_card = utils / "InfoCard.qml"
    thin_scroll = utils / "ThinScrollBar.qml"
    handle_qml = handle / "contents/ui/main.qml"

    require(
        tab_bar,
        "thin 2px active indicator",
        r"width\s*:\s*2\s*\n\s*height\s*:\s*26",
    )
    require(
        tab_bar,
        "keyboard navigation",
        r"Keys\.onPressed\s*:\s*function\s*\(event\)",
    )
    require(
        tab_bar,
        "tab focus support",
        r"activeFocusOnTab\s*:\s*true",
    )
    require(
        tab_bar,
        "focus ring",
        r"border\.width\s*:\s*button\.activeFocus\s*\?\s*1\s*:\s*0",
    )
    require(
        tab_bar,
        "subtle checked surface",
        r"button\.checked\s*\n\s*\?\s*0\.07",
    )

    require(
        header,
        "compact 58px header",
        r"implicitHeight\s*:\s*58",
    )
    require(
        header,
        "subtle runtime state fill",
        r"root\.awake\s*\?\s*0\.12\s*:\s*0\.08",
    )

    require(
        stat_card,
        "alternate surface",
        r"color\s*:\s*Kirigami\.Theme\.alternateBackgroundColor",
    )
    require(
        stat_card,
        "icons hidden by default",
        r"property\s+bool\s+showIcon\s*:\s*false",
    )
    require(
        stat_card,
        "accent strip",
        r"Layout\.preferredWidth\s*:\s*3",
    )

    require(
        info_card,
        "borderless card",
        r"border\.width\s*:\s*0",
    )
    require(
        info_card,
        "accent edge",
        r"width\s*:\s*root\.emphasized\s*\?\s*3\s*:\s*2",
    )

    require(
        thin_scroll,
        "6px thin scrollbar",
        r"width\s*:\s*6",
    )
    require(
        thin_scroll,
        "as-needed scrollbar",
        r"policy\s*:\s*ScrollBar\.AsNeeded",
    )
    require(
        thin_scroll,
        "scrollbar fade animation",
        r"Behavior\s+on\s+opacity",
    )

    for name in ("DashboardTab.qml", "IdentityTab.qml", "SelfTab.qml"):
        path = tabs / name
        require(path, "responsive Flickable", r"\bFlickable\s*\{")
        require(path, "thin vertical scrollbar", r"ThinScrollBar\s*\{\s*\}")

    for name in (
        "IntentionsTab.qml",
        "ActivityTab.qml",
        "PredictorTab.qml",
        "WorkspaceTab.qml",
    ):
        path = tabs / name
        require(path, "thin vertical scrollbar", r"ThinScrollBar\s*\{\s*\}")
        require(path, "keyboard focusable interactive rows", r"activeFocusOnTab\s*:\s*true")

    require(
        tabs / "DashboardTab.qml",
        "adaptive dashboard columns",
        r"columns\s*:\s*width\s*>=\s*280\s*\?\s*2\s*:\s*1",
    )

    require(
        unavailable,
        "keyboard-focusable retry",
        r"activeFocusOnTab\s*:\s*true",
    )
    require(
        unavailable,
        "soft unavailable surface",
        r"Kirigami\.Theme\.alternateBackgroundColor",
    )

    require(
        handle_qml,
        "accessible handle name",
        r"Accessible\.name\s*:",
    )
    require(
        handle_qml,
        "handle width animation",
        r"Behavior\s+on\s+width",
    )
    require(
        handle_qml,
        "handle height animation",
        r"Behavior\s+on\s+height",
    )
    require(
        handle_qml,
        "handle opacity animation",
        r"Behavior\s+on\s+opacity",
    )

    for line in errors:
        print(f"error: {line}", file=sys.stderr)

    print(f"validate-ui-polish: {len(errors)} error(s)")
    return 1 if errors else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
