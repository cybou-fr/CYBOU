#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# One launch, carried out, and then nothing left behind.
#
# A host gate. It starts a real per-capsule gateway through its system unit, runs a real capsule
# under a real cgroup, and then checks that the teardown removed every runtime file and stopped every
# unit. It needs a deployed host: the unit template, the polkit rule that lets the session owner
# start it, an operator-selected provider, bubblewrap, and a user service manager.
#
# Exit 3 means a precondition is absent, which is a check that did not run — never one that passed.
# The distinction is the whole reason this file separates the two.
set -euo pipefail

cd "$(dirname "$0")/.."
if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/cybou-target}"

not_run() {
    echo "==> agent launch gate NOT RUN: $1" >&2
    exit 3
}

command -v bwrap >/dev/null || not_run "bubblewrap is not installed here"
command -v systemd-run >/dev/null || not_run "systemd-run is not available"
systemctl --user is-system-running >/dev/null 2>&1 || not_run "there is no user service manager"
test -f /etc/systemd/system/cybou-agent-gateway@.service ||
    not_run "the gateway unit template is not deployed"
test -s /etc/cybou/provider.env || not_run "no provider policy is configured"
grep -q '^CYBOU_LITELLM_BASE_URL=.' /etc/cybou/provider.env ||
    not_run "the provider policy is still fail-closed"
test -s /etc/cybou/litellm-master-key || not_run "no provider credential is installed"
test -w /run/cybou-agent-leases || not_run "this user cannot write the session launch directory"
# The lease this run writes has to be readable by the account the gateway unit runs as, which is the
# account that owns the launch directory in a deployment. Run by anybody else — root included — the
# lease lands unreadable and the gateway fails with a permission error three layers down, which
# reads like a broken product rather than a proof run by the wrong user.
gateway_user="$(sed -n 's/^User=//p' /etc/systemd/system/cybou-agent-gateway@.service | head -1)"
launch_owner="$(stat -c '%U' /run/cybou-agent-leases)"
if [ -n "$gateway_user" ] && [ "$launch_owner" != "$gateway_user" ]; then
    not_run "the launch directory belongs to $launch_owner and the gateway runs as $gateway_user"
fi
if [ -n "$gateway_user" ] && [ "$(id -un)" != "$gateway_user" ]; then
    not_run "this proof writes leases the gateway must read, so it runs as $gateway_user"
fi
for program in cybou-capsule-enter cybou-model-bridge cybou-egress-bridge cybou-egressd; do
    test -x "/usr/libexec/cybou/$program" || not_run "$program is not deployed"
done

CAPSULE="$(cat /proc/sys/kernel/random/uuid)"
WORKSPACE="$(mktemp -d)"
trap 'rm -rf "$WORKSPACE"' EXIT
chmod 700 "$WORKSPACE"

cargo build --quiet --locked -p cybou-agentd
AGENTD="$CARGO_TARGET_DIR/debug/cybou-agentd"

# A capsule that proves it has what the lease granted and nothing else: its bearer file is readable
# and its workspace is writable. It does not spend the model budget — that is the live provider
# gate's job, and this one must pass on a host whose provider is configured but idle.
"$AGENTD" launch \
    --profile sandboxed-autonomous --agent gate --workspace "$WORKSPACE" \
    --memory-mib 512 --cpus 1 --tasks-max 64 --lifetime-seconds 120 \
    --token-limit 1000 --max-output-tokens 32 --sensitivity 1 \
    --model Strong --spend-limit 0 --may-execute --capsule-id "$CAPSULE" \
    -- /bin/sh -c 'test -s /run/cybou/model-token && : > /workspace/reached'

test -f "$WORKSPACE/reached" || {
    echo "the capsule did not run inside its granted workspace" >&2
    exit 1
}

# Ending is not asking, and it is also not leaving. Every one of these outliving a session is a
# surface an agent could still be talking to, or a record of authority nobody owns.
for leftover in \
    "/run/cybou-agent-leases/$CAPSULE.lease" \
    "/run/cybou-agent-leases/$CAPSULE.env" \
    "/run/cybou-session-$CAPSULE"; do
    test ! -e "$leftover" || {
        echo "teardown left $leftover behind" >&2
        exit 1
    }
done

for unit in "cybou-agent-gateway@$CAPSULE.service"; do
    ! systemctl is-active --quiet "$unit" || {
        echo "$unit is still running after teardown" >&2
        exit 1
    }
done
for unit in "cybou-capsule-$CAPSULE.service" "cybou-egress-$CAPSULE.service"; do
    ! systemctl --user is-active --quiet "$unit" || {
        echo "$unit is still running after teardown" >&2
        exit 1
    }
done

echo "=== Agent launch gate passed: one session came up and left nothing behind ==="
