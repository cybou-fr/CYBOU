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


def require(path: Path, label: str, needle: str) -> None:
    if not path.is_file():
        error(path, "missing")
        return

    text = path.read_text(encoding="utf-8")
    if needle not in text:
        error(path, f"missing {label}: {needle!r}")


def require_regex(path: Path, label: str, pattern: str) -> None:
    if not path.is_file():
        error(path, "missing")
        return

    text = path.read_text(encoding="utf-8")
    if re.search(pattern, text, re.MULTILINE | re.DOTALL) is None:
        error(path, f"missing {label}")


def check_relative_markdown_links(repo: Path, path: Path) -> None:
    if not path.is_file():
        return

    text = path.read_text(encoding="utf-8")

    for match in re.finditer(r"\[[^\]]+\]\(([^)]+)\)", text):
        raw_target = match.group(1).strip()

        if (
            not raw_target
            or raw_target.startswith(("http://", "https://", "mailto:", "#"))
        ):
            continue

        target_without_anchor = raw_target.split("#", 1)[0]
        if not target_without_anchor:
            continue

        resolved = (path.parent / target_without_anchor).resolve()

        try:
            resolved.relative_to(repo.resolve())
        except ValueError:
            error(path, f"relative link escapes repository: {raw_target}")
            continue

        if not resolved.exists():
            line = text.count("\n", 0, match.start()) + 1
            error(
                path,
                f"line {line}: broken relative Markdown link: {raw_target}",
            )


