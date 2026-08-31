<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0004: CI Workflow

## Status
Accepted

## Context
Cybou needs a Continuous Integration workflow to ensure code quality and catch issues early. The workflow should:
- Run quickly on every push (fast feedback)
- Be comprehensive on releases (full validation)
- Not duplicate what `nix flake check` already does
- Work within GitHub Actions constraints (14GB disk, no KVM)

## Decision
**Split CI into two jobs:**

### Fast Job (every push)
Runs on every push and pull request:
- Nix formatting validation (`nix fmt`)
- REUSE license compliance (`reuse lint`)
- Package metadata validation (`scripts/validate-packages.py`)
- Rust workspace compilation (`cargo build --workspace`)
- Formatting check (`nix fmt && git diff --exit-code`)

### Full Job (tags only)
Runs only when a tag is pushed:
- Complete `nix flake check`
- VM build (requires KVM, will fail on hosted runners without KVM)

## Rationale

### Why Split?
1. **Speed**: Fast job runs in minutes, providing quick feedback
2. **Cost**: Full job is expensive (VM build), only run on releases
3. **Reliability**: Fast job catches most issues without VM overhead
4. **Constraints**: Hosted runners don't have KVM, so VM build would always fail

### Why Not Run VM on Every Push?
- VM build takes significant time and disk space
- Hosted runners have ~14GB disk, VM image exceeds this
- KVM not available on hosted runners
- Would train developers to ignore red CI (false failures)

### Why Not Duplicate Checks?
`nix flake check` already runs formatting, REUSE, and package-metadata validation. Running these separately would create a second source of truth that could drift.

## Implementation

### .github/workflows/checks.yml
```yaml
jobs:
  fast:
    runs-on: ubuntu-latest
    steps:
      - nix build .#checks.x86_64-linux.formatting
      - nix build .#checks.x86_64-linux.reuse
      - nix build .#checks.x86_64-linux.package-metadata
      - nix build .#packages.x86_64-linux.cybou-mind
      - nix build .#packages.x86_64-linux.cybou-presence-applet
      - nix fmt && git diff --exit-code

  full:
    if: startsWith(github.ref, 'refs/tags/')
    runs-on: ubuntu-latest
    steps:
      - nix flake check
      - nix build .#nixosConfigurations.cybou-vm.config.system.build.vm
```

### Not in CI
- ISO build: Too large for hosted runners, built locally in WSL
- SHA256 verification: Manual step, published with release

## Consequences

### Positive
- Fast feedback on every push
- Comprehensive validation on releases
- No duplicated checks
- Respects hosted runner constraints
- Clear separation of concerns

### Negative
- VM not tested on every push
- ISO not built in CI
- Need manual steps for full release validation

## Related
- .github/workflows/checks.yml - CI configuration
- ADR-0005 - Calamares installer (ISO built locally, not in CI)
- ADR-0006 - State version pinning (affects what CI tests)
