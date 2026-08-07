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
lastError
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

`lastError` is presentation diagnostics for connection/retry UX. It does not make QML the owner of
the failing organ or storage resource.

## Commands

```text
wake()
promise(description)
reflect()
fulfillIndex(index)
abandonIndex(index)
observe(subject, value)
predict(subject)
```

`wake()` is explicitly invokable so the unavailable-state UI can retry the Presence1 connection.

presenced routes cognitive operations to the organ that owns them; it does not construct domain
organ objects itself.

Creating another QML Presence object creates another proxy only.
