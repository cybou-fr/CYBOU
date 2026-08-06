<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Release Process

## Versioning

Use semantic pre-release versions until the first stable release:

```text
v0.1.0-alpha.1
v0.1.0-alpha.2
```

## Release gates

- fast and full CI are green;
- all packages build from the tagged commit;
- VM and ISO smoke tests pass;
- checksums are generated;
- known limitations are documented;
- CHANGELOG is updated;
- persistent-state compatibility is stated;
- rollback instructions exist.

## Artifacts

A release may include:

```text
Cybou ISO
NixOS VM image
Hyper-V development image
SHA256SUMS
release notes
source archive
```

Never label a development image stable when Mind-state migrations or installation paths are unverified.
