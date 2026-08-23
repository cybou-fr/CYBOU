<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Reboot continuity

## The claim

Identity, accepted biography and open commitments survive a restart of every Mind process. A new
session is a new session of the same subject, not a new subject.

## Why it constrains today

Continuity is the difference between a system that remembers and a system that starts again
convincingly. Every organ except the Journal holds derived state and rebuilds it, so the test worth
running is not that a process comes back — it is that what comes back is the same subject with the
same obligations still open.

The failure this catches is subtle: a projection that rebuilds correctly but a session counter that
restarts, or an open commitment quietly closed by a rebuild, both leave a system that looks healthy
and has lost the thing it exists to keep.

## The evidence

The multi-daemon gate starts every owner, records an intention, restarts processes, and checks that
the identity is the same one and the obligation is still open:

```bash
bash scripts/test-multi-daemon-integration.sh
```

Persistence and recovery of the lifecycle and identity state have their own unit coverage:

```bash
cargo test -p cybou-identityd -p cybou-lifecycled --locked
```

## What this does not prove

Continuity across a machine reboot, as opposed to a restart of the processes. The units are
`WantedBy=default.target` and the gate does not power-cycle a host.
