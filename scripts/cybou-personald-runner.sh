#!/bin/sh
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# Helper runner to prepare one account's Personal Core store and socket, then drop to that account.
# The store lives under the user's own home, so the records belong to the person on the filesystem
# as well as in the process table.
set -eu

if [ $# -lt 1 ]; then
    echo "Usage: $0 <username>" >&2
    exit 1
fi

USER_NAME="$1"
USER_UID="$(id -u "$USER_NAME")"
USER_HOME="$(getent passwd "$USER_NAME" | cut -d: -f6)"

if [ -z "$USER_UID" ] || [ -z "$USER_HOME" ]; then
    echo "Unknown user: $USER_NAME" >&2
    exit 1
fi

# Per-UID runtime socket directory, traversable by the gateway group and nobody else.
mkdir -p "/run/cybou-personal/$USER_UID"
chown "$USER_NAME:cybou" "/run/cybou-personal/$USER_UID"
chmod 0750 "/run/cybou-personal/$USER_UID"

if [ -S "/run/cybou-personal/$USER_UID/personal.sock" ]; then
    rm -f "/run/cybou-personal/$USER_UID/personal.sock"
fi

# The store is the user's own data, kept in their home and readable by nobody else.
mkdir -p "$USER_HOME/.local/share/cybou"
chown "$USER_NAME" "$USER_HOME/.local/share/cybou"
chmod 0700 "$USER_HOME/.local/share/cybou"

export CYBOU_PERSONAL_STORE="$USER_HOME/.local/share/cybou/personal.sqlite3"
export CYBOU_PERSONAL_SOCKET="/run/cybou-personal/$USER_UID/personal.sock"

exec runuser -u "$USER_NAME" --preserve-environment -- /usr/libexec/cybou/cybou-personald
