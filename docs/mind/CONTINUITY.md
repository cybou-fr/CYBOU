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

## Lifecycle integration — implemented M5 boundary

Continuity is not only persistence of files. It includes the ability to explain a transition and
reconcile partial work:

```text
Awake
→ checkpoint/high-water mark
→ Consolidating | Maintenance | Suspended
→ completed terminal record or Recovering
→ verified projections
→ Awake | Degraded
```

Every consolidation/migration run has a stable identity. Restart reads accepted partial and
terminal events before repeating work. A missing terminal record is not interpreted as success.

See [Cognitive Lifecycle and Consolidation](LIFECYCLE.md) and
[ADR-0024](../adr/ADR-0024-cognitive-lifecycle-and-consolidation.md).
