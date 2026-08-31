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
- Ensure fast, reliable, reproducible CI execution
- Work within GitHub Actions constraints (14GB disk, no KVM)

## Decision
**Split CI into two jobs:**

### Fast Job (every push)
Runs on every push and pull request:
- Cargo formatting validation (`cargo fmt --check`)
- REUSE license compliance (`reuse lint`)
- Package metadata validation (`scripts/validate-packages.py`)
- Rust workspace compilation (`cargo build --workspace`)
- Formatting check (`cargo fmt --check`)

### Full Job (tags only)
Runs only when a tag is pushed:
- Complete workspace tests (`cargo test --workspace`)
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
The CI pipeline runs cargo fmt, cargo clippy, REUSE, and workspace test suites.

## Implementation

### .github/workflows/checks.yml
```yaml
jobs:
  fast:
    runs-on: ubuntu-latest
    steps:
      - run: cargo fmt --check
      - run: cargo clippy --workspace --all-targets --locked
      - run: cargo test --workspace --locked
      - run: cargo check --target wasm32-unknown-unknown -p living-canvas
      - run: reuse lint
      - run: python scripts/validate-cognitive-docs.py
      - run: python scripts/validate-doc-links.py
      - run: python scripts/sync-site-i18n.py --check
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
- ADR-0039 - Debian 13 Base System
- ADR-0006 - State version pinning (affects what CI tests)
