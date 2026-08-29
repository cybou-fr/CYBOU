#!/bin/sh
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# Helper runner to configure the per-UID socket directory and drop privileges to the owner.
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

# Prepare per-UID runtime socket directory with group traversal for cybou
mkdir -p "/run/cybou-host-files/$USER_UID"
chown "$USER_NAME:cybou" "/run/cybou-host-files/$USER_UID"
chmod 0750 "/run/cybou-host-files/$USER_UID"

# Clean up stale dead socket if any
if [ -S "/run/cybou-host-files/$USER_UID/owner.sock" ]; then
    rm -f "/run/cybou-host-files/$USER_UID/owner.sock"
fi

export CYBOU_HOST_FILES_HOME="$USER_HOME"
export CYBOU_HOST_FILES_SOCKET="/run/cybou-host-files/$USER_UID/owner.sock"

# Execute daemon dropped to user
exec runuser -u "$USER_NAME" -- /usr/libexec/cybou/cybou-host-filesd
