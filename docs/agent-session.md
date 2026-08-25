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
3. stop the egress broker
4. remove the broker socket
5. remove the launch file
6. remove the lease file  the record outlives the things it granted
```

Stopping the gateway first would leave a running agent making requests against a socket that has
disappeared, which it experiences as a refusal it can retry rather than as an ending. *Ending is not
asking*, so the thing that can ask is the thing that stops first.

## Carrying it out

`cybou-agentd launch <selection> -- <program>` runs the same plan. It writes the lease and then the
launch file, starts this capsule's egress broker if any network was granted, starts the gateway,
waits for the socket and bearer to exist, runs the program inside the capsule under its cgroup, and
then tears the session down in the order above — including when something failed on the way up. A
launch that gave up halfway and returned would leave a live gateway holding a bearer for a session
nobody is watching.

The broker and the capsule are transient *user* units rather than children of this process. A way out
that lives inside the coordinator survives exactly as long as the coordinator does, and outlives it
in the worse direction if the coordinator is killed and the broker is not.

`scripts/test-agent-launch-gate.sh` proves the whole of it on a deployed host and checks the part
that is easy to get wrong: that nothing is left. It exits `3` — `NOT RUN`, never passed — on a host
with no gateway template, no configured provider, or no user service manager.

## Asking the agent rather than running a program

`--prompt TEXT` instead of `-- <program>` drives the session's agent over ACP: the pack's own
entrypoint is started inside the capsule, and Cybou runs `initialize`, `session/new` and
`session/prompt` against it. The credential-free agent configuration is written into
`<workspace>/.cybou/`, where the agent looks for it; it names a token *file* and never a token, and
there is no provider credential in this process to write even by mistake.

The turn's deadline is what remains of the lease. A constant here would be a second clock beside the
one a person granted.

This is why a capsule's standard streams are connected to whoever started it rather than to the
journal. For a program that is a convenience; for an agent it is the whole channel — stdio *is* the
protocol, and a client talking to a closed pipe while the agent's half accumulates in a log is not a
subtle failure, it is every failure at once.

A program or a prompt, never both. They are two different claims about what the capsule is for.

## The one privileged step

### When to leave it

The trigger is not the number of verbs. It is any of: more than one OS user or tenant on the host;
arbitrary unit templates rather than one; root-side filesystem mutation; network configuration;
credential creation; or a profile authorization that has to be enforced above the `cybou` UID. Any of
those means the delegation has stopped being "start a process that already runs as this user", and
the typed boundary is then the smaller thing to build.

One adversarial gate is still owed before calling this production-ready: a malicious lease pathname
or symlink at `/run/cybou-agent-leases/<instance>.lease` must not be able to make the root-side
`LoadCredential=` expose an unrelated root-owned file. systemd ignores symlinks for directory
credential sources, but a single-file source deserves the test rather than the assumption.

`cybou-agent-gateway@.service` is a *system* unit for exactly one reason: the LiteLLM master key stays
root-owned and reaches the unprivileged gateway through `LoadCredential` rather than sitting in a file
the `cybou` user can read. The gateway process itself already runs as `cybou` and holds no privilege
— the credential is the whole of the difference.

So the session owner, which is unprivileged, needs permission to start and stop that one template.
Deployment installs `debian/cybou-agent-gateway.rules`, which grants exactly that: two verbs, one unit
name pattern matched by shape rather than by prefix, one user. Not reload, not enable, not mask, and
no unit outside the pattern — those would let a session owner reshape the host rather than start the
surface its own lease already describes.

The alternative was a typed request across the boundary, in the shape `Action1` already uses for
`Executor1`. It is the better long-term answer and it is a larger one: a new organ with its own bus
policy, for a delegation whose whole content is "start a process that already runs as this user". The
polkit rule was chosen because it is narrower than what the executor already permits, and because the
authorization is legible in one file rather than distributed across a protocol. If agent launches ever
need to do more than start and stop one template, that is the signal to move to the typed boundary
rather than to widen this rule.

## What a person would see

`launch` prints one line of JSON when the session comes up and again when it ends: the agent, the
profile, the workspace, how long it has run and how long the lease has left, the model class and what
has been spent against it, the ceilings, the exact hosts, and every unit the session put on the host
so any of them can be looked up by name.

Two things about it are deliberate.

**It shows what was granted, not what is being used.** `4096` MiB on that line is the ceiling a person
selected and the kernel enforces; it is not a reading of what the capsule currently occupies. Cybou
can observe the latter — that is what the telemetry layer is for — and until that observation is
actually pointed at a capsule's cgroup, printing a number that *looked* like usage would be inventing
the one thing a person is watching for.

**Whoever reports a spend has to hold the ledger.** The model gateway is a *different process*: it
receives the lease as bytes and charges its own copy, so the lease the launch path holds is the grant
and not the ledger — identical in everything a person selected, and permanently at nought in what has
been spent. So the figure does not arrive on a lease at all. It arrives as a `Ledger`, and a reporter
that has none must say `Elsewhere`, which shows as *unknown* rather than as nought.

That distinction is not theoretical. The first version of this module took a lease and read a spend
off it; the invariant was right and the test stating it was right, and one line of wiring handed it
the copy that could only ever say zero. Unifying the authority a launch is minted from did not
unify the mutable ledger, and the type now makes the difference impossible to paper over.

Making it *available* is a separate, unfinished piece: the gateway has the number and nothing asks
it for one. Until a session owner can, every launch-side view says unknown.

Gathering it in one place is the point: the lease knows what was granted, the plan knows which units
carry it, and the session knows what has happened, so a surface that reached into all three would be
deciding for itself which facts belong together — and a second surface would decide differently.

## What is not built yet

`Pause`. `Stop` from outside — today a session ends when its program ends, its lease expires, or the
owner is killed, and there is no second process that can withdraw a running lease. Both belong with
B11's quarantine and revoke rather than here.

More than one turn. `--prompt` asks once and the session ends; a working agent is a conversation, and
holding one open means keeping the ACP connection alive across prompts and streaming its updates
somewhere a person can watch. That is B8, and the seam for it already exists — every `session/update`
is kept whole rather than projected into a Cybou vocabulary.

Agents other than OpenCode. `--prompt` refuses any agent this build has no pack for, rather than
guessing at an entrypoint.

A surface to draw the session on. The projection exists and is printed; what is missing between it
and a card in the browser is a way to ask a *running* session for it. `cybou-agentd` is not yet a
daemon despite its name — `launch` owns one session and exits with it, so there is no bus name for a
web gateway to call and no registry of what is running. That is the next piece, and the card follows
it rather than the other way round: a card fed by anything other than the session's own owner would
be a second assembly of the same facts.

That daemon is also where the spend becomes knowable. It needs a typed usage snapshot from the model
gateway rather than another copy of the lease, because copying a mutable ledger between processes is
what produced the defect above.

### The daemon

`cybou-agentd serve` holds what is running and answers for it on `org.cybou.Runtime.Agent1`.

`Runtime`, not `Mind`, and the unit is deliberately not `PartOf=cybou-mind.target`. An agent runtime
is not part of what Cybou *is*: it starts, holds and ends capsules containing software Cybou did not
write and does not trust, which is the opposite of an organ owning a piece of Mind. A name under
`org.cybou.Mind.` would say the reverse in the one place an operator looks, and binding the unit to
Mind's target would mean stopping Mind stops the thing watching those capsules.

It starts by reading the host. Every `<uuid>.lease` with an `<uuid>.env` beside it is read back —
half a launch is not a session, because its ceilings were never written and inventing them would put
bounds on a bearer somebody approved with different ones — and each is re-derived through the same
`plan()` a launch used. Whether the capsule is still up is asked of the service manager. What is
still running is held; what is not has its leftovers cleared before anything is served, so a listing
never shows a session whose capsule is gone and never leaves a gateway holding a bearer for one.

It ends nothing on the way out. A capsule outlives this process on purpose, and tearing down every
session because the owner was restarted would make the coordinator into the boundary that ADR-0042
says it must not be.

### Read and stop, deliberately not launch

The surface offers `Sessions`, `Session` and `Stop`. It does not offer `Launch`, and that is a
decision about who may ask for a capsule rather than a gap.

A CLI launch is bounded by who can run it: whoever invokes `cybou-agentd launch` is already `cybou`
on this host. Putting `Launch` on the bus removes that bound — any process under the same UID could
then ask for a capsule, and the only thing left between such a request and a real grant would be the
profile it names. That is one of the conditions under which the polkit delegation below should become
a typed boundary, and it is not something to walk into by adding a method.

So `Launch` arrives together with a registry of operator-approved profiles the owner reads *itself*,
and takes a profile id rather than a set of ceilings. `Sessions`, `Session` and `Stop` are safe in a
way it is not: none of them can widen anything, and stopping removes authority rather than granting
it.

`Stop` runs the teardown. It sends nothing to the agent and waits for no agreement — the capsule is a
cgroup with a kill switch. The reason is recorded *before* the teardown, because a session torn down
first and labelled afterwards could be marked expired if the clock ran out in between, replacing a
person's decision with a timer. A stopped session then leaves the registry, because the registry
answers what is running; a listing of finished sessions a person can still read is not built.

It must also survive its own restart, and the part of that which is a judgement rather than plumbing
is now built and tested.

A capsule and a gateway are deliberately built to outlive the coordinator up to their hard deadlines
— that is what makes the boundary hold without one being alive. So an owner that came back and
reported no running agents while OpenCode was still working would be wrong about the host in the
direction that matters most: a working agent, in a capsule nobody is watching, with no surface
offering a person a way to stop it.

The registry is therefore not a memory of what a process started. It is a reading of the host.
Everything needed is already written where the session put it — the lease carries the whole grant and
the launch file carries the task and its ceilings — and recovery re-derives each session through the
same `plan()` a launch used, so a recovered session and a fresh one are the same object rather than
two descriptions that agree today.

One judgement is made and it is a small one. Whether a capsule is still up is asked of the service
manager, never inferred from a file: a file says what a launch intended, a running unit says what is
true now. And if the capsule is gone, the session is over — but *why* it ended was known only to the
owner that died with it, so recovery attributes nothing. It does not resurrect the session, does not
invent a verdict, and does not report an agent that finished as one somebody stopped. It hands back
the plan so the leftovers can be cleared, which is the one thing still worth doing about a session
nobody can describe. A lease that simply ran out needs no judgement at all: the clock says so, and it
is the same clock that ended the capsule's own unit.

What cannot be read back is reported rather than skipped. A launch file this build cannot re-derive
is either a defect or a session from another version, and both are things an operator should be told
about instead of having them quietly stop existing in a list of what is running.

The launch flags are a bring-up interface and should not become the web one. A browser should send a
profile, an agent, a workspace and a model class; the owner reads the ceilings, the lifetime and the
network allowance out of the operator-approved profile. An endpoint that accepted memory, CPUs, hosts
and lifetime from the caller would be asking the browser to invent its own `CapsuleGrant`.
