#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# What a browser is actually sent when it asks this gateway for the desktop.
#
# Deliberately not a browser. The first version of this drove headless Chromium with
# `--virtual-time-budget`, because `--dump-dom` fires on the load event and a WebAssembly desktop
# has mounted nothing by then. That budget waits for the page to fall idle, and this page never
# does: the Dock and the Terminal card both hold repeating timers, so Chromium advanced virtual
# time and fired them for two hours and sixteen minutes before it was killed. The technique is
# unusable against any page with an interval on it, which is every desktop worth shipping.
#
# What a browser does with the bundle is covered where it can be: `cargo test -p living-canvas
# --target wasm32-unknown-unknown` mounts real components in a real Chromium through
# `wasm-bindgen-test`, which settles on a microtask rather than on an idle page. This gate covers
# the other half, which that one cannot see at all — whether the thing the browser is handed is
# the thing that was built.

set -euo pipefail

cd "$(dirname "$0")/.."

if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi

if ! command -v trunk >/dev/null 2>&1; then
    echo "==> desktop delivery gate NOT RUN: trunk is required (see docs/BUILDING.md)" >&2
    exit 3
fi

WORK="$(mktemp -d)"
PORT="${CYBOU_DELIVERY_PORT:-18711}"
BASE="http://127.0.0.1:$PORT"
gateway_pid=

cleanup() {
    status=$?
    if [ "$status" -ne 0 ] && [ -f "$WORK/gateway.log" ]; then
        cat "$WORK/gateway.log" >&2
    fi
    [ -n "$gateway_pid" ] && kill "$gateway_pid" >/dev/null 2>&1 || true
    [ -n "$gateway_pid" ] && wait "$gateway_pid" >/dev/null 2>&1 || true
    rm -rf "$WORK"
    return "$status"
}
trap cleanup EXIT

echo "==> Building the frontend..."
(cd crates/living-canvas && trunk build --release >/dev/null)

# Served from a local path. On a workspace mounted from another operating system the eight-megabyte
# module takes minutes to read, and a gate that spends them is a gate nobody runs.
cp -r target/living-canvas "$WORK/web"

echo "==> Serving it..."
cargo build --quiet -p cybou-web-gateway
CYBOU_WEB_ROOT="$WORK/web" \
CYBOU_GATEWAY_ADDR="127.0.0.1:$PORT" \
CYBOU_GATEWAY_FIXTURE=1 \
    target/debug/cybou-web-gateway >"$WORK/gateway.log" 2>&1 &
gateway_pid=$!

for _ in $(seq 1 50); do
    if curl -fs -o /dev/null "$BASE/"; then
        break
    fi
    sleep 0.2
done
curl -fs -o /dev/null "$BASE/"

echo "=== The page names a module, and the module is there ==="
curl -fs "$BASE/" >"$WORK/index.html"
WASM="$(grep -o '/living-canvas-[a-z0-9]*_bg\.wasm' "$WORK/index.html" | head -1)"
JS="$(grep -o '/living-canvas-[a-z0-9]*\.js' "$WORK/index.html" | head -1)"
CSS="$(grep -o '/styles-[a-z0-9]*\.css' "$WORK/index.html" | head -1)"

for asset in "$WASM" "$JS" "$CSS"; do
    if [ -z "$asset" ]; then
        echo "ERROR: the served index names no such asset" >&2
        exit 1
    fi
    if ! curl -fs -o /dev/null "$BASE$asset"; then
        # An index that names a file the gateway will not serve is a desktop that loads a blank
        # page and says nothing, which is the failure this line exists to catch.
        echo "ERROR: the index names $asset and the gateway does not serve it" >&2
        exit 1
    fi
done
echo "    ok      $WASM, $JS and $CSS are all served"

echo "=== A WebAssembly module rather than something shaped like one ==="
# The first four bytes are the magic number. A gateway misconfigured to answer every path with
# index.html would serve HTML here, and a browser's only complaint is a console line nobody sees.
# Written out rather than piped into `head`: closing the pipe after four bytes sends curl a
# SIGPIPE, and under `pipefail` that fails the gate on a module that arrived perfectly.
curl -fs -o "$WORK/module.wasm" "$BASE$WASM"
magic="$(od -An -tx1 -N4 "$WORK/module.wasm" | tr -d ' \n')"
if [ "$magic" != "0061736d" ]; then
    echo "ERROR: $WASM does not begin with the WebAssembly magic number (saw $magic)" >&2
    exit 1
fi
echo "    ok      it begins 00 61 73 6d"

echo "=== It is compressed on the way out ==="
raw="$(curl -fs -H 'Accept-Encoding: identity' -o /dev/null -w '%{size_download}' "$BASE$WASM")"
br="$(curl -fs -H 'Accept-Encoding: br' -o /dev/null -w '%{size_download}' "$BASE$WASM")"
gz="$(curl -fs -H 'Accept-Encoding: gzip' -o /dev/null -w '%{size_download}' "$BASE$WASM")"
echo "    identity $raw   gzip $gz   br $br"
if [ "$br" -ge "$raw" ] || [ "$gz" -ge "$raw" ]; then
    # This was the state of the deployment until 2026-08-30: eight megabytes sent whole, four
    # fifths of a cold load spent on bytes nobody needed to send.
    echo "ERROR: the module is not being compressed" >&2
    exit 1
fi

echo "=== It is installable ==="
manifest_type="$(curl -fs -o /dev/null -w '%{content_type}' "$BASE/manifest.webmanifest")"
case "$manifest_type" in
    application/manifest+json*) ;;
    *)
        echo "ERROR: the manifest is served as '$manifest_type'" >&2
        exit 1
        ;;
esac
curl -fs "$BASE/manifest.webmanifest" >"$WORK/manifest.json"
for icon in $(grep -o '"/[a-z-]*\.svg"' "$WORK/manifest.json" | tr -d '"'); do
    if ! curl -fs -o /dev/null "$BASE$icon"; then
        echo "ERROR: the manifest names $icon and the gateway does not serve it" >&2
        exit 1
    fi
done
echo "    ok      $manifest_type, and every icon it names is served"

echo "=== desktop delivery gate passed ==="
