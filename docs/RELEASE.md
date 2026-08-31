<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Release Process

## Versioning

Use semantic pre-release versions during pre-release development:

```text
v0.1.0-alpha.1
v0.1.0-alpha.2
```

## Release gates

- fast and full CI are green;
- all packages build from the tagged commit;
- workspace unit and integration test suites pass (100% pass);
- SHA256 checksums are generated;
- known limitations are documented;
- CHANGELOG is updated;
- persistent-state compatibility is stated;
- rollback instructions exist;
- `CURRENT_STATE.md` matches the tagged implementation;
- proposed lifecycle/epistemic behavior is not presented as shipped;
- documentation link/cognitive validators pass;
- any state migration has interruption and recovery evidence.

## Artifacts

A release may include:

```text
Cargo workspace source tarball
Living Canvas WASM frontend bundle
SHA256SUMS
release notes
```

## Reproducible release outline

```bash
git status --short
cargo build --workspace --release --locked
cargo test --workspace --locked
cargo check --target wasm32-unknown-unknown -p living-canvas
python scripts/validate-cognitive-docs.py
python scripts/validate-doc-links.py
python scripts/sync-site-i18n.py --check
```

Run release commands from the clean tagged tree. Record the commit revision, artifact hashes, test
environment, known limitations, and whether persistent state can be upgraded or must be reset.
Use [Evidence](evidence/README.md) as the template for what a candidate must be able to show.

## Release notes must distinguish

- implemented capability from proposed architecture;
- local developer evaluation from production deployment;
- supported migration from untested state reuse;
- local-first runtime from future optional external model faculties.
