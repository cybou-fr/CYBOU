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



# The installed daemons are declared once, in the package's own install check. Counting them there
# means the documentation is measured against what actually ships rather than against a number
# somebody remembered to update.
# Test suites are declared once, in the tests CMakeLists. Counting them there is the same discipline
# as counting daemons in the package: the documentation is measured against what the build actually
# registers rather than against a number somebody remembered to update.
def count_test_suites(repo):
    text = (repo / "mind/tests/CMakeLists.txt").read_text(encoding="utf-8")
    return len(re.findall(r"add_test\(", text))


def count_installed_daemons(repo):
    text = (repo / "packages/cybou-mind/default.nix").read_text(encoding="utf-8")
    body = text.split("for daemon in", 1)[1].split("; do", 1)[0]
    return len(re.findall("cybou-[a-z]+d", body))


NUMBER_WORDS = {
    9: "nine",
    10: "ten",
    11: "eleven",
    12: "twelve",
    13: "thirteen",
    26: "twenty-six",
    27: "twenty-seven",
    28: "twenty-eight",
    29: "twenty-nine",
    30: "thirty",
    31: "thirty-one",
    32: "thirty-two",
    33: "thirty-three",
    34: "thirty-four",
    35: "thirty-five",
    36: "thirty-six",
}


def number_word(value):
    if value not in NUMBER_WORDS:
        raise SystemExit(
            f"validate-cognitive-docs: no spelled-out form for {value} daemons; add one"
        )
    return NUMBER_WORDS[value]


