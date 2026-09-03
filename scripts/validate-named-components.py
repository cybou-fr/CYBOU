#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
"""Refuse a document that names a component this repository does not have.

`cybou-shelld` was removed on 2026-08-27 and kept being described for a week: the README offered it
as a desktop card, ARCHITECTURE gave it a security zone, TESTING said a gate verified its builtins,
and WEB_UI_ARCHITECTURE drew it in a diagram. Nothing failed, because prose has no compiler.

For a repository whose whole argument is that a claim must be backed by something that can be
checked, a public page describing a daemon that does not exist is the same defect it refuses
everywhere else. So every `cybou-<name>` a document names has to be a crate in this workspace or a
unit in `systemd/`, and a name that is neither fails here.

`CURRENT_STATE.md` is exempt, and deliberately: it is where removals are recorded, so it is the one
document that has to be able to say a thing is gone.
"""

import pathlib
import re
import sys

DOCUMENTS = ["README.md", *sorted(str(p) for p in pathlib.Path("docs").glob("*.md"))]
EXEMPT = {"docs/CURRENT_STATE.md", "docs/NEXT_STEPS.md"}

# `cybou-something`, optionally a systemd template instance, as prose or code span.
NAMED = re.compile(r"\bcybou-[a-z0-9-]+\b")

# Names that are not components: repository, workspace and file naming.
NOT_COMPONENTS = {
    # Runtime and build directories, which are paths rather than daemons.
    "cybou-target",
    "cybou-src",
    "cybou-agent-leases",
    "cybou-host-files",
    "cybou-personal",
    "cybou-agent",
    # Units and helper scripts that are components but not crates.
    "cybou-mind",
    "cybou-desktop-session",
    "cybou-action-policy",
    # Not components at all: an asset file, the GitHub organisation, the Unix group that grants
    # access, and an ordinary hyphenated phrase.
    "cybou-aperture",
    "cybou-fr",
    "cybou-access",
    "cybou-owned",
}


def known_components() -> set[str]:
    known = {path.name for path in pathlib.Path("crates").iterdir() if path.is_dir()}
    for unit in pathlib.Path("systemd").rglob("*.service"):
        known.add(unit.name.removesuffix(".service").removesuffix("@"))
    for unit in pathlib.Path("systemd").rglob("*.target"):
        known.add(unit.name.removesuffix(".target"))
    for runner in pathlib.Path("scripts").glob("cybou-*"):
        known.add(runner.name.split(".")[0].removesuffix("-runner"))
    return known


def main() -> int:
    if not pathlib.Path("crates").is_dir():
        print("run this from the repository root", file=sys.stderr)
        return 2
    known = known_components() | NOT_COMPONENTS

    unknown: list[tuple[str, int, str]] = []
    for document in DOCUMENTS:
        if document.replace("\\", "/") in EXEMPT:
            continue
        path = pathlib.Path(document)
        if not path.exists():
            continue
        for number, line in enumerate(path.read_text(encoding="utf-8").split("\n"), start=1):
            for name in NAMED.findall(line):
                stripped = name.removesuffix("-runner")
                if stripped not in known and name not in known:
                    unknown.append((document, number, name))

    if unknown:
        print("Documents name components this repository does not have:", file=sys.stderr)
        for document, number, name in unknown:
            print(f"  - {document}:{number}: {name}", file=sys.stderr)
        print(
            "\nEither the component is gone and the document should say so, or the name is wrong.\n"
            "A page describing a daemon that does not exist is the defect this project refuses\n"
            "everywhere it can be checked.",
            file=sys.stderr,
        )
        return 1

    print(f"named components ok: {len(DOCUMENTS)} documents, every name has something behind it")
    return 0


if __name__ == "__main__":
    sys.exit(main())
