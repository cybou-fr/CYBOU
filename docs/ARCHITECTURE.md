<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Cybou Architecture

## Current M4 topology

```text
                          ┌───────────────────────┐
                          │      Plasma/QML       │
                          │ Presence proxy only   │
                          └───────────┬───────────┘
                                      │ Presence1
                                      ▼
                             ┌─────────────────┐
                             │ cybou-presenced │
                             └───────┬─────────┘
                                     │
          ┌──────────────┬───────────┼───────────┬──────────────┐
          ▼              ▼           ▼           ▼              ▼
   identityd       intentiond   predictord      selfd      workspaced
          │              │           │           │              │
          └──────────────┴───────────┴───────────┴──────┬───────┘
                                                        │ Event1
                                                        ▼
                                                 cybou-eventd
                                                        │
                                                        ▼
                                                   Journal v2
```

Each daemon is a separate executable, D-Bus name, and `systemd --user` service.

## Shared libraries

Shared libraries contain protocol/domain code and IPC utilities. They do not create a hidden
second Mind behind the QML surface.

The QML Presence library links only fabric/runtime client code; it no longer owns domain organ
objects.

## Failure domains

- stopping `predictord` does not terminate eventd, identityd, intentiond, workspaced, or presenced;
- restarting presenced does not start a new identity session;
- restarting identityd in the same login resumes the current identity through its runtime marker;
- restarting workspaced reconstructs bounded attention from Event1 history.

Full capability-deficit policy is M6, but M4 creates the isolation required for it.

## Presentation ordering

Presence listens to Workspace1 `Changed`, not directly to raw Event1 `Accepted`:

```text
durable COMMIT
→ accepted event
→ Workspace admission
→ presentation notification
```

## Next

M5 strengthens continuity/recovery. M6 turns process health into explicit capability deficits.
