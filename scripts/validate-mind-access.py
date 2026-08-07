#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

errors: list[str] = []


def error(where: Path | str, message: str) -> None:
    errors.append(f"{where}: {message}")


def require_text(path: Path, text: str, label: str, pattern: str) -> None:
    if re.search(pattern, text, re.MULTILINE) is None:
        error(path, f"missing {label}")


def check_metadata(package: Path, expected_id: str) -> None:
    metadata = package / "metadata.json"
    if not metadata.is_file():
        error(package, "metadata.json is missing")
        return

    try:
        data = json.loads(metadata.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        error(metadata, f"invalid JSON: {exc}")
        return

    plugin = data.get("KPlugin", {})
    if plugin.get("Id") != expected_id:
        error(metadata, f"KPlugin.Id must be {expected_id!r}")

    if data.get("KPackageStructure") != "Plasma/Applet":
        error(metadata, "KPackageStructure must be Plasma/Applet")

    if data.get("X-Plasma-API-Minimum-Version") != "6.0":
        error(metadata, "X-Plasma-API-Minimum-Version must be 6.0")


def main(argv: list[str]) -> int:
    if len(argv) != 4:
        print(
            "usage: validate-mind-access.py PRESENCE_PKG HANDLE_PKG LAYOUT_JS",
            file=sys.stderr,
        )
        return 2

    presence = Path(argv[1])
    handle = Path(argv[2])
    layout = Path(argv[3])

    check_metadata(presence, "org.cybou.presence")
    check_metadata(handle, "org.cybou.mindhandle")

    handle_qml = handle / "contents/ui/main.qml"
    handle_cfg = handle / "contents/config/main.xml"

    if not handle_qml.is_file():
        error(handle_qml, "missing")
    else:
        text = handle_qml.read_text(encoding="utf-8")
        require_text(
            handle_qml,
            text,
            "DockAccess object",
            r"\bDockAccess\s*\{",
        )
        require_text(
            handle_qml,
            text,
            "hover reveal",
            r"onEntered\s*:\s*\{[^}]*dockAccess\.peek\(\)",
        )
        require_text(
            handle_qml,
            text,
            "click pin toggle",
            r"onClicked\s*:\s*\{[^}]*dockAccess\.togglePinned\(\)",
        )
        require_text(
            handle_qml,
            text,
            "global-shortcut activation handler",
            r"onActivated\s*\(\)\s*\{[^}]*dockAccess\.togglePinned\(\)",
        )
        require_text(
            handle_qml,
            text,
            "first-run onboarding",
            r"onboardingVisible",
        )
        require_text(
            handle_qml,
            text,
            "no applet popup toggle on activation",
            r"activationTogglesExpanded\s*:\s*false",
        )

    if not handle_cfg.is_file():
        error(handle_cfg, "missing")
    else:
        cfg = handle_cfg.read_text(encoding="utf-8")
        require_text(
            handle_cfg,
            cfg,
            "onboardingSeen persistent flag",
            r'<entry\s+name="onboardingSeen"\s+type="Bool">',
        )

    if not layout.is_file():
        error(layout, "missing")
    else:
        text = layout.read_text(encoding="utf-8")

        for label, pattern in (
            (
                "native auto-hide Mind dock",
                r'mindDock\.hiding\s*=\s*"autohide"',
            ),
            (
                "Presence widget in Mind dock",
                r'mindDock\.addWidget\("org\.cybou\.presence"\)',
            ),
            (
                "separate handle panel",
                r"var\s+mindHandle\s*=\s*new\s+Panel",
            ),
            (
                "18px handle thickness",
                r"mindHandle\.height\s*=\s*18",
            ),
            (
                "custom handle length mode",
                r'mindHandle\.lengthMode\s*=\s*"custom"',
            ),
            (
                "82px handle length",
                r"mindHandle\.length\s*=\s*82",
            ),
            (
                "persistent visible handle",
                r'mindHandle\.hiding\s*=\s*"none"',
            ),
            (
                "Mind handle widget",
                r'mindHandle\.addWidget\("org\.cybou\.mindhandle"\)',
            ),
            (
                "Meta+M shortcut",
                r'handleWidget\.globalShortcut\s*=\s*"Meta\+M"',
            ),
        ):
            require_text(layout, text, label, pattern)

    for line in errors:
        print(f"error: {line}", file=sys.stderr)

    print(f"validate-mind-access: {len(errors)} error(s)")
    return 1 if errors else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