def main(argv: list[str]) -> int:
    repo = Path(argv[1] if len(argv) > 1 else ".").resolve()

    paths = {
        "root_readme": repo / "README.md",
        "docs_index": repo / "docs/README.md",
        "mind_model": repo / "docs/MIND_MODEL.md",
        "architecture": repo / "docs/ARCHITECTURE.md",
        "current": repo / "docs/CURRENT_STATE.md",
        "roadmap": repo / "docs/ROADMAP.md",
        "glossary": repo / "docs/GLOSSARY.md",
        "mind_index": repo / "docs/mind/README.md",
        "adr21": repo / "docs/adr/ADR-0021-language-models-are-optional-faculties.md",
        "adr22": repo / "docs/adr/ADR-0022-authorized-action-boundary.md",
        "adr24": repo / "docs/adr/ADR-0024-cognitive-lifecycle-and-consolidation.md",
        "adr25": repo / "docs/adr/ADR-0025-grounding-epistemics-and-cognitive-governance.md",
        "adr26": repo / "docs/adr/ADR-0026-lifecycle-owner-and-wire-contract.md",
        "lifecycle": repo / "docs/mind/LIFECYCLE.md",
        "epistemic": repo / "docs/mind/EPISTEMIC_GOVERNANCE.md",
        "security_index": repo / "docs/security/README.md",
        "next_steps": repo / "docs/NEXT_STEPS.md",
    }

    for path in paths.values():
        if not path.is_file():
            error(path, "missing")

    require(
        paths["root_readme"],
        "Mind Model entry point",
        "docs/MIND_MODEL.md",
    )
    require(
        paths["docs_index"],
        "Mind Model documentation entry",
        "[Mind Model](MIND_MODEL.md)",
    )
    require(
        paths["mind_index"],
        "conceptual model entry",
        "[Mind Model](../MIND_MODEL.md)",
    )
    require(
        paths["docs_index"],
        "lifecycle documentation entry",
        "[Cognitive Lifecycle and Consolidation](mind/LIFECYCLE.md)",
    )
    require(
        paths["docs_index"],
        "epistemic documentation entry",
        "[Grounding, Epistemics, and Cognitive Governance](mind/EPISTEMIC_GOVERNANCE.md)",
    )
    require(
        paths["docs_index"],
        "executable next-steps entry",
        "[Next Engineering Steps](NEXT_STEPS.md)",
    )

    for label in (
        "## Terminology and scope",
        "## Cognitive topology",
        "## Organ semantics",
        "## Core invariants",
        "### 1. Durable before visible",
        "### 2. Causality over loose memory",
        "### 3. Identity is not a process",
        "### 4. Attention is not biography",
        "### 5. Models are faculties",
        "### 6. External actions return as observations",
        "### 7. Consolidation derives; it does not rewrite",
        "### 8. Perception is not truth",
        "### 9. Forgetting and values are governed",
        "## Lifecycle and consolidation — M5 in progress",
        "## Future degraded cognition — M6",
        "## Future grounded and distributed cognition — M7",
        "## Future language faculty — M8",
        "## Future authorized agency — M9",
        "## Design test for future features",
    ):
        require(paths["mind_model"], label, label)

    require_regex(
        paths["mind_model"],
        "non-sentience terminology boundary",
        r"They are not claims that Cybou is\s+conscious,\s+sentient",
    )
    require(
        paths["mind_model"],
        "current-vs-future scope",
        "`CURRENT_STATE.md` remains authoritative for what is implemented today.",
    )
    require(
        paths["mind_model"],
        "action feedback loop",
        "External actions return as observations",
    )

    require(
        paths["architecture"],
        "cognitive topology section",
        "## Cognitive topology",
    )
    require(
        paths["architecture"],
        "future faculty boundary",
        "## Future faculty boundary",
    )
    require(
        paths["architecture"],
        "future action boundary",
        "## Future action boundary",
    )

    require_regex(
        paths["current"],
        "current/future boundary",
        r"tree does not yet contain the planned M8 language faculty\s+or M9 authorized executor",
    )
    require(
        paths["current"],
        "no current model owner",
        "There is currently no language-model process and no privileged action-executor process",
    )

    for milestone in ("M5", "M6", "M7", "M8", "M9"):
        require(
            paths["roadmap"],
            f"{milestone} milestone",
            f"## {milestone}",
        )

    require(
        paths["roadmap"],
        "capability progression",
        "## Capability progression",
    )
    require(
        paths["roadmap"],
        "agency ordering rationale",
        "agency is added after memory, ownership, continuity",
    )

    for forbidden_owner in (
        "identity authority",
        "biography owner",
        "canonical Journal writer",
        "intention owner",
        "authorization authority",
        "privileged executor",
    ):
        require(
            paths["adr21"],
            f"language-faculty non-owner: {forbidden_owner}",
            forbidden_owner,
        )

    require(
        paths["adr21"],
        "model replacement continuity rule",
        "Replacing, upgrading, disabling, or switching a model MUST NOT",
    )
    require(
        paths["adr21"],
        "typed protocol boundary",
        "Model output that should influence durable cognition enters the typed protocol",
    )

    require(
        paths["adr22"],
        "proposal not authorization",
        "### Proposal is not authorization",
    )
    require(
        paths["adr22"],
        "typed execution",
        "### Execution is typed",
    )
    require(
        paths["adr22"],
        "action feedback",
        "### Every attempted action returns to cognition",
    )
    require(
        paths["adr22"],
        "unknown outcome instead of invented success",
        "represent outcome as",
    )

    for label, needle in (
        ("lifecycle modes", "Awake"),
        ("coordinator non-owner", "The coordinator MUST NOT"),
        ("bounded consolidation input", "high-water mark"),
        ("interruptible consolidation", "ConsolidationInterrupted"),
    ):
        require(paths["adr24"], label, needle)

    for label, needle in (
        ("perception provenance", "Perception and provenance"),
        ("epistemic reconciliation", "Epistemic projection and reconciliation"),
        ("retention policy", "Retention and forgetting"),
        ("homeostasis", "Homeostasis and metacognition"),
        ("value constraints", "Executive attention and value constraints"),
    ):
        require(paths["adr25"], label, needle)

    for label, needle in (
        ("lifecycle owner", "`cybou-lifecycled`"),
        ("lifecycle state root", "$XDG_STATE_HOME/cybou/lifecycle"),
        ("single active run", "One active mutating run"),
        ("wire schema", "Wire schema version 1"),
    ):
        require(paths["adr26"], label, needle)

    for label, needle in (
        ("lifecycle modes", "## Modes"),
        ("run contract", "## Run contract"),
        ("owner-specific work", "## Owner-specific work"),
        ("M5 acceptance", "## M5 acceptance evidence"),
    ):
        require(paths["lifecycle"], label, needle)

    for label, needle in (
        ("epistemic statuses", "## Epistemic projection"),
        ("contradiction handling", "## Contradiction and reconciliation"),
        ("retention lifecycle", "## Retention and forgetting"),
        ("M7 vertical slice", "## M7 minimal vertical slice"),
    ):
        require(paths["epistemic"], label, needle)

    for label in (
        "## P0 — Restore a trustworthy green baseline",
        "## P1 — Freeze the M5 lifecycle contract",
        "## P2 — Prove continuity before consolidation",
        "## P3 — Implement the consolidation MVP",
        "## P4 — Make lifecycle visible without moving ownership into UI",
        "## P5 — Close M5 and publish evidence",
        "## P6.1 — Freeze the capability and health contract",
        "## Suggested PR decomposition",
    ):
        require(paths["next_steps"], label, label)

    require(
        paths["glossary"],
        "language faculty term",
        "**Language faculty**",
    )
    require(
        paths["glossary"],
        "authorized action boundary term",
        "**Authorized Action Boundary**",
    )
    for term in (
        "**Cognitive lifecycle**",
        "**Consolidation**",
        "**Provenance**",
        "**Epistemic projection**",
        "**Retention**",
        "**Homeostasis**",
        "**Metacognition**",
        "**Value constraint**",
    ):
        require(paths["glossary"], f"glossary term {term}", term)
    require(
        paths["glossary"],
        "terminology disclaimer",
        "They do not assert sentience or biological equivalence.",
    )

    link_documents = {repo / "README.md", *repo.glob("docs/**/*.md")}
    for path in link_documents:
        check_relative_markdown_links(repo, path)

    for line in errors:
        print(f"error: {line}", file=sys.stderr)

    print(
        f"validate-cognitive-docs: "
        f"{len(paths)} canonical document(s), {len(errors)} error(s)"
    )
    return 1 if errors else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
