#!/bin/sh
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# Prepare one account's terminal socket directory and drop to that account.
#
# The privileged half is only this: creating a directory the gateway group may traverse. The owner
# itself never has gateway authority and refuses to run as root, so everything after the last line
# is that person and nothing more.
set -eu

if [ $# -lt 1 ]; then
    echo "Usage: $0 <username>" >&2
    exit 1
fi

USER_NAME="$1"
USER_UID="$(id -u "$USER_NAME")"
USER_SHELL="$(getent passwd "$USER_NAME" | cut -d: -f7)"

if [ -z "$USER_UID" ] || [ -z "$USER_SHELL" ]; then
    echo "Unknown user: $USER_NAME" >&2
    exit 1
fi

# The account's own login shell, from the passwd database. Not a list of likely paths: a terminal
# that opened a different shell from the one `chsh` records would be answering a question nobody
# asked, and the person would find out from their prompt.
if [ ! -x "$USER_SHELL" ]; then
    echo "The login shell for $USER_NAME is not executable: $USER_SHELL" >&2
    exit 1
fi

mkdir -p "/run/cybou-pty/$USER_UID"
chown "$USER_NAME:cybou" "/run/cybou-pty/$USER_UID"
chmod 0750 "/run/cybou-pty/$USER_UID"

if [ -S "/run/cybou-pty/$USER_UID/owner.sock" ]; then
    rm -f "/run/cybou-pty/$USER_UID/owner.sock"
fi

export CYBOU_PTY_SOCKET="/run/cybou-pty/$USER_UID/owner.sock"
export CYBOU_PTY_SHELL="$USER_SHELL"

exec runuser -u "$USER_NAME" --preserve-environment -- /usr/libexec/cybou/cybou-ptyd
