<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Architecture Decision Records (ADRs)

This directory contains Architecture Decision Records (ADRs) for the Cybou project. ADRs document important architectural decisions along with their context and consequences.

## Format

Each ADR follows this structure:

```markdown
# ADR-XXXX: Title

## Status
Accepted | Proposed | Rejected | Deprecated | Superseded

## Context
The problem being addressed and any background information.

## Decision
The chosen solution.

## Consequences
Positive and negative outcomes of the decision.

## Alternatives Considered
Other options that were considered and why they were rejected.

## Related
Links to other ADRs, documentation, or code.
```

## Current ADRs

| Number | Title | Status |
|--------|-------|--------|
| [ADR-0003](ADR-0003-ai-in-v0.1-none.md) | AI in v0.1 - None | Accepted |
| [ADR-0004](ADR-0004-ci-workflow.md) | CI Workflow | Accepted |
| [ADR-0005](ADR-0005-calamares-upstream-profile.md) | Calamares Installer - Upstream Profile | Accepted |
| [ADR-0006](ADR-0006-state-version-pinning.md) | State Version Pinning | Accepted |
| [ADR-0007](ADR-0007-reuse-3.x-compliance.md) | REUSE 3.x Compliance | Accepted |
| [ADR-0008](ADR-0008-mind-dock-with-tabs.md) | Mind Dock with Organ Tabs | Accepted |

## How to Add a New ADR

1. Create a new file: `ADR-XXXX-title-in-kebab-case.md`
2. Use the next available number (increment from highest existing)
3. Follow the format above
4. Add entry to the table above
5. Commit and push

## Resources

- [ADR GitHub Repository](https://github.com/joel-costigliola/adr-tools)
- [ADR Template](https://github.com/joel-costigliola/adr-tools/blob/master/adr-template.md)
- [MADR (Markdown ADR)](https://adr.github.io/madr/)
