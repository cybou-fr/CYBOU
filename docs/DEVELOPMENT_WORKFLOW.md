<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Development Workflow

```text
issue
→ ADR when architecture changes
→ failing or missing test
→ implementation
→ local build
→ Nix package build
→ pull request
→ green CI
→ merge
```

## Rules

- Do not merge architectural behavior directly into `main` without review.
- Do not describe a commit as fixing an invariant unless a focused test demonstrates it.
- Keep commits narrow enough to review.
- Separate cleanup, protocol changes, migrations, UI work, and documentation when practical.
- Update `CURRENT_STATE.md` when an implementation milestone changes.
