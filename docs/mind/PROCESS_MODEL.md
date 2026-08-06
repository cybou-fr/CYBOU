<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Process Model

## Current

Mind organs are C++ objects in the Presence process.

## Target

```text
cybou-eventd
cybou-identityd
cybou-intentiond
cybou-predictord
cybou-selfd
cybou-workspaced
cybou-presenced
```

Each is a `systemd --user` service and separate `QCoreApplication`, except GUI-specific presentation code.

## Lifecycle

- explicit startup dependencies;
- restart after recoverable failure;
- reconstruction from owned state or Journal;
- capability-deficit reporting;
- Plasma restart does not restart Mind.

## Health

Available, Starting, Healthy, Degraded, Unavailable, and Recovering.
