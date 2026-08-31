<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0048: A person may ask for an action, and the asking is the confirmation

## Status

Proposed

Extends [ADR-0022](ADR-0022-authorized-action-boundary.md). Nothing in it is reversed.

## Context

`Action1` has one entrance: `EvaluateInsight`, which takes a finding the host reached about itself.
Every proposal in this system is therefore Mind's, made from readings Mind gathered. That is the
right shape for autonomous remediation and it is the whole of what exists.

It leaves the desktop with buttons that cannot work. `SystemHub` has fourteen methods that answer
`Err(Refused)` — restart a service, signal a process, apply an update, add a key, take a backup —
and the panels for all of them are drawn, reachable from the Dock, and complete except for the part
where anything happens. The refusal is honest and the comment above it is accurate: *privileged
execution requires an Action1 proposal*. There is simply no way for a person to make one.

So an operator who can see that `nginx.service` has failed, on a surface built to show them exactly
that, must leave and use `systemctl`. The desktop is a window onto a machine it cannot touch.

### What the missing piece actually is

Not a new capability. `Operation::RestartService` is already in the closed operation table,
`ExecutableAction::ServiceRestart` already has an adapter, and both are proven end to end by
`test-action-gate.sh` and `test-confirmation-gate.sh`. The kernel-facing half is built and running.

What is missing is a proposer. `Proposer` has two variants — `Mind` and `Agent` — and a person is
neither.

## Decision

**A person holding an authenticated seat may ask `Action1` for an operation, and their asking is
the confirmation that operation would otherwise wait for.**

### The asking is the confirmation

[ADR-0022](ADR-0022-authorized-action-boundary.md) separates operations that a standing policy
pre-authorizes from operations that require explicit confirmation. A person's request is already
explicit: they were looking at the panel, they read the unit name, and they pressed the button. To
decide `RequiresUserConfirmation` and then ask them again is to ask the same person the same
question twice, and a system that does that teaches them to click through it.

So a request from a person that passes criticism is decided
`GrantedOnConfirmation { confirmed_by: <seat> }` — the verdict added for the other half of this,
which exists precisely so that *a person allowed this* and *a policy allowed this* are never the
same record.

### Everything else still applies, and that is the point

- **The operation table is unchanged.** Critical operations — deleting a service's data,
  formatting a filesystem, powering off — are never offered and are refused if something else
  builds them. A person asking does not make them askable. There is no answer a person could give
  that would make those safe, which is the same reason they are not offered to Mind.
- **Criticism still runs**, and a failed critic still refuses. What a critic cannot do for a
  person's request is check it against evidence, because there is none — which is the next point.
- **The executor is unchanged.** It receives an opaque permit identity, claims it from `Action1`,
  and performs the action stored there. Nothing about this path lets a caller name an operation.

### A person brings no evidence, and the record says so

`Proposer::brings_its_own_evidence` is true only for `Mind`, and not because Mind is clever: a
proposal from Mind carries a finding, and a finding carries the readings behind it. A person's
request carries a name they typed and nothing else.

`Proposer::Person` therefore answers false, alongside `Agent`. The two are not equivalent in trust —
one is somebody who authenticated to this host, the other is a party inside a capsule — but they
are equivalent in this: what they ask for cannot be checked against anything the host observed. A
record of a person's request must never be readable as though the host had concluded something.

### The gateway carries a permit and never an operation

A granted request produces a permit, and something has to present it to the executor. For
remediation that is `cybou-remediationd`. For a person's request it is the gateway, which is where
the request came from.

This does not make the gateway an authority, and the reason is the shape ADR-0022 already chose:
the executor accepts a permit identity and nothing else, so the gateway can present one and cannot
name what it is for. It is a courier. What it supplies to `Action1` is the same thing it supplies
for a confirmation — which account is asking — and that is the one fact neither end can establish
for itself.

### Not every refused button becomes a working one

This opens the operations that already exist. `SystemHub`'s other refusals — signalling a process,
applying updates, adding an SSH key, taking a backup, formatting — stay refused, because each needs
an entry in the operation table, an adapter in the executor, and a decision about its risk and
reversibility that is that operation's own. Opening the door does not furnish the room.

## Consequences

### Positive

- The desktop can act on the host it is a window onto, for the operations the boundary already
  carries.
- The path is the one already built and gated, rather than a second way to reach the Body.
- A person's action and a policy's action are distinguishable in the Journal for the first time,
  which is what makes *why did nginx restart on the fourteenth* answerable with a name.

### Negative

- **An authenticated session becomes able to restart services.** That is the decision, not a side
  effect. It is bounded by the operation table rather than by hoping nobody finds the endpoint, and
  it is the same authority a person with that account already has over `systemctl` — but it is now
  reachable from a browser, and every weakness in session handling inherits the difference.
- **A request carries no evidence, so criticism is thinner for it than for a finding.** The critics
  that compare a proposal against its readings have nothing to compare. What remains is the
  operation table, which is a smaller check than the one Mind's proposals get.

### Neutral

- No new operation, adapter, or Body capability. If this ADR is wrong, what it costs is an entrance
  rather than an architecture.

## Open

- Whether a person's request should be rate-limited per seat. Nothing here stops somebody holding
  the button, and the executor is single-use per permit rather than per second.
- Whether the operation table should distinguish what a person may ask for from what a policy may
  pre-authorize. They are currently one list, and the argument for splitting it is that a person
  asking is present to see the result while a standing policy is not.
