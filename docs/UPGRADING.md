<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Upgrading

To update a local or remote Cybou installation on Debian 13:

```bash
git pull origin main
cargo build --workspace --release --locked
systemctl --user restart 'cybou-*'
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
