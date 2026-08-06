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

Until Mind migrations are stable:

- back up the Cybou state directory;
- never rewrite old Journal rows;
- reject unsupported newer schemas;
- do not create a new identity over damaged state.

Target migration:

```text
backup → validate → migrate transactionally → verify → restore intentions
→ verify continuity → start organs → record outcome
```
