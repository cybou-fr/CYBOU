<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Troubleshooting

## Living Canvas connection errors

If Living Canvas reports connection errors or failed SSE/WebSocket streams:
1. Verify `cybou-web-gateway.service` is active (`systemctl --user status cybou-web-gateway`).
2. Check that the port (default 8080) is listening on localhost (`ss -tulpn | grep 8080`).
3. Ensure user credentials and PAM authentication via `cybou-authd` succeed.

## D-Bus Daemon and Journal issues

If daemon states do not update:
1. Check Journal permissions under `~/.local/share/cybou/journal.db`.
2. Inspect daemon logs with `journalctl --user -u 'cybou-*' -f`.
3. Verify that `cybou-eventd` is running and accepting Event1 transactions.

## Repairing Living Canvas state

1. Clear browser local storage / cache if schema migrations encounter stale local state.
2. Cognitive state and Journal history remain persistent in `~/.local/share/cybou/journal.db` and are unaffected by browser refreshes.
