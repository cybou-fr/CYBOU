<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Rust Migration Plan

## Mandate and current truth

ADR-0038 declares the target: all authored executable product code moves to Rust, including the
Rust/WASM Living Canvas. R0 and the first W0 seam are now present: a locked workspace, shared typed
contracts, deterministic fixtures, CI/Nix entry points, and a browser-compiling UI shell. Mind is
still C++/Qt/CMake, installed Presence is QML/Plasma, and project automation still includes Python
and shell. Until each replacement gate passes, `CURRENT_STATE.md` and the existing binaries remain
authoritative.

## Non-negotiable invariants

The migration must preserve durable-before-visible ordering, canonical Journal history and hashes,
one writer, process ownership, identity continuity, typed D-Bus behavior, bounded calls, explicit
unknown/degraded states, named-consumer privacy, and UI-not-authorization. Language uniformity never
outranks these properties.

## Target stack

| Layer | Rust target | Compatibility seam |
|---|---|---|
| frontend | Leptos CSR, WASM, `web-sys` | versioned `/api/v1`, `MindClient`, browser standards |
| web boundary | Axum, Tokio, Tower, Rustls | explicit HTTP/SSE or WebSocket schemas |
| local fabric | zbus plus bounded resilience crate | current versioned D-Bus names and payloads |
| protocols | Serde domain types, CBOR/JSON adapters | golden C++ fixtures and canonical bytes |
| persistence | evaluated SQLx/rusqlite adapter | existing SQLite schemas, hashes, backups |
| services/organs | one binary crate per existing owner | D-Bus contract and black-box behavior |
| packaging | Cargo workspace built and pinned by Nix | parallel CMake package during transition |

Exact libraries must survive a spike and security review; the architectural boundaries are stable.

## Workstreams

### R0 — reproducible foundation

Create `Cargo.toml`, `Cargo.lock`, pinned Rust toolchain, Nix build, license policy, CI cache, clippy,
rustfmt, audit, deny, SBOM, and test-support crates. No production owner changes in this phase.

### R1 — contracts and differential oracle

Port value types before behavior. Export golden fixtures from the current implementation for
canonical envelopes, hashes, CBOR, sensitivity, health, lifecycle, outcomes, errors, and D-Bus
payloads. Run them in both languages. Add property/fuzz tests around parsers and canonicalization.

### R2 — Living Canvas and web contracts

Replace the exploratory React prototype with production Rust/WASM. Implement `MockMindClient`, the
state vocabulary, design system, keyboard/accessibility behavior, deterministic visual fixtures,
and the same-artifact local/hosted proof. Keep the prototype as visual evidence only.

### R3 — gateway and Presence adapter

Build the Rust gateway with hostile-input budgets, local bootstrap sessions, remote auth seams,
origin/CSRF enforcement, snapshots, resumable events, and typed mutations. Initially consume the
existing `Presence1` service through zbus; do not migrate Mind owners merely to unblock the UI.

### R4 — shared foundation

Migrate protocol, crypto adapter, state paths, IPC/fabric, event clients, resilience, and common
test support. C++ and Rust services interoperate in the same VM during this phase. Preserve the wire
format; do not expose Rust-native types across process boundaries.

### R5 — leaf and derived organs

Replace stateless or derived owners first, one process at a time: perception, predictor, epistemic,
context, workspace, self, intention, health, and Presence according to measured dependency risk.
The exact order is adjusted by the dependency graph, but every cutover is individually reversible.

### R6 — lifecycle and canonical Journal owners

Migrate `lifecycled` and `eventd` last. They have the highest continuity and persistence blast
radius. Require existing-database replay, canonical hash equality, concurrent-writer refusal,
split-commit recovery, migration interruption, rollback, scale, and multi-version fixtures.

### R7 — desktop replacement and legacy removal

Boot the Rust/WASM UI through the minimal Chromium desktop session. After web parity, Mind parity,
upgrade and recovery gates pass, remove QML/Plasma, then Qt/C++, then CMake packages and obsolete
validators. Python/shell validators, generators, website behavior, and automation follow after
product cutover so all first-party executable code reaches the same Rust policy. Declarative Nix,
schemas, CSS, metadata, documentation, and test data remain in their native formats.

## Cutover protocol for every component

1. Freeze and document its observable contract, state, resource budgets, and failure semantics.
2. Build Rust against golden fixtures and the live predecessor contract.
3. Run C++ and Rust black-box suites with identical inputs, clocks, faults, and state copies.
4. Deploy Rust opt-in with rollback to the unchanged persistent format.
5. Sabotage the claimed invariant so each gate proves the intended failure.
6. Switch the default only after VM reboot, crash, timeout, upgrade, and downgrade evidence passes.
7. Delete the predecessor only after one release/evaluation window establishes no rollback need.

Dual-running two canonical owners against the same writable state is forbidden. Shadow comparison
uses read-only copies, captured inputs, or a single authoritative writer with non-authoritative
observation.

## Definition of done

The migration is complete when the repository contains no Cybou-authored C++, Qt, QML, Python,
shell, JavaScript, or TypeScript executable code; one locked Rust workspace produces native
services, tools, and the Living Canvas WASM artifact; Nix builds it reproducibly; existing state
upgrades without history change; all continuity/security/accessibility gates pass; and removal of
the old toolchains is verified in the closure. Declarative and content formats are not rewritten
into Rust merely for cosmetic language purity.

## Immediate implementation slice

**Status: initial implementation present.** The workspace/toolchain, first protocol and web-contract
crates, nominal fixtures, `MockMindClient`, Rust/WASM shell, and native/WASM checks exist. Golden
C++ canonical-byte fixtures and the live Presence adapter remain the next contract work.

Start with R0 plus the R1 contract harness and R2 web-contract crate:

- workspace/toolchain/Nix skeleton;
- `cybou-protocol` and `cybou-web-contracts` crates without claiming owner behavior;
- golden fixtures generated and checked by the existing C++ tests;
- `living-canvas` WASM shell with `MockMindClient`;
- CI checks proving native Linux and `wasm32-unknown-unknown` builds.

This slice creates a typed Rust seam without risking canonical state.
