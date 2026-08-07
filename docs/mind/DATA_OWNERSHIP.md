<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Data Ownership

## Current M4 owners

| Resource | Owner |
|---|---|
| canonical `journal.db` | `cybou-eventd` |
| `identity.json` | `cybou-identityd` |
| volatile identity login marker | `cybou-identityd` |
| bounded Workspace | `cybou-workspaced` |
| presentation aggregation | `cybou-presenced` |
| visual cache | QML Presence proxy |

Intentions, Predictor, and Self derive their state from Event1 plus their narrow operation logic.

## Locations

Persistent:

```text
$XDG_STATE_HOME/cybou
```

Runtime:

```text
$XDG_RUNTIME_DIR/cybou
```

The runtime identity marker prevents a daemon restart from being confused with a new logical login.

## Invariants

- Plasma does not own cognitive persistence.
- presenced does not open `journal.db`.
- only identityd writes `identity.json`;
- only workspaced owns live bounded attention;
- opening another UI surface does not create another Mind;
- process isolation does not introduce duplicate authoritative copies.
