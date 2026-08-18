#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# Compare checked-in Rust fixtures with byte streams emitted by the predecessor Qt codec.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf -- "$work"' EXIT

c++ -std=c++20 \
  "$root/migration/oracles/fabric_v1_fixture.cpp" \
  "$root/mind/foundation/fabric/src/FabricCodec.cpp" \
  -I"$root/mind/foundation/fabric/include" \
  $(pkg-config --cflags --libs Qt6Core) \
  -o "$work/fabric-v1-oracle"

"$work/fabric-v1-oracle" >"$work/oracle.txt"
sed -n 's/^map=//p' "$work/oracle.txt" >"$work/map.hex"
sed -n 's/^list=//p' "$work/oracle.txt" >"$work/list.hex"

cmp "$work/map.hex" "$root/fixtures/fabric/v1/map.hex"
cmp "$work/list.hex" "$root/fixtures/fabric/v1/list.hex"
echo "fabric-v1-oracle: Qt and Rust fixtures are byte-identical"

c++ -std=c++20 \
  "$root/migration/oracles/observation_v1_fixture.cpp" \
  "$root/mind/protocol/src/Observation.cpp" \
  -I"$root/mind/protocol/include" \
  $(pkg-config --cflags --libs Qt6Core) \
  -o "$work/observation-v1-oracle"

"$work/observation-v1-oracle" >"$work/observation-oracle.txt"
sed -n 's/^payload=//p' "$work/observation-oracle.txt" >"$work/observation.hex"
sed -n 's/^message-id=//p' "$work/observation-oracle.txt" >"$work/message-id.txt"

cmp "$work/observation.hex" "$root/fixtures/protocol/observation-v1.hex"
cmp "$work/message-id.txt" "$root/fixtures/protocol/observation-v1-message-id.txt"
echo "observation-v1-oracle: Qt payload and identity match Rust fixtures"

c++ -std=c++20 \
  "$root/migration/oracles/canonical_envelope_fixture.cpp" \
  "$root/mind/protocol/src/CanonicalEnvelope.cpp" \
  -I"$root/mind/protocol/include" \
  $(pkg-config --cflags --libs Qt6Core) \
  -o "$work/canonical-envelope-oracle"

"$work/canonical-envelope-oracle" >"$work/canonical-oracle.txt"
for key in envelope-v2 nonerasable-v3 journal-row-v2 envelope-v2-sha256 nonerasable-v3-sha256 \
  payload-v3-sha256 commitment-v3 journal-row-v3 journal-row-v3-sha256; do
  sed -n "s/^$key=//p" "$work/canonical-oracle.txt" >"$work/$key.hex"
  cmp "$work/$key.hex" "$root/fixtures/protocol/$key.hex"
done
echo "canonical-envelope-oracle: Qt canonical bytes and digests match Rust fixtures"
