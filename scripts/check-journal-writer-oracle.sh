#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# The writer differential gate: the predecessor Journal and the Rust writer are given identical
# contributions, and every stored row must be identical.
#
# The canonical bytes and their digests are already proven by check-fabric-oracle.sh. This gate
# covers what no canonical form does: how each writer spells what it stores. Two writers can agree
# on every hash and still disagree about whether an absent capability scope is NULL or an empty
# string, and a Journal written by the replacement would then be distinguishable from one written by
# the predecessor for a reason no verification would ever report.
#
# Both dumps go through SQLite's own quote(), so neither side formats anything of its own.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf -- "$work"' EXIT

moc="$(pkg-config --variable=libexecdir Qt6Core)/moc"
if [ ! -x "$moc" ]; then
  moc="$(command -v moc-qt6 || command -v moc)"
fi

# Journal and EventStore are QObjects, so the oracle needs their generated meta-objects; the
# existing oracles compile plain classes and do not.
"$moc" -I"$root/mind/foundation/storage/include" \
  -I"$root/mind/foundation/events/include" \
  -I"$root/mind/protocol/include" \
  -I"$root/mind/foundation/crypto/include" \
  "$root/mind/foundation/storage/include/cybou/storage/Journal.h" \
  -o "$work/moc_Journal.cpp"
"$moc" -I"$root/mind/foundation/events/include" \
  -I"$root/mind/protocol/include" \
  "$root/mind/foundation/events/include/cybou/events/EventStore.h" \
  -o "$work/moc_EventStore.cpp"

c++ -std=c++20 \
  "$root/migration/oracles/journal_writer_fixture.cpp" \
  "$work/moc_Journal.cpp" \
  "$work/moc_EventStore.cpp" \
  "$root/mind/foundation/storage/src/Journal.cpp" \
  "$root/mind/foundation/events/src/EnvelopeCodec.cpp" \
  "$root/mind/foundation/crypto/src/KeyStore.cpp" \
  "$root/mind/foundation/crypto/src/SealedPayload.cpp" \
  "$root/mind/protocol/src/CognitiveEnvelope.cpp" \
  "$root/mind/protocol/src/CanonicalEnvelope.cpp" \
  "$root/mind/protocol/src/Sensitivity.cpp" \
  -I"$root/mind/foundation/storage/include" \
  -I"$root/mind/foundation/events/include" \
  -I"$root/mind/foundation/crypto/include" \
  -I"$root/mind/protocol/include" \
  $(pkg-config --cflags --libs Qt6Core Qt6Sql libsodium) \
  -o "$work/journal-writer-oracle"

"$work/journal-writer-oracle" >"$work/qt.txt"

# The Rust half writes through cybou-storage::writer and dumps the same way.
rust_work="$work/rust"
mkdir -p "$rust_work"
cargo run --quiet --locked --manifest-path "$root/Cargo.toml" \
  -p cybou-storage --bin cybou-journal-writer-fixture -- "$rust_work" >"$work/rust.txt"

# Compared against each other first: a checked-in fixture that both sides drifted away from
# together would still pass, and the point of this gate is that they cannot drift apart.
if ! diff -u "$work/qt.txt" "$work/rust.txt"; then
  echo "journal-writer-oracle: the Qt and Rust writers store different rows" >&2
  exit 1
fi

# Then against the recorded fixture, so a change to what both write is visible in review rather
# than only in a passing gate.
cmp "$work/rust.txt" "$root/fixtures/storage/journal-writer-v3.txt"

echo "journal-writer-oracle: Qt and Rust store byte-identical Journal rows"
