<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Identity Continuity

Continuity includes identity, verified biography, active commitments, architecture transitions, migration outcome, and self-model continuity.

## Session

A new session gets a new session identifier while preserving subject identity.

## Upgrade

```text
validate → backup → migrate → verify Journal → restore intentions
→ start architecture → record transition
```

If verification or reconstruction fails, Cybou reports degraded continuity instead of silently claiming success.
