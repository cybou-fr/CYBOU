<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Privacy Model

From most restrictive to least restrictive:

```text
Local
Node
Household
Public
```

**Local** remains on the originating device.

**Node** is available to trusted components on that node.

**Household** may be replicated to explicitly trusted household nodes.

**Public** may be displayed or exported subject to user policy.

A derived contribution is at least as restrictive as every cause and evidence item. Unknown values are treated as Local. Reject incorrectly weak declarations rather than silently exposing data.
