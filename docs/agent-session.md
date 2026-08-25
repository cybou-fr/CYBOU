<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# The agent session owner

Every part of an agent session existed before this component and none of it was owned. A capsule
could be compiled and started, a lease could be minted, a model gateway could be run, an agent pack
could be installed — and each was reached by a different caller. That is not a tidiness problem. It
is how the same launch came to be described twice: the per-capsule gateway rebuilt its own lease from
environment values, with its own hardcoded budget, so a launch file and a running capsule could each
be internally valid and still describe different permissions. Nothing downstream could say which of
the two a person had approved.

`cybou-agentd` is the single owner. One selection becomes one lease, and every runtime name is
derived from that one object:

```text
              one selection
                    │
                issue_lease            ← the one public mint
                    │
                  Lease
                    │
   ┌────────────────┼────────────────┬──────────────────┐
   ▼                ▼                ▼                  ▼
capsule spec    lease file       model token        the clock
(compile)       (gateway reads)  (issued against)   (ends both)
```

## It owns the session; it does not enforce it

A coordinator is exactly the shape of component that quietly becomes a boundary, so this is stated
rather than assumed:

```text
what ends the capsule    the kernel, through RuntimeMaxSec on its transient unit
what ends the model      the lease clock, checked at the gateway on every request
what cybou-agentd does   works it out, writes it down, starts it, tears down what is left
```

If the owner dies mid-session the capsule still ends at its deadline and the gateway still refuses
once the lease is over. Session state is a *report* — it says which of those happened, so a surface
can tell a person "you stopped it" rather than "your time ran out". Nothing consults it before an
agent acts, and it cannot be consulted to find out whether something is permitted.

## What one launch implies

`cybou-agentd plan` mints the lease and prints every file, unit and teardown step that launch would
produce, without touching a filesystem or a service manager:

```bash
cybou-agentd plan --profile sandboxed-autonomous --agent opencode --workspace /srv/project --memory-mib 4096 --cpus 2 --tasks-max 512 --lifetime-seconds 14400 --token-limit 200000 --max-output-tokens 4096 --sensitivity 1 --model Strong --spend-limit 0 --host github.com --may-execute
```

Two properties of that output are the point, and `scripts/test-agent-session-gate.sh` asserts both.

**Every name is the capsule's own identity.** The gateway instance, the capsule unit, the lease file,
the launch file and the runtime directory all carry the same UUID, so a pair of units in a service
manager's list can be matched back to one session.

**The launch file carries nothing that is authority.** It defines the task id and the per-token
ceilings, and no capsule id, workspace, lifetime, model class or spending ceiling — those are the
lease, and the lease already says them. Each of those names, written into a launch file, could
disagree with the approved grant.

## Teardown has one safe order

```text
1. stop the capsule       the untrusted party loses its hands first
2. stop the gateway
3. remove the launch file
4. remove the lease file  the record outlives the things it granted
```

Stopping the gateway first would leave a running agent making requests against a socket that has
disappeared, which it experiences as a refusal it can retry rather than as an ending. *Ending is not
asking*, so the thing that can ask is the thing that stops first.

## What is not built yet

Carrying the plan out. `plan` derives and refuses; it does not write the files, start the units or
hold the session. That is deliberate rather than pending polish: a coordinator that starts part of a
session and cannot end it is worse than one that has not started yet.

The open question it depends on is a privilege boundary, not an implementation detail.
`cybou-agent-gateway@.service` is a *system* unit precisely so the LiteLLM master key can stay
root-only and reach it through `LoadCredential`. An unprivileged owner therefore cannot start it
without either a narrow polkit rule scoped to that one template, or a typed request across the
boundary in the shape `Action1` already uses for `Executor1`. Choosing between those decides what the
owner is, so it is being chosen rather than defaulted into by whichever is easier to write.

Until then, `scripts/test-agent-gateway-gate.sh` starts a gateway directly against a fake LiteLLM
peer, and [Per-capsule model gateway deployment](agent-gateway-deployment.md) documents the file
contract the owner will produce.
