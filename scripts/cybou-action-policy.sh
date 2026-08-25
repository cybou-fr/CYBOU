#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# Replace the operator-owned unattended Action1 policy with one closed, validated list.

set -euo pipefail

if [ "$(id -u)" -ne 0 ]; then
    echo "cybou-action-policy must be run as root" >&2
    exit 1
fi
if [ "$#" -ne 1 ]; then
    echo "usage: cybou-action-policy none|service.status[,package.cache.clean,service.restart]" >&2
    exit 2
fi

requested="$1"
if [ "$requested" = none ]; then
    requested=""
fi

canonical=()
IFS=',' read -r -a verbs <<<"$requested"
for verb in "${verbs[@]}"; do
    [ -z "$verb" ] && continue
    case "$verb" in
        service.status | package.cache.clean | service.restart) ;;
        *)
            echo "unsupported unattended action: $verb" >&2
            exit 2
            ;;
    esac
    for seen in "${canonical[@]}"; do
        if [ "$seen" = "$verb" ]; then
            echo "action appears more than once: $verb" >&2
            exit 2
        fi
    done
    canonical+=("$verb")
done

policy_path="${CYBOU_ACTION_POLICY_PATH:-/etc/cybou/action-policy.env}"
policy_dir="$(dirname "$policy_path")"
install -d -m 0755 -o root -g root "$policy_dir"
temporary="$(mktemp "$policy_dir/.action-policy.XXXXXX")"
cleanup() {
    rm -f "$temporary"
}
trap cleanup EXIT

joined=""
if [ "${#canonical[@]}" -gt 0 ]; then
    joined="$(IFS=,; printf '%s' "${canonical[*]}")"
fi
printf 'CYBOU_PREAUTHORIZED_ACTIONS=%s\n' "$joined" >"$temporary"
chown root:root "$temporary"
chmod 0644 "$temporary"
mv -f "$temporary" "$policy_path"
trap - EXIT

if [ -z "${CYBOU_ACTION_POLICY_NO_RESTART:-}" ]; then
    machine="${CYBOU_ACTION_USER_MACHINE:-cybou@.host}"
    systemctl --user --machine="$machine" restart cybou-actiond.service
fi
printf 'Action1 unattended policy: %s\n' "${joined:-nothing}"
