<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0012: Organ Process Isolation and Lifecycle

## Status

Accepted

## Context

Before M4, Identity, Intentions, Predictor, SelfModel, Workspace, and Presence lived as one
Presence-owned object graph inside `plasmashell`. M3 isolated only eventd.

## Decision

True organs are separate executables managed as `systemd --user` `Type=dbus` services:

```text
cybou-eventd
cybou-identityd
cybou-intentiond
cybou-predictord
cybou-selfd
cybou-workspaced
cybou-presenced
```

Each owns one versioned D-Bus name.

Shared libraries contain protocol/domain logic and transport helpers, but QML does not instantiate
hidden mutable organ state. The QML Presence type is a proxy to presenced.

Identity uses a volatile runtime-session marker so restarting identityd inside one user login
resumes rather than increments the same logical session.

## Consequences

- Plasma can restart independently of Mind.
- individual organs can fail independently;
- IPC/lifecycle contracts are mandatory;
- M6 can express capability deficits using real process health;
- synchronous local RPC remains visible as a transport concern.

## Evidence

The M4 integration test launches seven separate processes on a private D-Bus session and verifies
routing, process identity, restart behavior, and one-organ failure isolation.

The VM smoke gate verifies the installed systemd user-service graph.

## Alternatives Considered

A permanent `cybou-mindd` monolith was rejected.

Keeping an in-process fallback behind the QML Presence proxy was rejected because it would make
ownership ambiguous again.
