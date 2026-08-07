<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Data Ownership

## Current

| Resource | Current owner |
|---|---|
| canonical `journal.db` | `cybou-eventd` |
| identity JSON | in-process Identity component |
| transient Workspace | in-process Workspace component |
| presentation wrappers | `plasmashell` |
| QML view state | Plasma applet |

The default production Presence reaches Journal only through Event1.

The explicit temporary/local Presence constructor is a test/tool seam and does not represent the
installed QML topology.

## Persistent location

```text
$XDG_STATE_HOME/cybou
```

with the standard `~/.local/state/cybou` fallback on Unix.

Legacy migration still runs before the first Event1 activation so eventd never opens a newly
created canonical Journal ahead of the one-time M1 state move.

## Target after M4

| Resource | Target owner |
|---|---|
| `journal.db` | `cybou-eventd` |
| identity state | `cybou-identityd` |
| transient workspace | `cybou-workspaced` |
| presentation snapshots | `cybou-presenced` |
