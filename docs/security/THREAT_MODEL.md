<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Threat Model

## Assets

Identity, Journal, intentions, private observations, Nix configuration, credentials, D-Bus interfaces, migration backups, and future inter-node messages.

## Threats

- Journal modification, insertion, deletion, or truncation;
- privacy weakening;
- false organ impersonation;
- replay of old messages;
- duplicate or invalid lifecycle outcomes;
- migration rewriting or partial completion;
- QML authority confusion and duplicate Presence instances;
- action escalation from uncertain cognition to privileged mutation.

## Controls

- single Journal writer;
- canonical full-envelope hashing;
- stable D-Bus policy;
- message uniqueness;
- explicit capabilities;
- privacy inheritance;
- transactional migration;
- process sandboxing;
- typed action boundary;
- security and concurrency tests.
