<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Data Ownership

One persistent resource has one authoritative owner.

| Resource | Owner |
|---|---|
| `journal.db` | eventd |
| identity state | identityd |
| transient workspace | workspaced |
| presentation snapshots | presenced |
| QML view state | Plasma applet |
| cache | creating component |

QML must not open cognitive databases. Shared libraries must not hide mutable cognitive state. Opening a panel must not increment identity sessions.
