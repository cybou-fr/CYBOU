<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0038: Rust-First Product Codebase

## Status

Accepted

## Context

The current Mind is C++/Qt, its local fabric is Qt D-Bus, and its shipped interface is QML/Plasma.
ADR-0037 replaces the presentation with one browser-delivered frontend. Maintaining C++, QML,
JavaScript/TypeScript, and a gateway language would create several ownership models, type systems,
build graphs, and security review surfaces.

The project therefore needs an explicit language destination and a migration rule that preserves
the already proved Journal, continuity, process-isolation, failure, and wire-contract behavior.

## Decision

### Rust is the product implementation language

All new executable Cybou product components are written in Rust. The target includes:

- Living Canvas as Rust compiled to WebAssembly;
- `cybou-web-gateway` and its authentication/transport adapters;
- Mind protocols, storage, services, organs, clients, and command-line tools;
- desktop session launchers and native integration owned by Cybou;
- validators, generators, migration utilities, website behavior, and repository automation owned by
  Cybou.

The target production tree contains no hand-authored C++, QML, JavaScript, or TypeScript.
WebAssembly tooling may emit a minimal JavaScript loader; it is generated output, not an authored
application layer and is never edited or treated as an authority boundary.

Nix expressions, systemd declarations, schemas, interface descriptions, SQL migrations, HTML
metadata, CSS/design tokens, documentation, test fixtures/data, and third-party code are
declarative or external artifacts rather than authored executable-language exceptions. Existing
Python, shell, and website JavaScript may remain only until their Rust replacement phase; new
product behavior may not be added to them.

### One Cargo workspace

The target repository has one locked Cargo workspace with crates grouped by responsibility:

```text
crates/
  protocol/        canonical domain and wire types
  storage/         Journal and owner-specific persistence
  fabric/          bounded D-Bus clients/servers and resilience
  runtime/         state paths, identity, lifecycle helpers
  organs/          one binary crate per owner process
  presence/        presentation aggregation
  web-contracts/   HTTP/event/session DTOs and schema generation
  web-gateway/     browser/network boundary
  living-canvas/   Rust/WASM frontend
  desktop-shell/   Chromium/session policy launcher
  test-support/    fixtures, clocks, fault injection, differential harness
```

Crate boundaries follow domain ownership. Rust adoption does not authorize a monolith.

### Frontend is Rust/WASM

Living Canvas uses a client-side Rust component framework and browser APIs through
`wasm-bindgen`/`web-sys`. The initial baseline is Leptos CSR because it produces a browser-native
WASM application without requiring a native desktop wrapper. A framework change is allowed only
behind the `MindClient`, component, and web-contract boundaries.

The same content-hashed WASM, generated loader, CSS, fonts, and assets run in local Chromium and in
the hosted browser. There is no desktop frontend fork and no privileged native frontend bridge.

### Native service baseline

The initial native baseline is Tokio for bounded async work, Axum for the explicit HTTP/event
gateway, zbus for compatibility with existing versioned D-Bus interfaces, Serde for typed
serialization, and SQLx/rusqlite evaluated against canonical SQLite and migration gates. Library
selection does not change owners or contracts and must be pinned and audited.

### Migration is contract-preserving replacement with executable oracles

The rewrite proceeds vertically, one owner or boundary at a time. C++/Qt/NixOS is frozen as
compatibility evidence and an executable oracle; the new production runtime is Rust/Debian
immediately. Legacy binaries are not maintained as a parallel production product.

A Rust replacement must validate against the predecessor's wire and storage contracts via differential
oracles before cutting over. For persistent owners, acceptance requires reading existing state without
reinterpreting history, writing byte/semantic-compatible canonical records, fail-closed migration,
crash recovery, and rollback evidence.

No production state is bulk-converted merely to make it Rust-shaped. Stored schemas and canonical
hash inputs change only through their own versioned ADR and migration.

### New work rule

- New web UI, gateway, remote-access, M8+, and desktop replacement implementation is Rust-only.
- Existing C++ receives security, correctness, compatibility, and migration-seam fixes.
- Net-new long-lived features are not added to a C++ owner when its Rust replacement phase has
  started, unless required to preserve an accepted invariant or unblock differential migration.
- No unsafe Rust is permitted in domain crates. Boundary crates require isolated `unsafe`, a
  documented invariant, tests, and review.

## Migration gates

| | Gate |
|---|---|
| **R1** | Locked Cargo workspace builds reproducibly in CI on Debian 13 |
| **R2** | Shared protocol fixtures pass in C++ oracle and Rust, including canonical hashes and failure values |
| **R3** | Rust fabric preserves timeouts, caller identity, bounded retries, and unknown-outcome semantics |
| **R4** | Each Rust owner passes black-box differential tests against its C++ predecessor oracle |
| **R5** | Existing Journal and owner state open without destructive conversion; interruption and rollback tests pass |
| **R6** | Rust/WASM Living Canvas is the identical content-hashed artifact in local and hosted modes |
| **R7** | Gateway and frontend contain no hand-authored JavaScript/TypeScript and expose no native bridge |
| **R8** | Renderer, gateway, and every migrated owner can crash independently without violating continuity |
| **R9** | Audit, license, SBOM, clippy, format, unit, integration, fuzz/property, and Debian gates pass |
| **R10** | C++/Qt/CMake, QML/Plasma, Python/shell tooling, and authored JavaScript are removed only after owning replacements pass their gates |

## Consequences

Positive consequences are one primary type system and toolchain, shared end-to-end DTOs, memory-safe
defaults at hostile boundaries, reusable native/WASM domain code, and less Qt-specific coupling.

Costs are managing differential test fixtures and oracle harnesses during migration, WASM bundle/startup
work, browser accessibility testing, Rust async complexity, and semantic-drift risk. Rust removes classes
of memory bugs; it does not provide authorization, privacy, correctness, boundedness, or continuity
automatically.

## Rejected alternatives

### Unvalidated rewrite without oracles

Rejected because dropping differential and oracle verification against predecessor behavior destroys
grounded invariants and combines storage, IPC, UI, and deployment risk into an unreviewable transition.
The legacy implementation is preserved as an executable oracle until replacement gates pass.

### Rust backend with TypeScript frontend

Rejected by the product-language decision. It is conventional, but keeps a second authored runtime
and duplicates contracts at the most exposed boundary.

### Tauri as the desktop architecture

Rejected because local and hosted rendering must be the same Chromium-targeted web artifact. A
native wrapper may not introduce a second capability path around the gateway.

## Related documents

- [Rust Migration Plan](../history/RUST_MIGRATION.md)
- [Web UI Architecture](../WEB_UI_ARCHITECTURE.md)
- [ADR-0037](ADR-0037-web-first-presence-and-desktop.md)
- [ADR-0010](ADR-0010-journal-v2-schema-and-canonical-hashing.md)
- [ADR-0012](ADR-0012-organ-process-isolation-and-lifecycle.md)
- [ADR-0013](ADR-0013-local-cognitive-fabric-qt-dbus.md)
