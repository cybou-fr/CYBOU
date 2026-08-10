<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# M5 Evaluation Evidence

## Scope

This record describes the implemented M5 subset. It is evaluation evidence, not a stable-release
claim. M6 degraded cognition, M7 retention/epistemic governance, M8 language, and M9 authorized
action remain outside this candidate.

## Supported transition paths

| Path | Demonstrated behavior | Gate |
|---|---|---|
| organ restart | identity and accepted run state survive; an active run becomes `Recovering` | process integration |
| presenced restart | cognitive owners and lifecycle state remain unchanged | process integration |
| Presence/QML recreation | same run ID/status and unchanged Event1 count | process integration |
| Plasma shell recreation | replacement shell and D-Bus surface; exact run blob and Event1 count unchanged | `p4-plasma-lifecycle` VM |
| logout/login simulation | same identity, reconstructed intentions, one new logical session | process integration |
| reboot | same identity and exact active run; recovery is explicit | `lifecycle-continuity` VM |
| owner split commit | deterministic owner contribution is reused after crash/reboot | process and VM integration |
| terminal split commit | deterministic terminal outcome is reused after crash/reboot | process and VM integration |
| optional owner deficit | explicit missing capability and cause; completed run becomes `Degraded` | lifecycle tests |
| user interruption | asynchronous shell command; durable `Interrupted` transition owned by lifecycled | process integration |
| legacy lifecycle state | backup to `.pre-v1`, v0/v1 migration to schema v2 | lifecycle tests |

## Unsupported or not yet release-qualified

- in-place upgrade reconciliation across architecture or NixOS state-version changes;
- rollback after a partially applied system upgrade;
- corruption recovery beyond fail-closed startup and preserved source data;
- retention, forgetting, evidence-expiry, or general epistemic freshness policy;
- multi-node replication, partitions, and identity reconciliation;
- full capability-deficit behavior for loss of every optional organ;
- installer persistence and migration of an existing user biography;
- stable-release compatibility guarantees for pre-release Mind state.

## Reproducible evidence commands

```bash
nix build --print-build-logs \
  path:.#packages.x86_64-linux.cybou-mind \
  path:.#packages.x86_64-linux.cybou-presence-applet

nix build --print-build-logs \
  path:.#checks.x86_64-linux.lifecycle-continuity \
  path:.#checks.x86_64-linux.p4-plasma-lifecycle \
  path:.#checks.x86_64-linux.vm-smoke

nix flake check --print-build-logs path:.
```

`path:.` is appropriate for evaluating a complete local candidate including untracked files. A
published stable release must instead use a clean tagged Git revision. The evaluation candidate
below was built from a clean commit but is intentionally untagged.

## Artifact record

```text
version: M5 evaluation candidate (unversioned)
git revision: ddd6c8390334213896591ed6b88ad48f40900c6b
Nix flake revision: ddd6c8390334213896591ed6b88ad48f40900c6b
VM output: /nix/store/5gpzv3wsqmpgzd4wi8w7wlgcs5wfaf6i-nixos-vm
VM closure size: 8.9 GiB
ISO filename: cybou.iso
ISO output: /nix/store/qmkj3vclwhk9r35r06111sykpb4yl491-cybou.iso/iso/cybou.iso
ISO size: 3,379,068,928 bytes
ISO SHA-256: e7e84489c1ffeccf70cca7c0e69b21a019feb704be2ea3c1c8f68626c3b7e56e
test environment: NixOS under WSL2/KVM
state compatibility: pre-release; schema v1 migration is tested, cross-release upgrade is not
```

## Current release-gate status

- Mind/package, static documentation/UI, lifecycle continuity, focused P4 Plasma, and two-node
  `vm-smoke` gates pass.
- Aggregate `nix flake check --print-build-logs path:.` passes.
- The graphical reboot path uses direct systemd reboot because test-driver Ctrl+Alt+Delete opens
  Plasma's interactive logout UI; the corrected two-node gate completes under nested KVM.
- VM/ISO evaluation artifacts were built from clean revision `ddd6c83`; their paths, size, and
  SHA-256 are recorded above.

## Artifact provenance note

The evidence-record commit follows the artifact-source commit, so it can name that immutable
revision without a self-reference. This is an evaluation artifact record, not a stable release or
compatibility promise; stable publication still requires an explicit version and Git tag.

## Known limitations

The UI reports lifecycle request age, not general evidence freshness. Interruption is asynchronous
at the Plasma boundary, while internal organ RPC remains synchronous and bounded. `Recovering`
means reconciliation is required; it is not a claim that recovery has already succeeded.
