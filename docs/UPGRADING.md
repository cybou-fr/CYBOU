<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Upgrading

Build and test a NixOS generation before switching:

```bash
sudo nixos-rebuild build --flake .
sudo nixos-rebuild test --flake .
sudo nixos-rebuild switch --flake .
```

## Current Journal migration

Journal v1 → v2 migration is implemented.

When a v1 on-disk Journal is opened by the current v2 implementation:

```text
checkpoint WAL
→ create journal.db.v1.bak with VACUUM INTO
→ BEGIN IMMEDIATE
→ add v2 schema fields and normalized evidence table
→ validate/migrate legacy evidence
→ reject duplicate legacy terminal Outcomes
→ preserve every v1 row hash
→ set PRAGMA user_version = 2
→ verify the mixed-version hash chain
→ COMMIT
```

The backup is retained for manual recovery and is not deleted automatically.

Migration fails closed for malformed legacy evidence, missing evidence targets, duplicate legacy
terminal Outcomes, a damaged legacy hash chain, partial versioned schemas, or a database schema
newer than the implementation supports.

Old Journal rows are never rehashed merely to make them look like v2 history.

## Current limitations

Journal schema migration being implemented does **not** mean all Mind upgrades are stable.

Still pending:

- migration from the current host-derived Mind data path to `$XDG_STATE_HOME/cybou`;
- one-owner `eventd` state transition;
- process-isolated organ lifecycle migration;
- architecture transition/reconciliation records;
- release-level continuity guarantees.

Back up persistent Cybou state before testing pre-release architecture transitions.

## Target architecture migration

Future process/state migrations should follow:

```text
backup
→ validate current state
→ migrate transactionally
→ verify Journal and identity
→ reconstruct active commitments
→ start the new ownership topology
→ verify continuity
→ record transition outcome
```

If verification fails, do not create replacement identity or silently claim continuity.
