#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
"""Static validation of Cybou KDE packages.

Catches the Gate B failures that would otherwise only surface on a running Plasma session.
Needs no Plasma, no Nix and no graphics: pure stdlib, runs on the Windows host or in CI.

Usage:
    python scripts/validate-packages.py packages/

Exit code 0 = clean, 1 = at least one error. Warnings never fail the run.

Checks (mirrors the `packages` block of spec/acceptance.yaml):
  1. metadata.json parses and has a KPlugin block      -> metadata_json_parses
  2. KPlugin.Id matches the directory name             -> kplugin_id_matches_directory
  3. Global Theme declares the LookAndFeel structure   -> lookandfeel_kpackagestructure
  4. layout script uses the fixed upstream name        -> layout_script_upstream_name
  5. no symlink anywhere inside a package              -> no_symlink_in_theme_package
  6. every SVG parses as XML                           -> svg_xml_parses
  7. License field is a decided SPDX id, not TBD       -> licenses_recorded

Check 4 exists because Plasma fails silently here: a layout script named after the package ID is
simply never executed. No panel, no error message. See docs/adr and docs/05-plasma-packaging.md.
"""

from __future__ import annotations

import json
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

# Fixed by Plasma. Not derived from the package ID. Verified against
# plasma-workspace/lookandfeel/org.kde.breezedark on 2026-08-04.
LAYOUT_SCRIPT_NAME = "org.kde.plasma.desktop-layout.js"

LOOKANDFEEL_STRUCTURE = "Plasma/LookAndFeel"

# Decided in docs/adr/0007-licensing.md.
ALLOWED_LICENSES = {"MIT", "CC-BY-SA-4.0"}

errors: list[str] = []
warnings: list[str] = []


def error(where: Path | str, message: str) -> None:
    errors.append(f"{where}: {message}")


def warn(where: Path | str, message: str) -> None:
    warnings.append(f"{where}: {message}")


def check_symlinks(pkg: Path) -> None:
    """Plasma 6 KPackage dropped symlink support entirely."""
    for path in pkg.rglob("*"):
        if path.is_symlink():
            error(path, "symlink inside a Plasma 6 package (unsupported since Plasma 6)")


def check_svgs(pkg: Path) -> None:
    for svg in pkg.rglob("*.svg"):
        try:
            ET.parse(svg)
        except ET.ParseError as exc:
            error(svg, f"SVG is not well-formed XML: {exc}")


def check_layout_script(pkg: Path) -> None:
    layouts = pkg / "contents" / "layouts"
    if not layouts.is_dir():
        return
    scripts = sorted(p.name for p in layouts.glob("*.js"))
    if not scripts:
        warn(layouts, "layouts/ exists but contains no .js file")
        return
    if LAYOUT_SCRIPT_NAME not in scripts:
        error(
            layouts,
            f"no {LAYOUT_SCRIPT_NAME}; found {scripts}. "
            "Plasma ignores any other name and fails silently (no panel, no error).",
        )
    for name in scripts:
        if name != LAYOUT_SCRIPT_NAME:
            warn(layouts / name, "unused layout script: Plasma only executes the fixed name")


def check_metadata(pkg: Path) -> None:
    if (pkg / "manifest.json").is_file():
        error(
            pkg / "manifest.json",
            "manifest.json is the Plasma 5 name; KF6 KPackage reads metadata.json",
        )

    meta_path = pkg / "metadata.json"
    if not meta_path.is_file():
        error(pkg, "no metadata.json at package root")
        return

    try:
        meta = json.loads(meta_path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        error(meta_path, f"invalid JSON: {exc}")
        return

    plugin = meta.get("KPlugin")
    if not isinstance(plugin, dict):
        error(meta_path, "missing KPlugin object")
        return

    pkg_id = plugin.get("Id")
    if not pkg_id:
        error(meta_path, "KPlugin.Id is missing or empty")
    elif pkg_id != pkg.name:
        error(
            meta_path,
            f"KPlugin.Id {pkg_id!r} does not match directory name {pkg.name!r}; "
            "kpackagetool6 will not find the package",
        )

    structure = meta.get("KPackageStructure")
    is_lookandfeel = (pkg / "contents" / "layouts").is_dir() or (
        pkg / "contents" / "defaults"
    ).is_file()
    if is_lookandfeel and structure != LOOKANDFEEL_STRUCTURE:
        error(
            meta_path,
            f'looks like a Global Theme but KPackageStructure is {structure!r}; '
            f'expected "{LOOKANDFEEL_STRUCTURE}" or it will not appear in System Settings',
        )

    licence = plugin.get("License")
    if not licence or licence == "TBD":
        error(meta_path, "KPlugin.License is unset or still TBD (see docs/adr/0007-licensing.md)")
    elif licence not in ALLOWED_LICENSES:
        warn(meta_path, f"License {licence!r} is not one of {sorted(ALLOWED_LICENSES)}")

    for field in ("Name", "Version"):
        if not plugin.get(field):
            warn(meta_path, f"KPlugin.{field} is missing")

    if plugin.get("Website") == "":
        warn(meta_path, "empty Website field: omit the key instead of shipping an empty string")


def find_packages(root: Path) -> list[Path]:
    """A package is any directory containing metadata.json or manifest.json."""
    found = {
        p.parent
        for name in ("metadata.json", "manifest.json")
        for p in root.rglob(name)
    }
    return sorted(found)


def main(argv: list[str]) -> int:
    root = Path(argv[1] if len(argv) > 1 else "packages")
    if not root.exists():
        print(f"validate-packages: {root} does not exist", file=sys.stderr)
        return 1

    packages = find_packages(root)
    if not packages:
        # Not an error. This runs against a built output tree, and before Phase 3 there are
        # no KDE packages in it yet. Failing here would only teach people to ignore the check.
        print(f"validate-packages: no KDE packages under {root} (nothing to validate)")
        return 0

    for pkg in packages:
        check_metadata(pkg)
        check_layout_script(pkg)
        check_symlinks(pkg)
        check_svgs(pkg)

    for line in warnings:
        print(f"warning: {line}")
    for line in errors:
        print(f"error: {line}", file=sys.stderr)

    print(
        f"\nvalidate-packages: {len(packages)} package(s), "
        f"{len(errors)} error(s), {len(warnings)} warning(s)"
    )
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
