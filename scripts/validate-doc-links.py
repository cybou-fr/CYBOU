#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
"""Every link in the documentation must point at something that exists.

Written the day `docs/history/` and eight superseded ADRs were removed, and written *because* of
that removal rather than after it. The rule the project now follows is that documentation carries
only what constrains the system today, which means files will be deleted again — and a rule that
makes deletion routine needs a check that makes deletion safe.

Without one, pruning is quietly discouraged. Every removal risks leaving a link that renders as an
ordinary reference and leads nowhere, and the cost of that is paid by whoever follows it later, so
the safe-feeling choice becomes leaving stale documents in place. That is the wrong incentive to
build into a repository that has just decided its documentation should be a snapshot.

Checked over relative links only. An external URL can go dead for reasons this repository has no
control over, and a check that failed the build on somebody else's outage would be the kind of noisy
gate people learn to bypass — which would take this check with it.

Anchors are checked for existence of the file, not of the heading. A heading rename is a smaller
loss than a missing file and the fragment syntax is loose enough that matching it produces false
positives; a check that guesses is worse than one that is narrow and exact.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

#: Where documentation lives, plus the root readme that points into it.
ROOTS = [Path("docs"), Path("README.md")]

#: A markdown link that is not a bare URL: `[text](target)`.
LINK = re.compile(r"\[[^\]]*\]\(([^)]+)\)")

#: Links this check deliberately does not follow.
#:
#: Anything with a scheme is somebody else's uptime. A bare fragment is a heading in the same file,
#: which is the loose case described above.
def skipped(target: str) -> bool:
    return (
        "://" in target
        or target.startswith("#")
        or target.startswith("mailto:")
    )


def markdown_files() -> list[Path]:
    """Every markdown file this check covers."""
    found: list[Path] = []
    for root in ROOTS:
        if root.is_file():
            found.append(root)
        elif root.is_dir():
            found.extend(sorted(root.rglob("*.md")))
    return found


def main() -> int:
    broken = 0
    checked = 0

    for document in markdown_files():
        text = document.read_text(encoding="utf-8")
        for match in LINK.finditer(text):
            target = match.group(1).strip()
            if skipped(target):
                continue
            # A fragment addresses a place inside the file, and the file is what must exist.
            path_part = target.split("#", 1)[0]
            if not path_part:
                continue
            checked += 1
            resolved = (document.parent / path_part).resolve()
            if not resolved.exists():
                line = text[: match.start()].count("\n") + 1
                print(f"error: {document}:{line} links to {target}, which does not exist")
                broken += 1

    if broken:
        print(f"validate-doc-links: {checked} relative link(s), {broken} broken")
        return 1

    print(f"validate-doc-links: {checked} relative link(s), all resolve")
    return 0


if __name__ == "__main__":
    sys.exit(main())
