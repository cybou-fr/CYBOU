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
