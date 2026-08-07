<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Testing Strategy

## Current automated layers

The current C++ Mind package runs these CTest suites during the Nix build:

```text
protocol
journal
identity
intentions
predictor
selfmodel
workspace
presence
presence-extended
```

Together they cover the implemented in-process protocol, Journal, domain components, Workspace,
and Presence projections.

The repository also has:

- Nix formatting validation;
- REUSE validation;
- static KDE package validation;
- static QML API/source validation for the Presence applet;
- Nix package builds;
- a VM smoke check exposed through `nix flake check`.

## Current Journal/protocol invariants

Current automated tests are expected to protect at least:

```text
reject self-causation
reject self-evidence
reject null/duplicate evidence
reject missing cause
reject missing evidence
reject weaker derived privacy
reject duplicate terminal outcome
preserve v1 Journal hashes during migration
retain a v1 migration backup
fail closed on malformed legacy evidence
detect mutation of v2 hashed fields
serialize concurrent writers
roll back failed appends
recover persisted domain state after reopening
```

## Pending runtime invariants

These are architectural requirements, not claims about current behavior:

```text
one Presence backend per user session
Workspace receives every accepted contribution live
stable Mind state is independent of plasmashell application identity
eventd is the only durable Journal writer
keep Mind alive after plasmashell restart
reconnect Presence after plasmashell restart
report organ/process capability deficits
recover/reconcile after isolated process failure
```

They require process/runtime or NixOS VM coverage when the corresponding architecture exists.

## CI split

### Fast job — every push / pull request

Current fast workflow builds:

```text
checks.x86_64-linux.formatting
checks.x86_64-linux.reuse
checks.x86_64-linux.package-metadata
packages.x86_64-linux.cybou-mind
packages.x86_64-linux.cybou-presence-applet
```

and then runs:

```bash
nix fmt
git diff --exit-code
```

A green ordinary push proves those gates only.

### Full job — tags

The full workflow is configured to run:

```bash
nix flake check --print-build-logs
nix build .#nixosConfigurations.cybou-vm.config.system.build.vm
```

Because it is tag-only, it is normally skipped on a regular push.

## Local validation

Fast package-level validation:

```bash
nix build --print-build-logs \
  .#checks.x86_64-linux.formatting \
  .#checks.x86_64-linux.reuse \
  .#checks.x86_64-linux.package-metadata \
  .#packages.x86_64-linux.cybou-mind \
  .#packages.x86_64-linux.cybou-presence-applet

nix fmt
git diff --exit-code
```

Additional desktop packages should also be built when they are changed:

```bash
nix build .#packages.x86_64-linux.cybou-tools
nix build .#packages.x86_64-linux.cybou-layout-templates
```

Full local validation:

```bash
nix flake check --print-build-logs
```

The full check includes the VM smoke derivation and therefore has heavier runtime requirements.

## Definition of Done

A change is complete only when:

- implementation claims have focused tests where practical;
- configure and compilation pass;
- relevant QtTest suites pass;
- affected Nix packages build;
- required CI gates are green;
- migration/rollback behavior is tested when persistent state changes;
- `CURRENT_STATE.md` and related documentation describe the same behavior as the code.

Target requirements must not be marked implemented merely because an ADR exists.
