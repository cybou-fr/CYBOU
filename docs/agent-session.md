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

`scripts/test-capsule-launch-gate.sh` runs a whole launch with no model in it. That case needs no
provider, no credential and no gateway, which is why it is the part of `launch` provable on an
ordinary host — and it is the case that says an Agent Capsule is a bounded place to compute rather
than a container that only exists around a model. For a while it was not merely untested but
impossible, because planning refused a lease with no model grant outright, and nothing noticed
because nothing ran a launch.

`scripts/test-agent-launch-gate.sh` covers the other half, the one that needs a deployed gateway and a
configured provider, and exits `3` — `NOT RUN`, never passed — where those are absent.

Running the first of those found something small and corrosive. A capsule run to completion has
already exited and been collected, so asking systemd to stop it fails; teardown believed that exit
code, and every clean session ended by printing a teardown error. Output a person is meant to ignore
on every good run is output they will ignore on the run that mattered. Teardown now asks the host
whether the unit is running rather than believing what stopping returned, and asks before it tells, so
a session that finished on its own is torn down in silence.

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

### The credential boundary, and a hole that was in it

That gate was owed, and it found something. `LoadCredential=` is read by the service manager, which
is root, and on systemd 257 it **follows symlinks**: a credential whose source is a symlink to a
root-only `0600` file delivers that file's contents to a service running as `nobody`. Measured, not
assumed — `scripts/test-credential-boundary-gate.sh` demonstrates it, and records the observation
whichever way it comes out, because the answer is a property of a version.

The gateway loaded its lease that way, out of `/run/cybou-agent-leases`, which is owned by the
unprivileged user that writes leases into it. Together those two facts meant `cybou` could put a
symlink where its lease goes, name any root-only file, and have root read it out into a process it
controls — the proxy master key among the reachable targets, which is the one secret this whole
arrangement exists to keep out of `cybou`'s reach. Neither the cybou-owned directory nor the polkit
rule that lets `cybou` start the unit is enough alone; the two combine.

The lease is no longer a credential. The gateway is told a path and opens it itself, which is the same
user reading a file it wrote: nothing is crossed and nothing escalates. It refuses a symlink there
anyway, and that refusal is not a boundary — it is the process declining to treat something other than
the file the owner wrote as that file.

The master key stays a credential, because its source lives in `/etc/cybou`, which is root-owned and
where `cybou` cannot put a symlink. The rule the gate now enforces is the general form: nothing root
reads may come from a path an unprivileged user can replace.

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

The gateway now publishes it. Every couple of seconds, and only when something changed, it writes a
`ModelUsageSnapshot` beside its socket — what has been charged, tokens, completions, and the instant
it looked — replaced atomically so a reader never catches half a figure. The session owner reads that
file and the listing stops saying unknown.

The instant travels the whole way onto the card, and that is the point rather than a detail. *Has
spent €0.42* and *had spent €0.42 when somebody last looked* are different claims, and only the
second is true of anything read out of a snapshot; a surface presenting the first would quietly
become wrong every time a completion happened between two readings. It also makes a stale figure
legible — a reading from ten minutes ago beside a session that is plainly working is worth seeing,
and a bare integer could not say it.

An unreadable or absent snapshot leaves the previous figure standing. A gateway that has not written
yet is not a session that has spent nothing, and replacing a real figure with a nought because a read
failed would be the same lie in a new place.

The file is not given to the capsule. An agent has no business reading its own ledger: an agent
reporting its own consumption is the executor grading its own homework, and this figure exists
precisely so nobody has to ask it.

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

`scripts/test-agent-runtime-gate.sh` runs one. It writes a session's two files the way a launch
would, starts a unit named the way a capsule's is, and then asks the owner over D-Bus what is
running — so the bus name, the launch directory, the service-manager question, the published ledger
and the teardown are exercised rather than reasoned about. Everything above had been checked without
a host until then, and code that is only ever right on paper accumulates until the first real host is
a bad place to discover which part of it was wrong.

`cybou-agentd sessions` and `cybou-agentd stop` are clients of that surface, not second readers of
the host. Two things walking the launch directory would be two answers to *what is running*, and the
one that is not the owner would be wrong the moment a session started between its listing and its
reading. Stopping goes through the owner for a stronger reason: the owner is what records *why* a
session ended, and units stopped behind its back would leave an agent that was stopped looking
exactly like one that finished.

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

### Two doors, and only one of them is safe to expose

`launch` takes ceilings as arguments. That is right for bring-up on a host somebody is sitting at:
whoever can run it is already `cybou`. It is wrong for anything reachable, because a bus method or a
web endpoint carrying the same shape would be asking its caller to invent a `CapsuleGrant`.

`start` is the other door. A caller names a profile, an agent, a workspace and one of the models that
profile offers; every bound comes from `/etc/cybou/agent-profiles.json`, which only root can write.
Deployment creates it empty, so a host offers nothing until an operator writes something — fail-closed
by construction rather than by a flag.

Three things a caller could otherwise have widened, and the last two are easy to miss:

**The workspace** is the one directory an agent may change, and a caller supplying it freely could
supply `/etc`. A profile carries the roots a workspace may live under, and the check is lexical, so
`/projects/../etc` is inside `/projects` by spelling and outside it by meaning — which is the one that
decides. A profile naming no roots permits no workspace, because reading an absent field as *anywhere*
would make the most permissive configuration the one an operator gets by leaving something out.

**The agent.** Ceilings are approved for a pack, not in the abstract, so a profile runs only the
agents it names.

**The model.** A caller picks a class from what the profile offers and supplies no bound that goes
with one: the spending policy, the token ceilings and the sensitivity are attached to the class the
operator approved. Choosing *Free* cannot arrive with a ceiling of a hundred beside it, and no caller
decides how exposing a prompt its agent may send — sensitivity is what governs which routes may see it
at all.

None of this is authorization. It decides what a named profile permits and would answer the same for
anyone; whether a particular caller may use a particular profile is a different question, and one this
layer would answer badly by guessing.

`scripts/test-agent-profile-gate.sh` runs `start` against a catalogue on disk, because the parsing,
the lookup and the lexical path check all sit between a caller and a grant and none of them had ever
read a real file. Running it found that a profile offering no model could not be launched at all: the
launch path still demanded token ceilings, which bound a bearer that a model-free session does not
have. So the profiles needing a model least were the ones that could not be used, and the ceilings
are now asked for only when there will be something for them to bound.
