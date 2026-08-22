#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# The Debian-native CYBOU Desktop session: one compositor, one surface, one origin.
#
# This is deliberately the smallest thing that can be called a session. Cage owns the display and
# shows exactly one window; Chromium draws Living Canvas from the loopback gateway. There is no
# panel, no launcher, no window management and no second application, because the desktop is inside
# the surface rather than around it.
#
# What it is not: a compositor Cybou wrote, a shell, or a replacement for a desktop environment.
# Calling this an operating system's GUI would be claiming a great deal more than one Chromium
# window in kiosk mode.

set -euo pipefail

# Where the gateway is. Loopback, always: the gateway refuses to bind anything else, and a session
# that pointed at a public address would be showing a filtered projection to the person the
# projection is filtered for.
CYBOU_DESKTOP_URL="${CYBOU_DESKTOP_URL:-http://127.0.0.1:8787/}"

# Where the desktop keeps what the person arranged.
#
# The layout lives in the browser's localStorage, so a fresh profile on every start is a desktop
# that forgets where every card was put. The previous session used an ephemeral profile and lost it
# each time. This one is durable and is state, not biography: it belongs under XDG_STATE_HOME and
# never in the Journal, because where a person likes their windows is not something the Mind should
# remember about them.
CYBOU_DESKTOP_STATE="${CYBOU_DESKTOP_STATE:-${XDG_STATE_HOME:-$HOME/.local/state}/cybou/desktop}"
CHROMIUM_PROFILE="$CYBOU_DESKTOP_STATE/chromium"

# Which browser binary. Debian ships `chromium`; some hosts only have `chromium-browser`.
pick_browser() {
    if [ -n "${CYBOU_DESKTOP_BROWSER:-}" ]; then
        # Checked rather than taken on trust. An override naming a binary that is not there used to
        # be accepted, and the session then failed inside the compositor with an error about
        # something else entirely.
        if command -v "$CYBOU_DESKTOP_BROWSER" >/dev/null 2>&1; then
            printf '%s' "$CYBOU_DESKTOP_BROWSER"
            return 0
        fi
        echo "ERROR: CYBOU_DESKTOP_BROWSER=$CYBOU_DESKTOP_BROWSER is not executable." >&2
        return 1
    fi
    for candidate in chromium chromium-browser; do
        if command -v "$candidate" >/dev/null 2>&1; then
            printf '%s' "$candidate"
            return 0
        fi
    done
    return 1
}

# The command Cage will run. Printed rather than only executed so a gate can check it without
# starting a compositor, and so a person can see what their session actually launches.
browser_argv() {
    local browser="$1"
    printf '%s\n' \
        "$browser" \
        --ozone-platform=wayland \
        --user-data-dir="$CHROMIUM_PROFILE" \
        --app="$CYBOU_DESKTOP_URL" \
        --no-first-run \
        --no-default-browser-check \
        --disable-features=TranslateUI \
        --disable-pinch
}

main() {
    local browser
    if ! browser="$(pick_browser)"; then
        echo "ERROR: no Chromium binary found; set CYBOU_DESKTOP_BROWSER." >&2
        exit 1
    fi

    if [ "${1:-}" = "--print-argv" ]; then
        # What would be run, without running it.
        browser_argv "$browser"
        return 0
    fi

    if ! command -v cage >/dev/null 2>&1; then
        echo "ERROR: cage is not installed; the session has no compositor to run in." >&2
        exit 1
    fi

    mkdir -p "$CHROMIUM_PROFILE"

    # Waiting rather than assuming. Chromium started before the gateway answers shows its own error
    # page, and a person then has a desktop that says the system is down when it is merely late.
    local deadline=$((SECONDS + 30))
    until curl -fsS -o /dev/null --max-time 2 "${CYBOU_DESKTOP_URL}api/v1/session" 2>/dev/null; do
        if [ "$SECONDS" -ge "$deadline" ]; then
            echo "ERROR: $CYBOU_DESKTOP_URL did not answer within 30s; not starting a session." >&2
            exit 1
        fi
        sleep 1
    done

    local argv=()
    while IFS= read -r arg; do argv+=("$arg"); done < <(browser_argv "$browser")

    echo "==> CYBOU Desktop session: cage -> ${argv[0]} -> $CYBOU_DESKTOP_URL"
    echo "==> Desktop state: $CHROMIUM_PROFILE"
    exec cage -- "${argv[@]}"
}

main "$@"