def main(argv: list[str]) -> int:
    repo = Path(argv[1] if len(argv) > 1 else ".").resolve()

    paths = {
        "root_readme": repo / "README.md",
        "docs_index": repo / "docs/README.md",
        "checkpoint": repo / "docs/PROJECT_CHECKPOINT_2026-08-10.md",
        "mind_model": repo / "docs/MIND_MODEL.md",
        "architecture": repo / "docs/ARCHITECTURE.md",
        "current": repo / "docs/CURRENT_STATE.md",
        "installation": repo / "docs/INSTALLATION.md",
        "roadmap": repo / "docs/ROADMAP.md",
        "glossary": repo / "docs/GLOSSARY.md",
        "mind_index": repo / "docs/mind/README.md",
        "adr21": repo / "docs/adr/ADR-0021-language-models-are-optional-faculties.md",
        "adr22": repo / "docs/adr/ADR-0022-authorized-action-boundary.md",
        "adr24": repo / "docs/adr/ADR-0024-cognitive-lifecycle-and-consolidation.md",
        "adr25": repo / "docs/adr/ADR-0025-grounding-epistemics-and-cognitive-governance.md",
        "adr26": repo / "docs/adr/ADR-0026-lifecycle-owner-and-wire-contract.md",
        "lifecycle": repo / "docs/mind/LIFECYCLE.md",
        "health": repo / "docs/mind/HEALTH.md",
        "rpc_resilience": repo / "docs/mind/RPC_RESILIENCE.md",
        "homeostasis": repo / "docs/mind/HOMEOSTASIS.md",
        "organ_contracts": repo / "docs/mind/ORGAN_CONTRACTS.md",
        "process_model": repo / "docs/mind/PROCESS_MODEL.md",
        "failure_modes": repo / "docs/mind/FAILURE_MODES.md",
        "data_ownership": repo / "docs/mind/DATA_OWNERSHIP.md",
        "epistemic": repo / "docs/mind/EPISTEMIC_GOVERNANCE.md",
        "security_index": repo / "docs/security/README.md",
        "threat_model": repo / "docs/security/THREAT_MODEL.md",
        "next_steps": repo / "docs/NEXT_STEPS.md",
        "history": repo / "docs/history/M5-M6.md",
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
    require(
        paths["docs_index"],
        "project checkpoint entry",
        "[Project Checkpoint — 2026-08-10](PROJECT_CHECKPOINT_2026-08-10.md)",
    )

    for label in (
        "## Checkpoint identity",
        "## Executive assessment",
        "## Quantitative repository snapshot",
        "## Evidence and quality assessment",
        "## Architecture maturity matrix",
        "## Risk register",
        "## Recommended M7 sequence",
        "## Checkpoint acceptance criteria",
        "## Decision at this checkpoint",
    ):
        require(paths["checkpoint"], label, label)
    require(
        paths["checkpoint"],
        "analysed source revision",
        "8857d32038f10892e718f7095da6b3f4207d6687",
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
        "### 5. Models are faculties, and language is a boundary",
        "### 5a. Learning is layered, and learned state outranks nothing",
        "### 6. External actions return as observations",
        "### 7. Consolidation derives; it does not rewrite",
        "### 8. Perception is not truth",
        "### 9. Forgetting and values are governed",
        "## Lifecycle and consolidation — M5 evaluated",
        "## Degraded cognition — M6 implemented",
        "## Future grounded and distributed cognition — M7",
        "## Future language and meaning — M8",
        "## Future lifelong learning — M9",
        "## Future authorized agency — M10",
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
        r"tree does not yet contain the planned M8 meaning boundary,\s+the M9 learning runtime,\s+the M10 authorized executor,\s+the M11 agent, worker and model runtime,\s+or\s+the M12 security control plane",
    )
    require(
        paths["current"],
        "no current model owner",
        "There is currently no language-model process and no privileged action-executor process",
    )
    # Derived, not hardcoded.
    #
    # This used to require the literal "nine-process Mind package". That did not merely go stale
    # when the tenth and eleventh organs arrived - it actively held the drift in place, because
    # correcting the documentation would have failed the check. A validator that pins the wrong
    # answer is worse than no validator: it converts a documentation error into a build error for
    # whoever tries to fix it.
    daemon_count = count_installed_daemons(repo)
    require(
        paths["installation"],
        "current Mind process count",
        f"{number_word(daemon_count)}-process Mind package",
    )
    require(
        paths["current"],
        "current Mind owner count",
        f"{number_word(daemon_count)} Mind owners",
    )
    require(
        paths["current"],
        "current test suite count",
        f"{number_word(count_test_suites(repo))} CTest suites",
    )
    require(
        paths["installation"],
        "continuity maturity boundary",
        "M5 continuity",
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
    require(
        paths["roadmap"],
        "M6 completion boundary",
        "Complete through P6.6; P6.7 resilience hardening is also complete.",
    )
    require(
        paths["roadmap"],
        "M7 current boundary",
        "Next engineering milestone.",
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
        "Replacing, upgrading, disabling, or removing any language/model implementation MUST NOT",
    )
    require(
        paths["adr21"],
        "typed protocol boundary",
        "Model-derived durable contributions enter the typed protocol",
    )

    require(
        paths["adr21"],
        "no-model configuration is valid",
        "`NoModel` is therefore a valid configuration",
    )
    require(
        paths["adr21"],
        "core cognition is local",
        "core cognition → local only",
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
        ("health schema", "## Schema v2"),
        ("health states", "## State vocabularies"),
        ("health validation", "## Validation invariants"),
        ("health ownership", "## Ownership boundary"),
    ):
        require(paths["health"], label, needle)

    for label, needle in (
        ("RPC semantics", "## Operation semantics"),
        ("RPC outcomes", "## Typed outcomes"),
        ("RPC retry", "## Retry and backoff"),
        ("RPC circuit", "## Circuit breaker"),
    ):
        require(paths["rpc_resilience"], label, needle)

    for label, needle in (
        ("homeostasis boundary", "## Boundary"),
        ("homeostasis schema", "## Schema v2"),
        ("homeostasis projection", "## Health1 projection"),
        ("homeostasis evidence", "## Evidence"),
    ):
        require(paths["homeostasis"], label, needle)

    for label, needle in (
        ("event owner", "## eventd"),
        ("health owner", "## healthd"),
        ("lifecycle owner", "## lifecycled"),
        ("presentation owner", "## presenced"),
        ("QML non-owner", "## QML Presence proxy"),
    ):
        require(paths["organ_contracts"], label, needle)

    for label, needle in (
        ("nine-process topology", "cybou-healthd"),
        ("lifecycle process", "cybou-lifecycled"),
        ("bounded Presence budget", "one monotonic request budget"),
    ):
        require(paths["process_model"], label, needle)

    for label, needle in (
        ("current M6 boundary", "Current M1–M6"),
        ("required-owner failure", "required eventd unavailable"),
        ("bounded timeout behavior", "one shared deadline"),
    ):
        require(paths["failure_modes"], label, needle)

    for label, needle in (
        ("current ownership boundary", "## Current M1–M6 owners"),
        ("health ownership", "`cybou-healthd`"),
        ("lifecycle ownership", "`cybou-lifecycled`"),
    ):
        require(paths["data_ownership"], label, needle)

    for label, needle in (
        ("epistemic statuses", "## Epistemic projection"),
        ("contradiction handling", "## Contradiction and reconciliation"),
        ("retention lifecycle", "## Retention and forgetting"),
        ("M7 vertical slice", "## M7 minimal vertical slice"),
    ):
        require(paths["epistemic"], label, needle)

    # The completed packages now live in the history document; Next Steps holds only executable
    # work. Checking them there keeps the record from being quietly dropped in the move.
    for label in (
        "## P0 — Restore a trustworthy green baseline",
        "## P1 — Freeze the M5 lifecycle contract",
        "## P2 — Prove continuity before consolidation",
        "## P3 — Implement the consolidation MVP",
        "## P4 — Make lifecycle visible without moving ownership into UI",
        "## P5 — Close M5 and publish evidence",
        "## P6.1 — Freeze the capability and health contract",
        "## Historical P6 PR decomposition",
    ):
        require(paths["history"], label, label)

    require(
        paths["next_steps"],
        "link to the moved history",
        "[Historical Execution](history/M5-M6.md)",
    )

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

    stale_current_claims = {
        paths["mind_model"]: (
            "M6 in progress",
            "current M5 substrate",
            "M6 should turn component health",
        ),
        paths["architecture"]: (
            "capability-specific recovery belongs to M6",
            "Resource/capability-aware operation belongs to M6",
            "current M5 organ topology",
        ),
        paths["glossary"]: (
            "M6 will model missing abilities",
            "Lifecycle coordinator — future",
            "Degraded mode — future",
        ),
        paths["failure_modes"]: ("## Not yet M6",),
        paths["installation"]: (
            "M5 continuity/migration guarantees are not implemented yet",
            "seven Mind services",
        ),
        paths["health"]: ("Presence projection remains later work",),
        paths["roadmap"]: ("**Current engineering milestone.**",),
        paths["threat_model"]: (
            "lifecycle and action boundaries are proposed, not active enforcement paths",
            "process sandboxing",
        ),
    }
    for path, forbidden_phrases in stale_current_claims.items():
        text = path.read_text(encoding="utf-8")
        for phrase in forbidden_phrases:
            if phrase in text:
                error(path, f"stale implementation claim: {phrase!r}")

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
