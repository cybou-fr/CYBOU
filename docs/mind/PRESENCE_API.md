<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Presence API

## Process boundary

The real backend is `cybou-presenced` on:

```text
org.cybou.Mind.Presence1
```

The C++ `Presence` type exported to QML is only a proxy/cache.

## QML properties

```text
awake
narration
obligations
attention
contributions
stats
identityState
calibrations
coalitions
moment
organHealth
```

## Commands

```text
promise(description)
reflect()
fulfillIndex(index)
abandonIndex(index)
observe(subject, value)
predict(subject)
```

presenced routes each operation to the organ that owns it; it does not construct domain organ
objects itself.

Creating another QML Presence object creates another proxy only.
