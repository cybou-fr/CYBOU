#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT

# Fetch one immutable upstream OpenCode release, verify the ACP-registry digest, and install it at
# the read-only path the pack command names. This script never reads or writes provider credentials.
set -euo pipefail

destination="${1:-/usr/local/libexec/cybou/agents/opencode/1.18.23}"
version=1.18.23
case "$(uname -m)" in
  x86_64)
    archive=opencode-linux-x64.tar.gz
    digest=ab7015cd8113e011a461f30a0c2b77d8299a144ff688cb62e93e8802835d7288
    ;;
  aarch64)
    archive=opencode-linux-arm64.tar.gz
    digest=86d3afaf4e8784f9adab189be2a315c12b27ec40a04b70defbe70595c3cc7c65
    ;;
  *) echo "unsupported OpenCode pack architecture: $(uname -m)" >&2; exit 2 ;;
esac

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
url="https://github.com/anomalyco/opencode/releases/download/v${version}/${archive}"
curl --fail --location --proto '=https' --tlsv1.2 --output "$work/$archive" "$url"
printf '%s  %s\n' "$digest" "$work/$archive" | sha256sum --check --status
tar -xzf "$work/$archive" -C "$work"
test -x "$work/opencode"
install -d -m 0755 "$destination"
install -m 0755 "$work/opencode" "$destination/opencode"
"$destination/opencode" --version
