<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Development Workflow

## Change flow

```text
problem or capability
→ confirm current/future boundary
→ ADR if ownership, protocol, persistence, privacy, or authorization changes
→ focused failing/missing test
→ implementation
→ local cargo test
→ workspace cargo test and CI checks
→ documentation/status update
→ review and green CI
```

## Before implementation

1. Read [Current State](CURRENT_STATE.md) and the relevant [Mind contract](mind/README.md).
2. Identify the owner of every state being changed.
3. State whether behavior is current milestone work or future design.
4. Add an ADR when a change crosses process ownership, Event1 semantics, persistence, lifecycle,
   privacy, replication, language, or action authority.
5. Define recovery and degraded behavior, not only the successful path.

## Local development

One script runs every check that has to pass, on a Linux host with the Rust toolchain and
`chromedriver` present:

```bash
bash scripts/gate.sh
```

It stops at the first failure and names the step. Do not assemble the same checks by hand in a
shell: the ad-hoc form of this lied twice in one afternoon, once because a `;` sat where an `&&`
belonged and once because `cmd | grep x | head -1` takes its exit status from `head` and reports
success whether or not grep matched. A check whose failure is invisible is not a check.

**The browser step is not optional and is not covered by the native one.**
`crates/living-canvas/src/components` is compiled only for `wasm32`, so `cargo check --workspace`
says nothing about it and a broken card passes a native run clean. That has cost this tree several
times.

From Windows, send the current working tree to the Debian builder:

```bash
bash scripts/vps-checks.sh fast
```

## Documentation rules

- `CURRENT_STATE.md` changes with implemented capability, never ahead of it.
- `ROADMAP.md` describes sequencing; it is not evidence of completion.
- Proposed ADR text must remain labelled as future until accepted and implemented.
- Update protocol/owner/failure documents in the same change as their code contracts.
- Run `scripts/validate-cognitive-docs.py` after changing canonical architecture documentation.
- Keep SPDX metadata and relative links valid.

## Review checklist

- Is durable state accepted before it becomes visible?
- Is there one explicit owner for every mutable projection?
- Are causes, evidence, privacy, provenance, freshness, and retention defined where relevant?
- Can restart/interruption duplicate an effect or invent success?
- Does degraded behavior name the missing capability?
- Can UI or a language model accidentally become authority?
- If external state changes, are authorization and observed outcome separate?

## Commit discipline

- Keep protocol, migration, UI, cleanup, and documentation changes separable when practical.
- Do not merge architectural behavior directly into `main` without review.
- Do not call an invariant fixed unless a focused test demonstrates it.
- Do not update generated artifacts by hand when a source-of-truth generator exists.
