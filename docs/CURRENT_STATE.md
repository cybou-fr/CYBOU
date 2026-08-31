<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Current State

What exists now. Not what was built, and not when — `git log` holds that, and holds it better than
prose does.

This document is the implementation authority: where it disagrees with an aspirational description
elsewhere, this is what the code does. Where it disagrees with an **Accepted** ADR, the ADR outranks
it and the implementation is wrong.

Claims here that need defending have an [evidence document](evidence/README.md) naming the command
that re-checks them. Claims with nothing behind them are in [What is not
built](#what-is-not-built), stated as absences rather than omitted.

## Deployment

A VPS, a server, a VM or a container running Debian 13, reached through a browser over HTTPS
([ADR-0041](adr/ADR-0041-server-first-deployment.md)). A personal workstation is a supported place to
run it and is not what it is for.

Debian 13 is the integration authority: the daemons need a session bus and systemd user units, so
the multi-daemon gate and every deployment run there. The portable half — protocol, storage,
meaning, the frontend — builds and tests anywhere.

Nothing in the cognitive substrate loads a model, needs an accelerator, or requires a network.

## Mind owners

There are fifteen Mind owners: fifteen user-session processes, each owning one versioned D-Bus
interface under `org.cybou.Mind.`.

One process in the table below owns no interface. `cybou-remediationd` is listed with them because it
runs beside them and is governed by the same layering, but it offers nothing to anybody: it reads what
the telemetry organ concluded, asks `Action1` what may be done, and reports back what happened. A bus
name would be a second place to ask about actions when `Action1` already answers for the lifecycle.

| Owner | Interface | Owns |
|---|---|---|
| `cybou-eventd` | `Event1` | the canonical Journal; the only writer |
| `cybou-identityd` | `Identity1` | subject continuity across sessions |
| `cybou-healthd` | `Health1` | capability states and the dependency graph |
| `cybou-intentiond` | `Intention1` | open obligations and commitments |
| `cybou-predictord` | `Predictor1` | statistical prediction and calibration |
| `cybou-perceptiond` | `Perception1` | stable observations about the host |
| `cybou-telemetryd` | `Telemetry1` | bounded transient Body state and system insight |
| `cybou-epistemicd` | `Epistemic1` | what is known, and with what epistemic force |
| `cybou-contextd` | `Context1` | associative context and bounded activation |
| `cybou-workspaced` | `Workspace1` | attention coalitions and admission |
| `cybou-meaningd` | `Meaning1` | the meaning boundary |
| `cybou-lifecycled` | `Lifecycle1` | sleep, wake and consolidation |
| `cybou-selfd` | `Self1` | autobiographical self-assessment |
| `cybou-presenced` | `Presence1` | the presentation-ready projection |
| `cybou-actiond` | `Action1` | proposals, criticism, policy decisions and permits; no Body adapter |
| `cybou-remediationd` | none | takes a finding as far as an outcome; owns no interface because it offers nothing |

Beside them, and deliberately not among them:

| | |
|---|---|
| `cybou-model-brokerd` | `org.cybou.Faculty.ModelBroker1` — a faculty, owning no part of Mind |
| `cybou-web-gateway` | the HTTP boundary; not a Mind owner and holds no cognitive state |
| `cybou-agentd` | `org.cybou.Runtime.Agent1` — owns bounded agent launches, live sessions, teardown and recent final views; not Mind |

`cybou-shelld` is a library with no binary. Its unit exists, describes a process that does not run,
and is excluded from the deploy and from `cybou-mind.target`.

Every organ is a separate process that fails separately, so a silent organ is a gap on a page rather
than an outage. A projection that could not be read reports *unknown*, never *empty*.

**Layering.** `telemetry → journal → epistemic → associative → attention → meaning → governance`.
A layer may read the one above it and may not overrule it. A faculty may depend on no organ in either
direction. Both are checked at the manifests by `scripts/validate-organ-layering.py`.

## The Journal

One writer. Append-only, hash-chained, schema v3, with v2 still readable.

- A contribution's canonical form is pinned to byte fixtures, not to a round trip. See
  [journal compatibility](evidence/journal-compatibility.md).
- Erasure removes a payload and raises an epoch; every derived projection discards and rebuilds. See
  [erasure gate](evidence/erasure-gate.md).
- A contribution whose privacy is weaker than something it references is rejected.
- Terminal outcomes are typed: an attempt that did not finish is not an outcome.

## Body observation

`Perception1` records what is stable about the machine — kernel, hostname, memory size — and stops
there.

`Telemetry1` watches what a host is doing: load, memory and swap use, memory/IO/CPU pressure, root
filesystem bytes **and inodes**, open file descriptors against the system limit, failed units.
Windows are bounded twice, by span and by count.

Those subjects are readable on any Linux host with no configuration. A subject that needs to be told
*which one* is a different kind of thing, and is declared rather than discovered — a probe that went
looking for certificates would decide for the operator what is worth watching.

Declarations live one per line in `telemetry.watch` under the configuration directory. **A line this
build cannot read is an error, not a comment**: an operator who mistypes `certificate` has told their
machine to watch one, and a skipped line means they believe it is watched, nothing is, and the first
they hear of it is an expired certificate. A refused file is announced with every bad line and its
number, and the universal subjects keep running — a mistake in an optional file must not remove
watching that was never in question.

Three subjects are declared rather than universal: `certificate.days.remaining`, `service.active`
and `backup.age.days`. One window per declared thing, one finding per thing rather than one naming a
count — an operator with four certificates needs to know which.

**What a measurement is about travels as one key, not two fields.** `MetricKey` is a subject and,
for a declared thing, which one. It is what the windows are keyed by, what a deviation is keyed by,
what a finding cites as its evidence, what a projection names, and what a remediation proposal reads
its target from. The two halves used to travel separately and were repeatedly dropped apart: two
certificates produced two windows, two findings and *one* deviation, because the map holding the
deviations had nowhere to put the name — so whichever finding was built second cited the other
certificate's readings as the evidence for itself. A proposal about a declared service now names the
unit it would act on, taken from the finding, rather than the `systemd:<unit>` placeholder a
proposal falls back to when it genuinely does not know which one it means. The count of failed units
still falls back, because it genuinely does not.

**A projection from readings that stopped is not a projection.** The arrival time is counted down
from now, which is correct and is the whole reason it is measured from now rather than from the last
reading — but a window nobody has fed for an hour counts down past zero, is clamped there, and
reports that the threshold is reached *now*. A stopped probe produced the most alarming answer
available from evidence that stopped an hour ago: a disk nobody was measuring, reported as full.

Past the staleness bound the answer is `ReadingsStopped`, carrying when the last reading arrived.
The value is still shown, because it is a fact; what is refused is the rate. The card draws no
heading for it, since the staleness is already said on its own line — a heading would be the one
place on the page still claiming a live trend for a metric nobody is reading. Inside the bound the
clamp still applies: a sampler that missed a tick is still measuring, and an arrival that has just
passed is honestly *now*.

This was found by asking what the projection does for a window the watched states already call
stale — the two were built a commit apart and never asked about each other.

**The slope is taken from the readings, and taken again only when the readings change.** Theil–Sen
compares every pair, so the work is quadratic; a six-hour window at one sample every ten seconds
holds 2160 points, and a host watching a dozen subjects would spend four seconds per page load
answering *why is this server busy* — which would be a reason the server is busy. Two bounds hold
it. The slope is estimated from at most 128 points sampled evenly across the window, which the
detector still sees whole, and the newest point is always kept because it is the one the arrival
time is measured from. And the estimate is held per window against a revision counter, so a person
refreshing a page does not pay for a slope that has not moved: a second projection of an unchanged
host costs over four hundred times less than the first.

What is held is the estimate and never the projection. A held projection would count down from an
instant that is receding, which is an error this module already removed once — and it would come
back invisibly, as a number that simply stopped moving.

**The instance reaches the reader, and a target the proposal did not know does not pretend to be
one.** The path from a window to a screen was correct up to the last inch and then dropped it: the
gateway held which certificate a finding was about and did not put it on the wire, so a host
watching four of them drew four rows reading *a watched certificate is close to expiry, or past it*
and nothing else. And `OfferProjection` carried the target from the day it existed while the card
never drew it, so a proposal naming a real unit and one naming `systemd:<unit>` looked identical.
Both are on screen now, and the placeholder is drawn as an absence rather than literally — rendered
as written it reads as a real name badly formatted, which is the opposite of this host admitting it
does not know which unit it means. Both decisions live in `heading.rs`, where the native test run
reaches them.

**A finding shows its reading whether or not there is a baseline behind it.** The two detectors ask
different questions: *is this value a problem*, which needs one reading, and *is this value unusual
here*, which needs a window. Evidence carried only the second, so on a fresh host the first cited
nothing at all — a filesystem at 97% produced a `StorageExhaustion` of `Strong` strength with an
empty `because`, which is precisely the shape this build calls indistinguishable from something a
model made up. `InsightEvidence` now carries the observation, with the deviation as the optional
half, and the absent baseline is said in the prose and on the card rather than filled in with a zero
that would claim the reading is enormously far from a normal nobody established.

That emptiness had a second consequence, and it is the reason this was a defect rather than a gap.
The outcome layer checked that every measure behind a finding was readable again with `all()`, and
`all()` over an empty list is *true* — so an action against a baseline-free finding was reported as
having worked whatever happened to the readings afterwards. On a fresh VPS, which is every VPS for
its first six hours, that was the ordinary path. It is fixed at the source and refused again at the
outcome, because a vacuous truth that says *it worked* must not be one upstream change away from
returning.

**What an agent may do is decided before anything can create a capsule.** `cybou-capsule` holds the
grant a person gives once and answers one question about one request. It depends on the protocol and
on no runtime: it cannot create a namespace, spawn a process, or open a socket, and the layering
validator fails if that changes. Written in this order for the reason the authorization gate was —
build the sandbox first and the decision about what may happen inside it dissolves into whichever
call site needed it.

Three answers, not two, and the distinction is the load-bearing part. `Allowed` means nobody is
asked, for hours. `CrossesBoundary` means the request left the capsule and is answerable, so it
becomes an `ActionProposal` and takes the path ADR-0022 already defines. `Refused` means never —
another capsule, the Journal, the key store — because there is no answer a person could give that
would make those safe, and offering the question would be offering a choice that does not exist. A
fourth, `NotGranted`, says a profile *could* have included this and does not, which sends a person to
widen their profile rather than to argue with the architecture.

Path containment is lexical and says so. `..` is applied before comparison, because
`/srv/project/../../etc/shadow` is inside the workspace by string prefix and outside it by meaning —
that is where sandboxes actually fail, and a mutation check confirms the test sees it. Symlinks are
not followed: resolving them means touching a filesystem, and a decision whose answer depends on when
it was asked is not a decision. A symlink out of the workspace is the kernel's to refuse.

**A selected profile now has one public path to authority.** `CapabilityProfile` contains only the
reusable limits shown at launch. `LeaseRequest` binds the explicit selection to a fresh capsule id,
agent and workspace, and `issue_lease` validates exact host and tool names and compiles that same
grant before minting anything. The resulting lease records the profile id and adds no implicit
network, model, tool or execution capability. Its public-boundary gate proves that one selection is
silent for every ordinary request inside the live lease and that an ambiguous or un-runnable
selection fails before it becomes authority.

**A grant has an end, and reaching it is not a request.** The lease carries the clock and the
ledger: after its lifetime or after somebody withdraws it, nothing it used to permit is permitted.
The agent is not told to stop — telling an untrusted party to stop is a request, and a boundary made
of requests is not a boundary. This produces the first half, that nothing further is `Allowed`;
freezing the capsule is the kernel's, and this module does not pretend otherwise.

**Two kinds of ending, and they are not the same kind.** The lifetime running out or somebody
withdrawing the lease finishes the capsule; running out of model budget refuses completions and
stops nothing else. Folding them together was wrong twice over. It would have frozen an agent
halfway through a compile for running out of money it was not spending — and because a zero ceiling
read as an exhausted one, a capsule that wanted no model at all was dead at the instant it was
issued, including the grant this crate hands out as the starting point for a profile and every
capsule on an unplugged host, which is the configuration the whole system exists to survive. A model
grant is now optional and separate, and a zero ceiling is a real configuration meaning *use something
free and run up no bill*, distinct from having no grant.

Two endings, kept apart because only one of them is something a person did. A lease withdrawn and
then expired was **withdrawn**: reporting the expiry would quietly replace somebody's action with a
timer running out. The spending ceiling is reached *at* the ceiling, not one unit past it — `>` lets every
limit be exceeded by exactly one, which is invisible until a month of them is added up — and
charging is saturating, so no arithmetic here can make a lease look healthier than it is. Both are
mutation-checked. A request that would be refused outright is still recorded as refused rather than
as a dead lease, because an audit saying *lease ended* for an attempt on the key store would have
lost the interesting half.

Spend is whole units of the smallest denomination. A ceiling compared as a float is one that is
occasionally exceeded by a fraction in whichever direction the rounding went, and *occasionally* is
not a property a limit may have.

**A grant compiles once into what the kernel is asked for.** `KernelCapsuleSpec` is that compiled
shape — a value, inspectable and comparable, that a test asserts against with no Linux kernel in the
room, and that is recorded beside what the capsule then did. Compiling is total and deterministic, so
a capsule can be examined before it exists and two runs can be compared.

As much as possible is unrepresentable rather than merely unused, because an unused possibility is
one somebody uses later for a good reason. `Namespaces` has no fields: a capsule gets all seven or is
not a capsule, and something switchable would eventually be switched off by somebody debugging. There
is no *no-new-privileges is off*. `Network` has two variants: `Denied`, and `Brokered`, which carries
only a capsule-local port and pathname. The grant's host list never enters the kernel spec: it stays
with the broker that owns the name decision, so runtime plumbing cannot become a second network
policy by accident.

**The filesystem is built up, never pruned down.** The mount list starts empty and gains what a
program needs to be a program, read-only, and one writable path. A host root with things removed is a
deny-list, and a deny-list is a list somebody forgets to extend — where the thing forgotten is found
by an agent rather than by a reviewer. The workspace appears at a fixed place inside rather than at
its host path, so an agent learns nothing about the machine and a profile moved between hosts
produces the same environment.

The one path a person supplies is the workspace, so it is the one place a grant can ask for something
that undoes everything else. A workspace that is, contains, or sits inside the root, `/etc`, the
Journal, the key store, `/proc` or `/sys` is refused with a reason rather than compiled into
something that runs — checked in both directions, since a workspace inside `/etc` exposes part of it
and one containing `/etc` exposes all of it. The first version of that check was wrong in the
refusing direction: every absolute path starts with `/`, so everything collided with the root and
nothing compiled at all. Its own tests caught it, and both directions are now mutation-checked.

**The command that builds a capsule is a value, not something buried in a spawn.**
`CapsuleBackend::command` returns the argument vector, so whether a sandbox is correct is answerable
by reading what it was asked to do rather than by running it on a machine willing to run it. One
implementation, bubblewrap, behind a trait — a first implementation that is also this project's own
`clone`/`unshare`/mount orchestration is the wrong place to be original.

Three properties are mutation-checked, because each is the kind that fails silently. The environment
is **emptied** and rebuilt by name, not filtered: a filter is a deny-list somebody keeps current, and
the host environment of a Cybou process holds the key store path, an SSH agent socket, and the next
thing somebody exports. `/proc` is the capsule's own — the host's, bound in, is the PID namespace
undone by a mount, which looks like a convenience. And the program follows `--`, so a program named
`--bind` is a program rather than an argument to bubblewrap, which is the same lesson this repository
already learned about passing a systemd unit name.

Also asked for and asserted: no nested user namespaces, a private `/tmp` rather than the host's
shared one, a session of its own so nothing can push characters into the terminal that started it,
and death with the parent — a capsule that outlives its supervisor is a lease that ends with nothing
left to act on, which turns *ending is not asking* back into asking.

**A capsule now actually holds, and the gate says so by trying to break it.** `scripts/test-capsule-gate.sh`
builds a real capsule from the argument vector this crate produces — through an example binary, so it
tests the code rather than a command written out in a shell script and left to go stale — and then
attempts, from inside it, everything ADR-0042 G1 says must fail. The workspace is readable and
writable and a program runs; `/etc/shadow`, the Journal and the host root are absent; a symlink out
of the workspace leads nowhere; fewer than ten processes are visible and a host process cannot be
signalled; there is no interface but loopback; a nested user namespace is refused; and neither the
key store path nor an agent socket came through. It runs twice, the second time asserting no Cybou
process is involved, because a capsule that holds only while Mind is watching has cognition for a
boundary.

**Three of those checks were worthless when first written, and mutation testing is what found it.**
A connectivity probe used `/dev/tcp`, which is a bash feature while the capsule runs `/bin/sh`, so it
failed identically whether the network was denied or wide open — removing `--unshare-net` left the
gate passing. A loopback check printed the same word on both branches and could not fail. And the
environment checks passed on a shell that happened not to have the variables set, so removing
`--clearenv` changed nothing; the gate now exports them itself, because a gate must create the
condition it claims to test. All three are structural now and all three fail under mutation.

The first run also found a real defect in the backend: `--seccomp` takes a file descriptor, and it
was being emitted bare, so bubblewrap read the `--` before the program as the descriptor and nothing
started at all. **No seccomp filter is applied by this build.** The flag is not emitted, and
`requires_seccomp()` says the debt is owed, because a known gap is worth more than a silent one.

**The budget is a cgroup, and it is a transient service rather than a scope.** That is a measured
decision, not a stylistic one. Asked for the same limits, `systemd-run --user --scope` accepted every
property, reported success, and left `MemoryMax` at infinity; the service form put `memory.max=67108864`,
`pids.max=17` and `cpu.max=50000 100000` into the kernel. A scope implementation would have looked
correct in the code, in the command and in every record, and held a capsule to nothing — which is the
worst available outcome for a limit. The gate reads the kernel's own files rather than
`systemctl show`, and switching that line to a scope fails three checks, which is how this is known
rather than assumed.

`MemorySwapMax=0` goes with the memory ceiling. Without it a capsule at its limit pushes the host
into swap instead of being stopped: the limit failing in the direction that hurts the machine rather
than the capsule. The lifetime is `RuntimeMaxSec` on the unit, not a timer inside Mind, because a
lifetime enforced by something that has to still be running ends when that thing does — and *ending
is not asking* means the end must not depend on anyone being there to ask.

A capsule with no CPU quota is now refused at compile time alongside no memory, no processes and no
time. Zero is not a small share; the cgroup holds the capsule at a standstill, which looks exactly
like one that hung.

**A lease that ends is now a capsule that stops.** Until this week the end of a lease was a
decision and nothing more: after it no request was `Allowed`, and the agent carried on running,
holding its memory, writing to its workspace and talking to whatever it had already opened. A
permission withdrawn from a process that never notices is not a withdrawal.

`cybou-capsule::end` produces two commands, in order. Freeze, then kill — and the order is the whole
point rather than tidiness. Killing a live process tree is a race the tree can win: a capsule under a
ceiling of a few hundred tasks can fork faster than the signals arrive, and every new process is one
the kill pass has already gone past. Freezing ends that race before it starts.

That sequence is only sound because `SIGKILL` reaches a *frozen* cgroup, which is not obvious, and
would be catastrophic to assume: if it did not, a capsule would stay alive for as long as the host
was up while every record said it had been ended. It was measured on a frozen unit rather than
believed, and the gate repeats the measurement on every run.

`SIGTERM` appears nowhere. A termination signal is a request honoured at the recipient's discretion,
and the recipient here is the party this crate exists to bound. The kill addresses the whole cgroup,
not the first process, because a capsule's first process is not the capsule — everything it forked
would carry on, reparented and no longer named by anything that could find it again.

The first version of the gate for this could not tell the difference. Its capsule was a plain
`/bin/sh`, which dies to `SIGTERM` as readily as to `SIGKILL`, so a build that merely *asked* the
capsule to stop passed every check; *ending is not asking* was being verified by a string in a unit
test. The gate's capsule now ignores every signal it is allowed to ignore, the way an agent that
wanted to outlive its lease would, and the `SIGTERM` build fails with two processes still running.
Two more checks were vacuous before they were mutation-tested: one read `cgroup.procs` only after the
kill, where a missing file counted as zero survivors — and systemd removes the directory along with
the unit, so it passed because the path had gone rather than because the processes had.

There is no `thaw`. Nothing in this design resumes a capsule, because nothing in it un-ends a lease.

`Ended::Expired` and `Ended::Revoked` produce the same two commands. They differ in what is recorded,
which matters — an operator told their agent ran out of time when somebody in fact stopped it has
been told the wrong thing — but not in what is done. A gentler ending for the gentler reason would be
a lease that expires more politely than it is revoked, which is a lease an agent could prefer to
reach.

**There is a second barrier now, and finding somewhere to test it found a hole.** Landlock is a
restriction a process applies to itself, inherited across `exec` and never removable, which means it
cannot be written on a command line — and a seccomp filter has the same shape, with bubblewrap's own
`--seccomp` wanting a file descriptor no argument vector can carry. So a capsule is entered in two
hops now: `systemd-run … -- bwrap … -- cybou-capsule-enter … -- the agent`. The new crate applies the
ruleset to itself and then becomes the agent.

It runs *after* bubblewrap, which is why the paths it is given are the ones seen inside the capsule.
Landlock applied before would have to permit everything bubblewrap needs to build a sandbox, which is
most of the host.

An agent that finds this program can run it, and that is fine: Landlock and seccomp are monotonic. A
process may add restrictions to itself and may never drop one, so invoking it again with a generous
list produces a process bounded by what it already had. There is no argument to it that grants
anything.

If the kernel will not enforce what was asked, it refuses to `exec` rather than carrying on — an
agent running with one barrier where it is supposed to have two, and nothing anywhere saying so, is
the exact failure this tree keeps finding.

The hole: the gate needed somewhere the mount namespace and Landlock could be told apart, since every
mount had a matching rule and a write denied by both proves neither. The capsule root turned out to
be it. Bubblewrap builds `/` as a **writable tmpfs**, and the ruleset names `/workspace`, `/tmp`,
`/dev`, `/proc` and the read-only system paths but never the root — so before this an agent could
write to `/`. Removing the Landlock call makes the check fail with `WROTE`, which is how that is
known rather than argued.

A second thing fell out of it. The Landlock list was built from the mounts, and `/proc`, `/dev` and
`/tmp` are made by the backend rather than mounted — so they were absent from it, and Landlock denies
what it was not told about. The first thing to break was `/dev/null`: every redirection in every
script an agent runs, failing with a permission error, on a capsule whose mounts were perfectly
correct.

The entry program is bound at `/.cybou-capsule-enter`, on the root tmpfs, and not under `/usr` where
it belongs by convention. `/usr` is bound read-only, so there is nowhere under it to make a mount
point, and bubblewrap refused the whole capsule rather than half of it. That refusal was right; the
obvious fix — binding `/usr` writable — would hand an agent the compiler it is about to run.

`Bubblewrap` now carries the entry program's location and has no constructor without one. A backend
that could be built without it would be a type able to describe a capsule missing two of its ten
parts, and the missing half would not appear anywhere in the command it produced.

**The seccomp debt is paid, and not the way it was owed.** `Bubblewrap::requires_seccomp` used to
return true and say plainly that no filter was applied: bubblewrap takes one on a file descriptor,
and an argument vector has no descriptor to give. The first attempt pushed a bare `--seccomp` anyway,
bubblewrap read the `--` before the program as the number, and nothing started at all.

The answer was not a descriptor. It was that a filter, like a Landlock ruleset, is something a
process installs on itself just before `exec` — so it belongs in the entry program, which had to
exist for step 3 regardless. `requires_seccomp` is false for this backend now, and the reason it is
allowed to be is that the gate kills a capsule that calls `unshare` and reads the signal.

What is denied is the small set that would let a capsule rearrange itself, never an allow-list. An
allow-list against a development agent — compilers, linkers, package managers, whatever a build
script felt like that morning — is either enormous or breaks on a Tuesday, and a sandbox that breaks
a legitimate build gets switched off, which is the least secure outcome available.

A matched call **kills** the process rather than returning `EPERM`, and the deciding reason is about
what can be tested. `EPERM` is already what these calls return to an unprivileged process in a user
namespace, so a filter returning it would be indistinguishable from no filter at all — the gate would
pass identically on a build where none was installed. That is not hypothetical: the existing check,
`no nested user namespace`, passes with the filter removed, because bubblewrap's own flags already
refuse it. The new check reads the exit status, sees `159`, and fails with `only-refused:1` when the
filter is taken out.

`clone3` is the one exception and has to be. Seccomp reads a syscall's arguments but never the memory
they point at, and `clone3` takes a pointer to a struct, so its flags are invisible to any filter.
Killing it would break modern glibc, which creates processes with it. It is answered `ENOSYS`
instead — the documented handshake: glibc concludes the kernel lacks it and falls back to `clone`,
whose flags *are* an argument and are checked for `CLONE_NEWUSER`. Every fork ends up inspected, at
the price of one wasted syscall per process.

Two filters, because one filter has one action for everything it matches. Seccomp evaluates all
installed filters and returns the most severe answer, so a kill filter and an `ENOSYS` filter applied
in turn give each call the answer meant for it.

**There is a way out of a capsule now, and it decides by name.** `cybou-egressd` speaks `CONNECT`
over a Unix socket, checks the host against the grant, and then does the resolving itself.

That last clause is the whole design. A grant says `github.com`; a firewall works in addresses; and
turning one into the other means owning a policy for how long a resolution is good for, what happens
when it changes underneath you, and what a name means when it answers differently to every caller.
Every one of those is somewhere being wrong is silent — the rule still loads, the counters still
increment, and the capsule reaches somewhere nobody granted. Here there is one resolution, it happens
after the decision, and the capsule neither performs nor supplies one, so there is no window between
checking a name and using it.

An address where a name belongs is refused before any grant is consulted, and that refusal is about
grammar rather than permission: `CONNECT 140.82.121.4:443` cannot be checked against a grant at all,
and accepting it would make the name in a grant decoration.

The second check is not about the grant. A name is controlled by whoever runs it and can answer with
anything, including `169.254.169.254` — where every cloud host serves its own credentials to whatever
asks. A broker that checked the name correctly and connected there would have done everything right
and handed over the machine. So every address a granted name resolves to is checked against loopback,
link-local and the unspecified address, and it is every address rather than the first, because a name
that answers with the metadata endpoint second would otherwise be reached on a retry. Private ranges
are permitted: an operator who grants an internal host name means it, and a rule people work around
is a rule that ends up switched off.

It is a tunnel and not a proxy. After the decision it copies bytes it does not interpret, so the
capsule's traffic is between the capsule and what it was granted, and the broker is not a place that
traffic could be read.

The address check canonicalizes IPv4-mapped IPv6 before classification, so
`::ffff:127.0.0.1`, `::ffff:169.254.169.254` and `::ffff:0.0.0.0` cannot cross through the IPv6
arm. Immediately before connect it also asks the kernel whether each exact resolved address belongs
to this host. Private ranges remain permitted; this host's `10.0.0.4` is refused while another
machine's `10.0.0.20` is not refused merely for being private.

CONNECT is read through the blank line under one 8 KiB ceiling, so ordinary proxy headers never
become the first bytes of the TLS tunnel. Handshake, DNS/connect and idle time are bounded, and one
broker admits at most 64 concurrent tunnels; the 65th receives 503 rather than allocating another
host task, file descriptor and outbound socket. The runtime directory is `0700`, the socket is
`0600`, and startup replaces only a socket proven stale — never a file, symlink or active listener.

**The last hop exists.** `cybou-egress-bridge` runs inside the capsule and copies opaque bytes from
`127.0.0.1:3128` to its one pathname Unix socket. It has no grant, DNS or policy vocabulary. The
entry program applies Landlock and seccomp, forks the bridge, waits until it is listening, and then
execs the agent, so the agent is PID 1 and the bridge is its one infrastructure child. Both inherit
the same namespaces, barriers, cgroup and lifetime. A brokered grant with fewer than two tasks is
refused at compile time rather than producing a capsule that cannot hold agent plus bridge.

The network gate now runs `curl` and `git ls-remote` through that complete path. It also checks a
denied name, direct-address no-route, exact host-interface refusal, cross-capsule socket absence,
bridge death, and the host-side tunnel ceiling. Killing the bridge removes network and grants no
direct route. Mind participates in none of it.

`validate-organ-layering.py` now refuses a governance crate that names `cybou-egressd`. The capsule
crate decides what an agent may reach and the broker connects it — the same split as `cybou-actiond`
and `cybou-executord`, one layer down, and the kind of edge somebody adds for a good reason, which is
why it is checked rather than remembered.

**None of this enforces anything.** A capsule holds because the kernel holds it. This is the
description of what was granted, used to decide what to ask and what to record.

**A proposal carries who is asking, and a permission does not cross between them.** Found while
wiring the capsule to the action boundary, which is where this tree keeps finding things. A person
pre-authorizes `package.cache.clean` because the party asking reached a finding from readings it
gathered and can show them. `ActionProposal` carried no proposer and `StandingPolicy` was one flat
list of verbs, so the moment an agent existed it would have inherited that permission — unattended,
on evidence nobody saw, from a party this system trusts not at all. There are two lists now, the
agent one empty by default even on a machine whose owner has pre-authorized plenty for Cybou itself,
and neither leaks into the other. Mutation-checked in both directions.

The same wiring exposed a third instance of the vacuous truth. `find` over an empty check list
returns nothing, so an untrusted proposal that no critic examined ran straight into the
pre-authorization test and was granted. A proposal nothing examined is not one that passed
examination, and that is now an explicit refusal naming who asked.

And it exposed a category error worth keeping. The critic *does this action relieve the finding*
cannot run against a request that cites no finding — it objects, correctly by its own rule, to a
claim the agent never made. So the critics are split: the two that need a finding, and the two that
do not. The second of those matters most here, because `ActionProposal` carries risk and
reversibility as ordinary fields, and an untrusted party asking for something dangerous while
calling it `Low` is exactly what that check is for.

**An action does not get to say whether it worked.** The outcome stage is built before the
executor, on purpose and for the same reason the gate was: the natural shape of an executor is one
that returns whether it succeeded, and an executor written first arrives with that answer already in
its return type. Written second, it arrives to find its own report is one of two fields and not the
deciding one.

`AttemptReport` is what the thing that carried it out said; `Relief` is what the readings say
afterwards, derived from findings taken before and after by the telemetry organ — which did not
carry the action out and has no notion that one happened. `Agreement` is whether those two tell the
same story, and it is a value rather than something a reader is left to work out, because the case
that matters is the one where they differ: `apt clean` exits zero on a filesystem that is still
full, and anything recording only the exit code records a remedy that worked.

Three of the relief states are ways of not knowing, and none collapses into failure. A measure that
went unreadable after the attempt is the worst of them to get wrong — a finding disappearing because
nothing could read the thing it was about, reported as the problem being solved. An attempt read
sooner than ninety seconds after it ended establishes nothing, because a restart takes longer than a
sample interval. An operation that was declined has nothing to have relieved, and is not offered a
rollback for something that never happened.

A finding is matched across the two sets by what it is and what it is about, never by identity: a
condition that briefly cleared and returned carries a different identity, and for this question it
is still present. An authorization decision now carries its own derived identity, so an attempt can
name the permission it rested on and not only the proposal it carried out.

**The first executor now exists, and the split is a process boundary.** `cybou-actiond` owns the
proposal lifecycle and mints a random, 60-second, single-use `ExecutionPermit` only after criticism
and a granted standing-policy decision. The default policy still grants nothing. The browser's
read-only offer projection mints no lifecycle identity; Action1 is the owner of proposal and permit
identity.

`cybou-executord` receives only an opaque permit identity. It atomically claims the complete typed
action from Action1, so its caller supplies neither a verb, a program nor arguments. Claiming now
also mints a stable attempt identity and synchronously writes `ExecutionStarted` to the Journal.
Action1 does not return the action to Executor1 until that write is acknowledged, so a Body effect
can never precede the durable fact that it may have begun. If the executor dies or its reply is lost
after mutation, replay materializes `DidNotFinish`; that episode blocks automatic repetition rather
than turning missing evidence into permission. Executor1 reports its final `ExecutionAttempt`
directly to Action1, while `cybou-remediationd` only coordinates re-observation and outcome. There are three
adapters: `service.status`, fixed `/usr/bin/apt-get clean`, and `service.restart`. Services use the
systemd manager D-Bus API and only concrete names ending in `.service`; the `systemd:<unit>`
placeholder and every operation without one of those adapters are refused before a permit exists.
The layering validator rejects any dependency from the governance owner to `cybou-executord`.

The A1 gates also hold the adversarial interval explicitly: a Body restart can happen and the final
report can be lost; the durable start survives Action1 restart as `DidNotFinish`, and initiative
returns `OutcomeUnknown`, so there is zero second mutation. The live A1 gate creates a harmless disposable systemd unit, observes it inactive, passes a strong named
finding through Action1 and the executor, and then re-reads systemd independently until the unit is
active. It also replays the permit and requires refusal. No model, shell command string, real
workload or executor self-report supplies the final observation.

**That topology now ships on the VPS.** `cybou-actiond` is a hardened user service in
`cybou-mind.target`. The root `cybou-executord` is a separate system service and owns
`org.cybou.Body.Executor1` on the system bus. Action1 remains owned by the unprivileged Cybou user
but exports its permit endpoint on that same transport, because a session bus rejects a root client
with a different UID. Explicit D-Bus policy permits only Cybou to own Action1 and call Executor1,
and only root to own Executor1 and claim from Action1. The caller still supplies only the opaque
identity. The gate exercises this same transport and policy shape.

`/etc/cybou/action-policy.env` is root-owned, created empty, and never overwritten by a deploy.
`cybou-action-policy` accepts only the three implemented verbs, writes the replacement atomically,
and restarts Action1. An invalid list leaves the previous policy byte-for-byte intact. Thus
installing or upgrading grants nothing; unattended authority appears only after an operator runs
that explicit command.

## ACP discovery boundary

`cybou-acp` now speaks stable ACP v1 to an agent subprocess over stdio using the upstream Rust SDK.
The implemented exchange is deliberately only `initialize`: it records the negotiated wire
version, implementation identity, authentication methods and advertised capabilities, and refuses
an agent that selects an unsupported version. The process configuration passed to this library must
be a capsule entrypoint; the protocol client is not a sandbox and no production command exposes a
raw host-process launcher.

The deployed `cybou-acp registry` command exposes only that read-only browser. The handshake probe
is a development example and is not installed on the VPS.

The registry browser fetches the canonical public index at
`https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json` over HTTPS, with a fifteen
second deadline and an eight MiB response ceiling. Required manifest fields, unique identities and
at least one distribution declaration are checked before an entry is presented. Search is local and
deterministic. Distribution commands, arguments, environment declarations and URLs remain inert
upstream metadata: B2 discovers them and installs nothing. Every snapshot carries its source and
observation time because `latest` is mutable.

**A watched thing has four states, and three of them are not silence.** `Observed`, `NeverRead`,
`ReadFailed` and `Stale`. A declared thing that produced no reading used to be simply absent from
every surface, which reads exactly like a thing nobody declared — and the operator who declared a
certificate and sees nothing about it has been told, by that silence, that it is fine. The three
unhappy states are kept apart because they call for different actions: a path that may not exist, a
file this process cannot open (usually a permission), and a probe that worked and has stopped
(usually the sampler, not the thing sampled). A failed attempt outranks an old success, because
"last read four minutes ago" hides "and every attempt since failed"; a reading that arrives clears
the failure before it. The unread things are named in the prose *before* the all-clear, and never
counted among the things looked at.

**A finding keeps one identity for as long as it is one condition.** The identity is derived from
what makes a condition itself — what was concluded, what it is about, and when it began — rather
than generated per read. A fresh identity per read meant two requests a second apart described one
physically identical situation with two different identities: harmless while nothing referred to
them, and an architectural defect the moment an action proposal cites one as its cause, because the
cause it names would not exist by the time anybody looked. An episode that ends and returns gets a
new identity, which is correct: it is a new occurrence, and a `since` spanning both would describe a
stretch of time the host was fine for part of.

**A declaration line is read strictly, because a lenient reader is a silent one.** A word no kind
uses is refused rather than discarded — it is usually a typo in the word before it, or a path with a
space in it the operator believes was taken whole. The same thing declared twice is refused rather
than deduplicated, because two lines about one marker usually disagree and keeping either one
silently chooses a policy. A threshold of `inf` or `NaN` is refused: watched-and-never-judged is a
real state, reached by declaring no threshold, not by writing one nothing can reach. Zero or
negative is refused too, since both mean an alarm that never clears. A declared unit name reaches
`systemctl` after `--`, so a name beginning with a dash cannot become an option to systemctl.

**Backup age has no universal threshold, and this build does not invent one.** How stale a backup may
get is a policy the operator holds, and two backups on one host can honestly disagree, so the number
comes from the declaration and the subject table supplies none. A backup declared without one is
refused rather than defaulted; a backup watched with no threshold at all is watched and never judged,
which is the honest state rather than a number nobody chose.

Nothing is offered to remedy an expiring certificate or a stale backup. Renewal is a deadline met
outside this machine's control, and what would relieve a stale backup is the backup succeeding —
there is no operation here that could make it. An inactive service is offered the same three things
as a failed one, because which it is affects what the host says rather than what it could do.

Two of them exist because an existing measure reads healthy while the machine is broken. A filesystem
out of inodes has free bytes, so every byte-based reading says forty percent used while nothing can be
created. A host at its file-descriptor limit has memory, disk and load all fine, and cannot open a
socket. Both are found and both are their own finding, because deleting things frees no descriptors.

**Telemetry is not biography.** A `Reading` has no path into the Journal anywhere in this tree — no
kind, no conversion. It is transient by construction rather than by policy. A `SystemInsight` may
enter Event1, and only as a `Hypothesis` carrying the readings it was drawn from.

Detection is robust statistics — median and median-absolute-deviation over the window — rather than a
model, because the fault contaminates a mean and a standard deviation while it is happening, and
because the answer must be checkable by a person and runnable on one vCPU. Categorical facts (a disk
at 95%, a failed unit) are evidence in their own right and do not need a baseline.

A window too short to have an opinion says so. That is a different answer from *nothing is wrong*.

**Where things are heading** is a separate question from what is wrong now, and a disk at 71% that
produces no finding can be the most important thing on the page. The slope is a Theil–Sen estimate
over at most 128 points sampled evenly from the window — the estimator compares every pair, so a
full six-hour window would be 2.33 million comparisons for one subject and four seconds for a page
load on one vCPU. The surface that answers *why is this server busy* must not be a reason it is.
The detector still sees every reading; only the slope is estimated from a subset, and the sampling
is deterministic so two projections of one state are the same projection. What it costs is a little
precision in a number that is rounded to "about three days" before anyone reads it. The slope is
the median of the sampled pairwise slopes — so one spike does not move the date, and the yardstick it is
called flat against is a median absolute deviation for the same reason. A subject that is flat or
moving away does not arrive: *not at this rate* is an answer, and a very large number would be read
as a date — and *away* depends on which side of the threshold the problem is on, because a
certificate losing a day a day is approaching while a filesystem losing a percent a day is
retreating. A projection is measured from now rather than from the last reading, and it says when it
is looking further ahead than the window has watched — the most useful projection is usually the
least certain, and a reader deciding whether to act tonight is entitled to both facts.

## Meaning

An utterance becomes a typed `CognitiveAct` or none at all; an unrecognised opening produces no
interpretation rather than a guess. A reference the vocabulary cannot settle stays unresolved and
names the candidates it was torn between. A bare pronoun never resolves.

Dialogue state remembers **referents, not a topic**, bounded by turns, by time, and by erasure. It
can make an ambiguity visible and has no way to make one disappear; there is deliberately no
accessor for "the current subject".

Answers are built as a typed `ResponsePlan` before any prose exists, so qualifications — not read,
stale, partial, withheld, unverified, disputed, superseded — are decided in the typed layer where a
renderer cannot lose them. Realization has no input but the plan. Plans compose only when they share
an intent, and **a qualification on any part qualifies the whole**.

Everything here is deterministic and runs with no network and no model.

## Context and attention

Activation walks associations from seeds under a budget enforced in five dimensions — nodes, edges,
depth, time, tokens — and says which one stopped it. Every reached concept carries the path it came
along; *why did you think of that* is answered from the graph, never by asking something to compose
a reason.

**A seed is not only a word.** ADR-0029 states the cost of getting this wrong plainly: restricting
seeds to text would make the whole layer an accessory to a chat box. A `Seed` is a concept, what the
workspace is looking at, an intention being held, a finding this host reached about itself, a metric
it watches, or an episode — so the host can ask what relates to `storage.exhaustion` without
anything having to phrase it first, which is the class of question a machine asks about itself.

Kind is part of identity rather than a prefix convention. A file called `lemon` and the concept
`lemon` are different seeds; under plain strings they were one key, so activating from either
returned what belonged to the other along a path that read entirely plausibly and was about the
wrong thing. A concept keeps its bare label, so a graph built by people naming things is unchanged.
Seeds the graph has never held are named back rather than counted — a caller that cannot tell which
of four seeds found nothing cannot tell an empty corner of the graph from a mistyped one.

An epistemic standing travels with a concept through retrieval and into attention. A concept reached
*through* a disputed one is **not** thereby disputed: the walk is association, not inference.

Attention admits proposals under a quota and **a proposal never evicts a resident**. Relevance
discovered by retrieval is not permission to displace the current focus. What was refused is counted,
so a short list is never mistaken for a whole one.

## Disclosure

**What a restored backup can read is checked against an actual backup.** ADR-0028 says a copy taken
before an erasure still holds the ciphertext and only a destroyed key reaches it. Every other test
here checks the live database — the copy the erasure ran against — which can prove a row was
redacted and can prove nothing about a copy nobody controlled. So the file is copied, the erasure
runs, and the copy is opened as a journal afterwards: that is the restore, not a simulation of one.

**A fresh installation keeps its keys where a backup of the Journal does not reach them.** The store
goes under the state directory and the Journal under the data directory — a separation ADR-0017
already draws, and the first thing for which it is more than tidiness. An installation that already
has a store keeps using it, wherever it is: moving one would leave a deployment unable to unwrap
yesterday's keys, and this organ has already learned once what that costs. So the old layout stays
reachable and is reported at every start for as long as somebody runs it. Deployed and checked on
the live host, which has the old layout: no second store was created, and `master.json` still
carries its original timestamp.

**The guarantee has a precondition, and it is now a test rather than an assumption.** It holds only
because the key is somewhere the backup did not reach, and `cybou-eventd` puts the key store beside
the Journal by default — so `tar czf backup.tgz ~/.local/share/cybou/` captures both, and a restore
of that reads everything the erasure was meant to make unreadable. A second test demonstrates
exactly that, and the daemon says so at every start. It is not a defect the crypto can fix; it is a
fact about what a deployment must exclude, and a guarantee whose precondition is untested is one
that will be reported as holding on the day it does not.

Writing this found a second thing worth keeping. SQLite in WAL mode holds recent writes in
`journal.sqlite3-wal` until a checkpoint moves them, so copying the main file alone from a running
system produces a backup that opens cleanly and is missing the newest contributions — worse than one
that fails, because it restores and looks right. Measured, not assumed: the test that does it prints
`a single-file copy held the contribution: false`.

**So the writer offers the primitive a backup needs**, because it is the only party that can produce
one. `Event1` can write a consistent copy of the Journal in a single statement against the
connection that made the commits, and the result is a plain database file — no WAL to carry
alongside, nothing to replay. It refuses to write over anything: the file most likely to be at a
backup path is the last good backup, and replacing it with one that then fails halfway is how
somebody ends up with neither.

It takes a snapshot and nothing else. Scheduling one, keeping a rotation, and deciding what to
delete are an operator's policy, and `BackupState` exists so a deployment declares theirs rather
than having one assumed. The copy holds ciphertext and no keys, which is what keeps it inside the
erasure guarantee.

**A person can see what they were supplied before now, not only what they are being supplied.** The
surface answered one question — what am I being given — which makes it a status light rather than a
record. The recent deliveries to this consumer are carried beside the current one, bounded to
sixteen per consumer and sixty-four consumers, and only *changes* produce an entry: a reader
receiving the same projection every few seconds fills nothing. The history carries counts and an
instant, never the items or the subjects — repeating every subject for every past delivery would
multiply the one thing the withholding rules exist to keep rare by the length of the list. The
durable record remains the `ContextDisclosed` contribution in the Journal; this is a window onto its
recent end, so the surface answers without a Journal query.

**And the window says it is a window.** The list lives in the gateway process and starts empty when
the process does, so three entries are three changes since this gateway started rather than three
deliveries in the life of the machine. `historyComplete` is false and `historyCoverageSince` names
the earliest instant the list could cover, because a surface that let a person read a fraction as
the whole would answer *what was I supplied* with a fraction and no hedge. Making it true means
reading the `ContextDisclosed` contributions back at start, which is worth doing and is not done.

Every supply of the Mind projection across a boundary writes a `ContextDisclosed` naming the
consumer, the contributions the supplied items came from, and what was held back and why.

`GET /api/v1/disclosure` shows the person it is about: how much was supplied against how much can be
accounted for, and every refusal with its reason. A withheld subject is named to the owner and
withheld from a stranger — a surface reporting a filter must not be a way around it.

Recorded provenance is bounded, and a record says by arithmetic when it is a sample: the count
exceeds the length. The count is optional, because a record written before the field existed cannot
say how many sources there were, and that is not zero.

## Desktop (CYBOU Spatial Desktop)

One Rust/WebAssembly frontend — Living Canvas — served to browsers and, as a target, to a
Chromium/Wayland session ([ADR-0037](adr/ADR-0037-web-first-presence-and-desktop.md), [ADR-0040](adr/ADR-0040-spatial-card-desktop-and-bounded-body-capabilities.md), [ADR-0044](adr/ADR-0044-cybou-spatial-desktop-architecture.md), [ADR-0045](adr/ADR-0045-cybou-core-desktop-pack-and-workspace-primitives.md)). The browser is a renderer and an untrusted client: it talks only to the
gateway and never becomes a Mind owner, D-Bus peer, or authority.

CYBOU Desktop is an infinite spatial presence map of host reality, rejecting classical window managers and tabbed page-routed SPAs:
- **Panels & Decks**: Fourteen singleton system cards and dynamic tool cards (code: `Card`, UX: `Panel`), with tabbed grouping inside decks (`role="tablist"`).
- **Presentation Modes**: Supports both the canonical **Home / Operator** mode (System Insight, Agents, Recent Activity, Forecast) and the relational **Mind Explorer** substrate mode.
- **System Insight**: Renders the 5-stage self-healing lifecycle (`Detected → Decided → Acting → Re-observed → Relieved`) and comparative baseline explanations (`Why?`), strictly matching durable `ActionRecord` projections from `Action1`.
- **Agents Runtime**: Reads canonical `SessionView` records and structured `AgentOffersResponse` (`profiles_state`, `capacity_state`, `provider_state`) from `Agent1` through the gateway. When unconfigured, it displays multi-dimensional setup readiness diagnostics. When configured, it offers guided profile, workspace, model class, and autonomy boundary selections, alongside live task prompt, real-time execution phase tracking, agent result response, and direct File Manager workspace inspection.
- **Ask CYBOU & Command Palette**: Provides instant deterministic query resolution across host findings, remediation actions, running agents, and isolation boundaries with strict epistemic truthfulness.
- **Spatial Architecture v2 ([ADR-0044](adr/ADR-0044-cybou-spatial-desktop-architecture.md))**: Defines the formal blueprint for Panel 2.0 representations (`Glance`, `Standard`, `Expanded`, `Focus`), semantic clusters, semantic zoom / level-of-detail, canvas anchors, camera history, typed relations, contextual spawning, and non-cognitive layout invariants across milestones SD0–SD14.
- **Core Desktop Pack & Universal Workspaces ([ADR-0045](adr/ADR-0045-cybou-core-desktop-pack-and-workspace-primitives.md))**: Defines the 3-tier desktop capability model (`Desktop Core`, `Personal Core`, `CYBOU-native Core`), `LocationRef` authority domains (`HostUserPath`, `SystemConfigPath`, `AgentWorkspace`, `SafeShellJail`, `BackupSnapshot`), Text Editor & Diff Engine with Action1 commit governance, Files 2.0 multi-panel management, Mail/Calendar/Notes zero-trust architecture, Control Center, Storage/Disks vs Files differentiation, and Dual Terminal execution across phases CP0–CP10.

Every class the components render has a rule, checked by `scripts/validate-desktop-styles.py` and true again since 2026-08-30, when the thirty-eight that had none were given one; interaction is exercised in real Chromium. See [desktop and browser gate](evidence/desktop-browser-gate.md).

A stranger is served the sign-in view and nothing else where the deployment says so.

## Terminal

`cybou-ptyd` owns one interactive pseudoterminal per connection, running as the account it was
started for ([ADR-0047](adr/ADR-0047-interactive-terminal-under-the-authenticated-account.md), which
supersedes the shell half of [ADR-0040](adr/ADR-0040-spatial-card-desktop-and-bounded-body-capabilities.md)).
It refuses to start as root, binds a per-UID socket its systemd instance created, and spawns that
account's login shell from the passwd database rather than a guess.

Nothing inside the terminal is filtered, and that is the decision rather than an omission: command
filtering on a real shell is theatre, and a filter that can be defeated is worse than an absent one
because it is believed. The boundary is the account, held by the kernel, as it is for SSH. A terminal
is therefore never a route for Action1 operations — doing one by hand is a person acting with their
own authority, and the Journal records that differently, because the host did not do it.

What is here instead is bounds, each on something a terminal can make unbounded. A frame's declared
length is refused before anything is allocated for it. Unread output past four mebibytes ends the
session rather than being held or silently dropped — held bytes are this host's memory and dropped
bytes are a terminal that lies about what it printed. A window size outside 1000 by 1000, or zero in
either direction, is refused rather than clamped: zero columns is a browser that has not measured
itself, and programs divide by it. A session with no input and no output for four hours is closed,
which collects the tab that was shut without the socket noticing. The shell never outlives the
connection.

The frames are exercised against a real pseudoterminal and a real `/bin/sh`, not a mock: `test -t 0`
answers from the kernel, and `stty size` reports back the window the browser said it had. That is the
whole difference between this and the sandboxed Safe Shell, so it is asked of the kernel rather than
assumed from the fact that a crate was called.

`cybou-ptyd@.service` ships **disabled** and no deployment enables it. Enabling it is an act naming
one account, because the set of people who may read a projection and the set who may run programs are
not the same set. Where no instance is enabled the capability is absent and says so; it does not fall
back to the sandboxed shell, since a person who believes they are on the host and is not would run
the right command in the wrong place.

`GET /api/v1/terminal` upgrades to a WebSocket and carries one session between a browser and that
account's owner. It is the gateway's first bidirectional surface and deliberately the thinnest one
that can exist: it parses no frame, knows what no keystroke means, and decides nothing about what
may run. It supplies the single fact neither end can establish for itself — which Linux account is
at the keyboard — taken from the numeric identity the privileged helper established, never from a
name in the request.

Frames are re-framed rather than re-encoded. A WebSocket message already carries a length and the
owner's socket needs one, so the difference between the two ends is four bytes of prefix; decoding
here would put a second parser on the path with nothing to add and its own opinions about malformed
input. The frame bound is read from the protocol rather than restated, because a gateway that
accepted a larger frame than the owner would leave a gap in the boundary exactly the width of the
disagreement.

A refusal happens before the upgrade, not after. A socket that opened and then closed would look
like a terminal that crashed, and the difference between *this host has no terminal for you* and
*your terminal died* is what tells a person whether to ask an operator for access. A seat without a
Linux account behind it — the local desktop seat holds none — is refused rather than given a guessed
account, because the guess would be choosing whose shell somebody gets.

The browser keeps a real screen rather than a span parser. `living_canvas::terminal` holds a grid
of cells with a cursor, fed bytes and read back as rows, because the ANSI renderer beside it has
no cursor and no memory of where anything was put: a carriage return, a backspace and every
program that repaints come out of it as escape sequences flattened into a stream, which is the
thing ADR-0047 says the Safe Shell already is. Colour is emitted as `var(--term-N, ...)` so a
theme can override any of the 256 indexed colours, and a cell that set nothing carries no colour
at all rather than a black this theme never chose. Bytes are fed as bytes: decoding to text first
would lay replacement characters out as though a program had drawn them.

The screen is proven where it can be, which is natively: cursor addressing puts a character where
the escape sequence says, a carriage return overwrites rather than appends, a backspace removes,
a resize changes both dimensions, and invalid UTF-8 does not derail the grid. Nothing is
persisted, because a terminal buffer is the likeliest place for a password typed at a prompt to
reach a browser profile: the scrollback is held in the tab and nowhere else.

The Terminal card joins them. It is a separate card kind from the Shell rather than a mode of it:
one is a bounded read-only surface every deployment serves, the other runs programs as a person
and exists only where an operator enabled it, and a single card that quietly became either would
leave somebody unsure which one they are typing into. It is in the Dock and the command palette
beside the Shell, and its session is held in the card state rather than the component, so closing
the panel and reopening it finds the same shell — closing a card is a presentation act and must
not end a process.

Keys are turned into bytes rather than sent as names, which is most of what a terminal is: Ctrl-C
is `0x03` and not the letter `c`, the arrows are escape sequences, Enter is a carriage return,
Backspace is `0x7f`, and Alt is an escape prefix. A key this card handles is consumed, so Ctrl-C
does not also copy a selection and Tab does not leave the panel while completing a filename. That
mapping is in the portable module beside the screen and is checked on the native target, because
every one of those keys does nothing useful if it arrives as its own name.

The panel is measured and both ends are told. A terminal fixed at eighty by twenty-four inside a
panel somebody has dragged wider would have every program laying out for a screen that is not there:
`top` drawing a quarter of it, `vim` leaving the rest holding whatever was underneath. One cell is
measured rather than assumed, because a monospace cell's size comes from the font the browser
actually resolved and changes with the theme, the zoom and the platform's fallback. The arithmetic is
in the portable module and is checked natively, including that every size it can produce is one
`window_is_possible` accepts — so the browser never asks for a window the owner would close the
session over. It runs on a short timer rather than a resize observer, because a card is resized by
this desktop's own interaction code without the element firing anything a browser calls a resize, and
a measurement that changed nothing sends nothing: a resize frame reaches `TIOCSWINSZ` and every
program in the session gets `SIGWINCH`.

**No browser has driven one end to end.** The card compiles and its logic is covered natively;
the socket, the keyboard and the grid have not been exercised in a real browser, because the
browser gate needs `chromedriver` and this workspace has none installed. The vertical is built
and unproven at its last inch.

## Agent runtime

`cybou-agentd serve` owns `org.cybou.Runtime.Agent1`. It recovers still-running capsules from their
lease and launch records plus the service manager's answer, exposes `Sessions`, `Session`, `Launch`
and `Stop`, and admits new sessions atomically against operator-selected whole-host limits. Deployment
starts with an empty profile catalogue and zero capacity, so reachable launch is fail-closed until an
operator chooses both.

One accepted launch becomes one lease, capsule specification, optional model gateway and ACP prompt
turn. The OpenCode 1.18.23 pack is digest-pinned and uses its official `opencode acp` entrypoint; the
credential-free live capsule/ACP handshake has passed. A real provider answer has not: provider
policy and credentials remain an explicit operator decision, and their absence is reported as
`NOT RUN` rather than success.

Confirmed endings release live admission and retain the most recent 32 canonical final views in the
owner process. This is operational context, not durable biography. After an owner restart the host
can recover what is still running but cannot honestly reconstruct why an already-gone unit ended.

## Model brokerage

`org.cybou.Faculty.ModelBroker1` selects a route, enforces a budget, puts a request and attributes
what comes back. It holds no biography, reads no Journal, touches no filesystem, authorizes nothing
and executes nothing.

- A task is a closed set, and every task answers for its own absence: something deterministic already
  does it, or the feature is **absent** — never silently degraded.
- No `ModelOutput` variant asserts a fact or names an action. The strongest thing a model can return
  is a candidate something else must accept.
- Attribution is by artifact digest and template version, not by configured name.
- A request names the disclosure its input was drawn from, in a field that is not optional.
- Route selection is by declaration order, so the same request always chooses the same worker.

External agents now have a separate OpenAI-compatible `/v1/chat/completions` router in
`cybou-model-gateway`. It does not turn arbitrary agent prompts into a `ModelTask` and does not call
the D-Bus surface. Instead, both neighbouring request shapes meet at the same registered provider
worker, route policy and bounded usage ledger. Every agent completion is attributed to capsule,
agent, task, registered worker and provider. A proxy worker additionally records its model group,
concrete deployment id, response model and call id; it does not mislabel a remote model name as a
locally verified artifact digest.

The gateway accepts only an unpredictable ephemeral bearer minted from a live capsule lease. The
token is scoped to that capsule's agent and model class, a task, route sensitivity, lifetime, token
ceiling and the lease's spend ceiling. Revocation and expiry take effect on the next request; input
and maximum output are reserved before a worker runs, provider-observed usage is checked against the
reservation, and successful usage is charged by the gateway rather than reported by the agent. The
HTTP router is complete and tested but opens no listener by itself; the first agent pack owns binding
it to the capsule-only endpoint and injecting the token.

`cybou-provider-litellm` is the first replaceable provider worker. Capability classes map to
operator-owned LiteLLM model groups rather than compiled provider names. Its proxy master key never
enters a capsule: each completion receives a separate five-minute virtual key scoped to one model
group, the request's remaining budget and one parallel request. Proxy-observed decimal cost is
rounded upward into whole operator units; missing cost or attribution is a refusal. The blocking
HTTP adapter runs outside the async gateway executor. A deployable registration additionally
requires LiteLLM's database-backed budget reservation to be enabled and every mapped route to have
known token pricing, so `max_tokens` can be priced and reserved before provider dispatch.

`cybou-agent-gateway` closes the host-side lifecycle gap for an external-agent model lease. One
process owns one private Unix listener and one ephemeral token file, registers one configured
LiteLLM worker, and binds its bearer to the capsule, task, class, lifetime and ceilings. Its systemd
template is deliberately not boot-enabled: root-owned provider policy and credential plus a
short-lived per-launch lease file are required before an instance can exist. The fake-provider gate
proves the complete socket/token/worker path; a real provider remains an explicit B7 live gate.

**No inference runtime or LiteLLM service is deployed, and no real model has ever been called.** The
worker is exercised against a fake HTTP proxy. On an installation with no registered worker, every
request is answered with what happens instead. That remains a supported configuration.

Provider availability, zero-cost access and material terms now have a separate data boundary in
`cybou-provider-catalogue`. The compiled catalogue is empty. External schema-v1 entries carry
independent UTC observation and expiry times plus HTTPS evidence for availability and zero-cost
claims; warnings about data use, payment methods, geography and quota are source-backed the same
way. Stale claims remain visible but are ineligible for routing. Operator policy supplies the only
preferred and alternative provider names, and resolution reports a fallback as `NamedAlternative`
rather than silently relabelling it as the preferred route. See the
[provider catalogue contract](provider-catalogue.md).

## Action boundary

**A proposal a person was asked about can now be answered.** With no standing policy — the default,
and the only state a fresh installation has — every proposal decides to `RequiresUserConfirmation`
and stopped there, because nothing could carry the answer back. `ActionCore::confirm` mints the
permit that follows a yes, and refuses four ways: the verdict must still be the one that asked, so a
denial cannot be confirmed into existence and one agreement cannot mint two permits; the decision the
person saw must be the decision that is here, so a proposal re-decided between being drawn and being
clicked cannot have one question's answer authorize another; every criticism must have passed,
because confirmation grants the half a person owns and does not revive what the critics objected to;
and the proposal must be inside a fifteen-minute window, because it carries a diagnosis drawn from
readings that stop being true.

The four refusals share one error. Which of them was tripped is exactly what a caller would need in
order to keep trying, and a surface that reports how close a guess came is a way to search the
lifecycle for something confirmable.

`GrantedOnConfirmation` is deliberately not `Granted`. The two authorize the same execution and are
not the same authorization: one says a standing policy already covers this, the other says somebody
was asked, agreed, and is named. A record that could not tell them apart would answer *who
authorized this* with the policy, on a host whose policy authorized nothing. The seat is established
by whatever authenticated it and is never supplied by the party being authorized.

**And a person can now ask.** `Action1` had one entrance — a finding this host reached about
itself — so every proposal in the system was Mind's. That is right for remediation and it left
the desktop with fourteen `SystemHub` methods answering `Err(Refused)` beneath panels that were
complete except for the part where anything happens.

`ActionCore::request` is the other door ([ADR-0048](adr/ADR-0048-a-person-may-ask-for-an-action.md)),
and its shape is that **the asking is the confirmation**: somebody who read the unit name and
pressed the button has already answered the question a confirmation asks, and asking them again
teaches them to click through it. So a request that passes criticism is decided
`GrantedOnConfirmation` naming the seat, never `Granted` — no policy granted this, and the
record must not read as though one had.

It opens no new capability. `Operation::RestartService` was already in the closed table and
`ExecutableAction::ServiceRestart` already had an adapter, both proven end to end by two gates;
what was missing was a proposer, since `Proposer` had `Mind` and `Agent` and a person is
neither. What is forbidden stays forbidden: a person asking does not make formatting a
filesystem askable, for the same reason it is not offered to Mind. A verb outside the table is
refused as not an operation rather than as an operation of unknown risk. Risk and reversibility
are taken from the table rather than from the asker, so a proposer cannot understate what it is
asking for. And a person brings no evidence, so `brings_its_own_evidence` answers false for
them and the proposal cites no cause — inventing a finding to point at would be this host
claiming it had concluded something.

`Action1.Request` carries it, and `POST /api/v1/system/services/action` reaches that. The
gateway supplies the seat and carries the permit to the executor, deciding nothing: a permit
names no operation, so a courier holding one cannot choose what it is for, which is the
property that lets the gateway carry it at all. What the browser gets back is the lifecycle
record rather than a sentence — a refusal is a record too, carrying the reason the boundary
gave rather than one the gateway composed.

**One of the Services panel's six buttons does something.** Restart is in the closed
operation table with an executor adapter behind it; start, stop, enable and disable are not
in the table at all, and reload is in it with no adapter. Each of those five is refused by
name at the gateway rather than proposed and refused three layers down, because a refusal
that says what is missing is worth more than one that arrives from somewhere else. Opening
the door did not furnish the room, and the other thirteen `SystemHub` refusals are unchanged.

**A browser can now answer.** `POST /api/v1/actions/confirm` carries a person's answer to
`Action1.Confirm` and is the first write the gateway makes against the action boundary. It remains
unable to execute anything and decides nothing: it supplies exactly two things Action1 cannot know
— who is at the keyboard, and that they hold a seat entitling them to be asked — and the request
contract carries only two identities, the proposal and the decision that was on screen. There is no
field on it for an operation, a target or an argument, so a confirmation cannot become a request and
the proposal Action1 holds stays the thing that gets carried out. The permit the answer mints is
dropped where the reply is decoded and never crosses to the browser that prompted it.

Both the seat name and the refusal are single. `authenticated_principal` now lives on the gateway
state rather than beside the drafts it used to name alone, because it names an authorization too and
two definitions of *who is at the keyboard* that could drift is exactly the defect this codebase
keeps finding. And every way an answer can fail — a stale decision, a spent verdict, an objecting
critic, a proposal older than its readings, an unreachable owner — is reported as one
non-retryable `409`, because Action1 deliberately does not say which of its checks refused and a
gateway that split that into distinct statuses would say it on Action1's behalf.

The Insight card draws the control from the `ActionRecord`, not from the offer beside it: an offer
is the gateway's own recomputation of what could be proposed and carries no proposal identity, and
the record is the only thing there is to answer.


The host observes itself, concludes, explains, offers, and refuses.

A proposer **cannot choose its own risk**: operations are a closed set and risk and reversibility are
functions of the operation. Reversible does not mean harmless; irreversible does not mean forbidden.

Critical operations — deleting a service's data, formatting a filesystem, powering off — are never
offered and are refused if something else builds them. A standing policy cannot grant what the
operation table forbids, and a failed critic stops a pre-authorised operation.

**Nothing is granted on an installation nobody has configured.** The executor exists, but without a
granted Action1 decision it has no operation to perform: its public request contains only an opaque
permit identity and a missing, expired or consumed identity is refused.

`cybou-remediationd` can carry an eligible finding through proposal, criticism, standing policy,
single-use permit, execution and independent re-observation. It acts only when `Action1` can prove
the cause belongs to the episode; an owner lookup failure grants nothing. A completed episode whose
finding is still present remains completed evidence and is not silently retried after either daemon
restarts.

## Living Canvas and Spatial Desktop Operating Model

The CYBOU Living Canvas ([ADR-0037](adr/ADR-0037-web-first-presence-and-desktop.md), [ADR-0040](adr/ADR-0040-spatial-card-desktop-and-bounded-body-capabilities.md), [ADR-0044](adr/ADR-0044-cybou-spatial-desktop-architecture.md), [ADR-0045](adr/ADR-0045-cybou-core-desktop-pack-and-workspace-primitives.md), [ADR-0046](adr/ADR-0046-cybou-spatial-desktop-operating-model.md)) operates as a spatial operating system compiled in Rust/WASM (Leptos):

- **Infinite Canvas & Spatial Decks**: 2D infinite workspace featuring pan, zoom, snapping guides, Deck merging/splitting, monotonic z-indexing, and local presentation history (undo/redo).
- **Workspace Primitives & LocationRef**: Typed filesystem authority domains (`Draft`, `HostUserPath`, `SystemConfigPath`, `AgentWorkspace`, `SafeShellJail`, `BackupSnapshot`). File reads now carry gateway-issued authority-domain references into the Editor; browser path spelling is not used to infer authority. Direct mutation of privileged paths is prohibited; the future write path must route through Action1, but Editor-to-Action1 submission is not connected yet.
- **HostUserPath owner transport**: Successful PAM authentication returns owner-established UID/home metadata internally, and the gateway carries it in the server-side session without disclosing it to the browser. Separate authenticated host-file list/read routes accept only clean absolute paths within that home. `cybou-host-filesd` is a Linux-only, root-refusing per-user owner with bounded CBOR requests, bounded UTF-8 reads, jailed path resolution and owner-issued `HostUserPath` projections. When `CYBOU_HOST_FILES_SOCKET_DIR` is set, the gateway addresses `<directory>/<authenticated uid>/owner.sock`; otherwise the capability remains fail-closed with `hostUserFilesystemUnavailable`. `cybou-host-filesd@.service` runs one explicitly enabled instance under the named account, creates a numeric-UID runtime directory, and grants the `cybou` gateway group access to only that instance's socket after it has bound.
- **Core Desktop Pack**: Multi-tab Text Editor with active line/column cursor positioning, in-editor Search & Replace (<kbd>Ctrl</kbd>+<kbd>F</kbd> / <kbd>Ctrl</kbd>+<kbd>H</kbd>), and safe structured Markdown preview; standalone Universal Diff Viewer; sandboxed File Manager with interactive path breadcrumbs, instant in-directory name filtering, multi-criteria sorting (Name, Size, Type), exclusive file creation dialog, and rich preview metadata (SHA digest, human-readable sizes, authority badge); and bounded Safe Shell with zero-unsafe ANSI SGR color/formatting rendering, terminal session toolbar (Clear, Copy All), command execution status badges with per-entry output copy, and quick starter suggestion chips. Files opened from the Safe Shell jail can be conditionally saved with an owner-issued location, expected SHA-256 conflict check, bounded write, and post-write verification; host and system locations remain unwritable. A stale write is stopped, the current server version is re-read under the same authority reference, and a read-only Diff Viewer compares it with the preserved editor buffer. Save remains disabled until the person either explicitly adopts the verified server version (with destructive confirmation) or accepts its digest as the base for a subsequent conditional save; the viewer claims no commit action.
- **Editor buffer safety**: Closing a dirty or conflicted tab requires explicit destructive confirmation and states that no file is changed. The same guard covers closing the entire Editor panel when any tab is dirty or conflicted. Closing the final clean tab replaces it with a new editor-local draft; every new draft has a distinct identity rather than aliasing every `untitled` buffer.
- **Files → Editor admission**: Opening the same owner-issued `LocationRef` again focuses its existing tab and never replaces that tab's local contents. The first file replaces only a pristine empty draft; otherwise it opens in a new tab. The Editor panel is brought forward and selected.
- **Refresh boundary & Draft Recovery**: Browser navigation, refresh, or tab closure triggers the platform's unload confirmation while any editor instance holds a dirty or conflicted buffer. File contents are strictly kept out of `localStorage`. Unsaved and file-backed editor buffers are debounced per-tab and persisted to a durable, user-scoped SQLite store outside the file jail (`$XDG_STATE_HOME/cybou/drafts.sqlite3`), partitioned by authenticated seat (`linux-account:<user>`). On desktop reload, drafts are restored at desktop bootstrap with server-established authority and verified against current file digests, discovering conflicts without trusting stale browser state.
- **System journal**: `GET /api/v1/system/logs` reads the real systemd journal through `journalctl
  --output=json`, with a unit filter, a syslog-severity floor, and a bounded substring search. The
  search is applied here rather than handed to `--grep`, whose argument is a regular expression: a
  search box wired to a regex engine on the host lets a browser choose how long the journal spends
  answering. Severity is a closed set of the eight syslog names and an unrecognised one is refused,
  because a filter that silently stopped filtering would answer *show me the errors* with
  everything and look like a quiet host. An empty feed says which kind of empty it is: a journal
  that matched nothing, or one this reader could not run, could not find, or was refused by. And
  `journalctl` does not fail for a reader outside the `systemd-journal` group — it narrows to that
  account's own entries — so the projection carries whether the whole system journal was visible,
  and the card says so rather than drawing one service's half of the host as the whole of it.
- **File transfer**: `POST /api/v1/files/upload` and `/api/v1/files/download` move bytes in and out
  of the Safe Shell jail, bounded at `FILE_TRANSFER_MAX_BYTES` (8 MiB) in either direction. Separate
  routes rather than a mode of the text ones: `read` and `write` answer *what does this file say*
  and refuse anything they cannot decode as UTF-8 within a panel-sized budget, and an image is not
  unreadable, it is not text. An upload creates exclusively and is refused with `409` when the
  destination already holds something — silently replacing it would make losing a file
  indistinguishable from placing one. The bytes are read back and compared before success is
  reported. A download is served `application/octet-stream` with `X-Content-Type-Options: nosniff`,
  and its `Content-Disposition` carries the name twice: an ASCII-filtered quoted form that cannot
  close its own parameter, and a percent-encoded `filename*` that cannot be read as header syntax at
  all. The File Manager shows Upload and per-file Download in the sandbox domain only; the home and
  agent-workspace domains are served by an owner that carries bounded UTF-8 reads and no transfer, so
  the buttons are absent there rather than present and failing.
- **Every card the Dock can open now draws**: the viewport named each card it could render, one
  component at a time. Fourteen singletons and six dynamic kinds had one; twenty-one others —
  Services, Processes, System Logs, Storage, Network, Packages, Updates, Users, Security,
  Backup, Mail, Calendar, Notes, Contacts, the Cognitive Graph, the Event Journal, Meaning,
  Learning, Operations, Notifications — did not, and were reachable from the Dock and the
  command palette regardless. Opening one added it to the layout, moved the selection onto it,
  saved, and drew nothing: not an error and not an empty panel, but nothing at all on a desktop
  that had just been told to open it. The same card tabbed into a Deck drew perfectly, because a
  Deck has always rendered whatever `CardContent` can dispatch, which was every kind all along.
  A card with no component of its own is now drawn by the generic frame, and which kinds have
  one is a question the card answers about itself, so it is checked on the native target rather
  than being a fact about a file somebody has to remember.
- **The desktop has a palette rather than 886 colour literals**: colours were written into Rust
  source at the call site, 886 of them against five tokens, and the two tokens components asked
  for most — `--bg-card` and `--text-main` — were never defined at all, so every one of their
  forty-three call sites drew its fallback. A fallback is exactly what a theme cannot change.
  Forty tokens now carry what was repeated, named for what each colour is for rather than what
  it is: `--fill-subtle` survives a decision to make it warmer, where `--white-06` would have to
  be renamed or start lying. Each was set to the literal it replaced, so the change is invisible
  on screen and 738 of the 886 literals are gone. A light desktop follows from redefining the
  tokens under `prefers-color-scheme`, and is deliberately overridable by an explicit choice: a
  person who has set one has said something, and the system preference is only a guess about
  them. What remains in source is one-off colour, not repetition.
- **A card that could not be read says so**: two panels wrote why they had failed into a signal
  no view rendered. The Editor put twenty-eight messages there — a draft autosave that failed, a
  save the host refused, a conflict re-read, the authority a file was admitted under — and the
  System Monitor put `Failed to load telemetry` there, so a host whose gateway could not be
  reached drew an empty Monitor, indistinguishable from a machine doing nothing. Both render
  now, and an unread projection says it is unread rather than drawing a host with no memory and
  no disks. `scripts/validate-card-signals.py` refuses a panel that writes a message it never
  reads; a file that writes into another card's state is listed by name with its reason, so the
  exemption cannot widen quietly. The check is per file, which is the whole of why it works — an
  earlier version asked whether the name was read anywhere in the crate, and since every card
  has a `status_msg` it passed on both defects it was written for.
- **The desktop is installable, and it is no longer sent whole**: the frontend had no manifest,
  no theme colour and no icon beyond a favicon, so a browser had nothing to install and painted
  its own chrome from a default. It declares both colour schemes now, since the palette has a
  light half, and ships a maskable icon — the aperture already fits the safe zone, so what the
  variant adds is an opaque background, because a cropped icon has to be opaque to its own edges.
  More consequentially, the gateway was serving 8 041 363 bytes of WebAssembly with no
  compression at all: measured against a running gateway, the same module is 2 048 707 bytes
  gzipped and 1 697 033 with Brotli, so four fifths of a cold load was bytes nobody needed to
  send. Compression wraps the file service rather than the router, which keeps it off the two
  paths where it would harm: `/api/v1/events` is an event stream that a compressor filling a
  block would delay, and `/api/v1/terminal` is a socket upgrade with no body to compress.
- **A screen reader is told what the desktop did**: the crate rendered 128 messages and carried
  no `aria-live` at all, so a panel that had just refused a write, lost its connection or
  finished a replace changed in silence for anybody not looking at it. Twenty live regions now,
  polite where the message answers something the person did and assertive where the panel has
  nothing else to show — interrupting somebody to say a listing refreshed is worse than waiting
  for a pause, and holding *this host has no terminal for you* until they stop typing is worse
  than interrupting. The Editor's own footer is `aria-hidden`, because reading a line and column
  aloud on every keystroke makes an editor unusable with a screen reader rather than usable.
  `validate-card-signals.py` now refuses a file that renders a message with no live region in it,
  and found five the first time it ran, one of them in a card written the same day.
- **A dialog says it is one**: ten modals — sign in, five in the Editor, four in the File Manager
  — were plain divs. Nobody was told the desktop had asked them something, and a keyboard could
  tab straight past the question into the canvas behind it, where the buttons still worked. Each
  carries `role="dialog"`, `aria-modal` and the heading already inside it as its name.
- **The canvas answers to fingers**: the cards declared `touch-action: none` and the plane they
  sit on did not, so a one-finger drag on empty canvas was a page scroll and panning by touch did
  not work at all. And there was no way to zoom: the wheel gesture wants a modifier key, so a
  person on a tablet could pan an infinite canvas and never change how much of it they could see.
  A two-finger pinch scales about the point between the fingers and follows that point as it
  moves — doing only the first makes the canvas slide out from under the gesture — and the scale
  is applied after clamping, or a pinch past the limit keeps translating while the size stays
  put. The arithmetic is in `layout::camera` and checked natively, including that the canvas
  point under the fingers is the same one after. Fingers in the same place change nothing, since
  dividing by that distance sends the canvas to infinity on the frame two fingers touch. A third
  finger is ignored: that is somebody resting a hand. `overscroll-behavior: none` on the body,
  because pull-to-refresh on a canvas is a gesture that discards every unsaved buffer on it.
  SD14's mobile layout — cluster stack views for a narrow viewport — is not started.
- **The frontend's lint output means something again**: `living-canvas` carried 310 warnings, and
  120 of them were one systematic false positive — `pedantic` asking every Leptos `#[component]`
  to be `#[must_use]`, when the `view!` macro that calls one is the only thing that ever does.
  Silenced at the crate root with its reason rather than answered attribute by attribute, since
  an earlier mechanical attempt wrote it twice on every component and failed the build on
  `duplicated_attributes`. With the noise gone the rest could be read and fixed: 310 to 108,
  every remaining one measured rather than assumed. What is left is 23 byte-count casts to `f64`,
  exact to four pebibytes and harmless for a file size, and the structural ones — 35 functions
  over 100 lines, of which `file_manager` at 1132 and `editor` at 1004 are genuinely too large
  and are recorded rather than silenced.
- **A file is recognised whatever case its name was written in**: opening one in the Editor chose
  its language from a chain of `ends_with` comparisons that each had to remember to lower-case
  and none did, so `README.MD` from a system that does not care about case, or `SETUP.PY` out of
  an archive, opened as plain text. It is a table now, in the portable half and checked there,
  including that only the last dot decides — `notes.rs.bak` is a backup and not Rust.
- **The two frontend fixes that live in the DOM are asserted there**: the generic frame and the
  culling both had native tests for their arithmetic and nothing for the thing they are about.
  Two browser tests now say it: a card kind with no component of its own is drawn *with its own
  panel inside it*, and a card panned ninety thousand pixels away keeps its `.object` frame
  while its contents are not built — which is what the minimap, hit-testing and the tests that
  click a card by index all depend on. Seven browser tests, run in headless Chromium.
- **What a browser is handed is what was built**: `test-desktop-delivery-gate.sh` builds the
  frontend with trunk, serves it from the real gateway and asks for it the way a browser does.
  It checks that the index names a module the gateway will actually serve — an index naming a
  file that 404s is a blank page that says nothing — that the module begins with the WebAssembly
  magic number rather than being an `index.html` some fallback handed back, that it is compressed
  on the way out, and that the manifest arrives as `application/manifest+json` with every icon it
  names. Measured on a passing run: 8 091 716 bytes identity, 2 060 918 gzip, 1 706 298 Brotli.

  It is deliberately not a browser. The first version drove headless Chromium with
  `--virtual-time-budget`, because `--dump-dom` fires on the load event and a WebAssembly desktop
  has mounted nothing by then. That budget waits for a page to fall idle and this page never does
  — the Dock and the Terminal card each hold a repeating timer — so Chromium advanced virtual
  time and fired them for two hours and sixteen minutes before it was killed. What a browser does
  with the bundle is covered by the seven `wasm-bindgen-test` tests, which settle on a microtask
  rather than on an idle page.
- **A narrow window gets a stack rather than a plane**: below 760 pixels a panel is wider than
  the screen it is on, and a spatial desktop is asking somebody to pan sideways to read a
  sentence. The cards leave their coordinates and become one column in the order the layout
  holds them, which is ADR-0044's cluster stack view at its simplest: not a smaller canvas,
  because pan and zoom mean nothing once everything is already as wide as the window, so the
  transform is dropped and the column scrolls. The threshold belongs to the canvas — a window
  exactly that wide keeps its plane, since stacking one that fits takes the desktop away from
  somebody who could use it — and a window nobody has measured is not a narrow one, or every
  panel would stack on the first frame and unstack on the second in front of the person.
  Culling is off on a stack, which is the part that would have shipped wrong: it reads where a
  card would be on a plane, and in a column that is nothing, so a panel the layout happens to
  hold at ninety thousand pixels is simply the next one down. A browser test holds that.
  The Dock adapts with it — full width and scrolled sideways rather than a centred pill, since
  thirty-one items at thirty-four pixels do not fit a telephone and a pill that overflows puts
  half of them past both edges with no way to reach them. It asks the same rule the canvas
  asks, which is why the camera is provided by `App` rather than by the viewport: the Dock is
  the viewport's sibling and could not otherwise see it.

  The stylesheet had been stacking cards at `max-width: 760px` since before any of this and
  could only ever say so in CSS, so a narrow window got a column of cards on a plane that still
  panned and zoomed underneath them. The two also disagreed by a pixel — `max-width: 760px`
  includes 760 and the rule excludes it — so a window exactly that wide was stacked by the
  stylesheet and spatial to everything else. The stylesheet no longer decides; the media query
  keeps only what is about a small screen and nothing else.
- **Spatial Clusters Engine**: 2D bounding hull clusters (`DesktopCluster`) visually grouping related cards with contextual themes.
- **A card nobody can see is not built**: ADR-0044 named this as the cost of an infinite canvas
  and nothing implemented it, so every panel stayed in the DOM however far it had been panned
  away — and these are not idle markup, since each holds signals that update on a timer. The
  stage is drawn as `translate3d(pan) scale(zoom)` from its top left, so where a card reaches
  the window is arithmetic; it lives in `layout::camera` rather than in the component and is
  checked on the native target. What is dropped is the card's contents. The frame stays, which
  keeps the card where the layout says it is, keeps the minimap and hit-testing honest, and
  keeps a card the browser gate clicks by index still there to click. The margin is wide,
  because the cost this saves is drawing something nobody sees and the cost it risks is a panel
  arriving late as somebody pans towards it — and the second is the one a person notices. An
  unbelievable camera draws everything: a zoom of zero, a window nothing has measured yet, an
  infinity from somewhere upstream. A card must never be hidden because a number arrived wrong,
  so the failure of this is drawing too much.
- **Command Palette & Omnibar Navigation**: Unified desktop action launcher (<kbd>Ctrl</kbd>+<kbd>K</kbd>) with real-time fuzzy search, keyboard selection cycling (<kbd>↑</kbd>/<kbd>↓</kbd>), <kbd>Enter</kbd> execution, Ask CYBOU cognitive question answering, shortcut badges, and automatic uncollapse/bring-forward card focusing.
- **Decoupled Lifecycle & SubjectRef**: Cards act as projections into system entities; closing a card never terminates the underlying system process, agent, or account. `SubjectRef` is the canonical primitive for new cross-panel entity references; migration of existing interactions is incomplete.
- **Universal Entity Inspector & Deep Links**: Dedicated introspection panel for canonical `SubjectRef` entities featuring categorized preset pickers (Services, Files, Agents, System), custom target entry, 1-click copyable `cybou://` canonical URIs and deep link hashes, formatted raw JSON spec viewer, and epistemic honesty regarding authoritative projection status. Complete service, package, mail, and calendar subject hashes open and focus the Universal Inspector on initial load and subsequent `hashchange`. Identity-only links whose metadata or filesystem authority requires an owner lookup are refused rather than completed with browser-invented values; that resolver remains unbuilt.
- **Canvas Outline & Workspace Tree Navigator**: Non-spatial hierarchy explorer and accessibility navigator ([ADR-0046](adr/ADR-0046-cybou-spatial-desktop-operating-model.md) §22, §29) with real-time text search, workspace statistics chips (Cards, Decks, Anchors), card state controls (1-click collapse/expand toggle, bring forward/focus, close), deck splitting, and complete camera anchor management (create at viewport, rename, delete, fly-to).
- **Live Presence SSE Stream**: Gateway SSE consumer (`/api/v1/events`) for bounded Presence1 snapshots and explicit projection errors. This is not the canonical Event1 Journal. The UI reports actual connection state, supports snapshot/error filtering, search, pause/resume, a bounded ring buffer, and JSON inspection.
- **Resilience Acceptance Criteria (SD1–SD15)**: Formal invariants define the required offline honesty, crash isolation, secret handling, and hostile-content boundaries. Dedicated executable coverage for the complete SD1–SD15 set is not yet implemented.

## Current gates

Every one of these runs on Debian 13 and in CI, and all pass:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test -p living-canvas --target wasm32-unknown-unknown --locked
bash scripts/test-multi-daemon-integration.sh
bash scripts/test-capsule-gate.sh
bash scripts/test-egress-gate.sh
bash scripts/test-action-gate.sh
bash scripts/test-confirmation-gate.sh
bash scripts/test-desktop-delivery-gate.sh
bash scripts/test-acp-gate.sh
bash scripts/test-standing-lease-gate.sh
bash scripts/test-model-gateway-gate.sh
bash scripts/test-litellm-worker-gate.sh
bash scripts/test-provider-catalogue-gate.sh
bash scripts/test-agent-session-gate.sh
bash scripts/test-agent-runtime-gate.sh
bash scripts/test-opencode-pack-gate.sh
bash scripts/test-self-maintenance-gate.sh
python3 scripts/validate-cognitive-docs.py .
python3 scripts/validate-card-signals.py
python3 scripts/validate-desktop-styles.py
python3 scripts/validate-organ-layering.py
python3 scripts/validate-doc-links.py
reuse lint
```

`unsafe_code = "forbid"` and `clippy::pedantic` are workspace-wide.

There is currently no language-model process. The action services are deployed, but their standing
policy is empty by default; the live gate uses host authority only against a disposable unit.

## What is not built

Stated as absences rather than left out, because a capability nobody built and a capability nobody
mentioned look identical to a reader.

| | |
|---|---|
| Inference runtime | no local or remote model worker exists; the brokerage contract has nothing behind it |
| General agent sessions | one digest-pinned OpenCode pack and one ACP prompt turn exist; multi-turn streaming, further packs and real-provider evidence do not |
| Native desktop session | `cybou-desktop.service` is built and ships disabled; it has never run on a machine with a seat |
| A terminal proven in a browser | owner, transport, screen and card all exist and are covered by native tests; no browser has driven one end to end, because the browser gate needs `chromedriver` and this workspace has none |
| Sensitive payload storage | the AEAD primitive, key store and erasure protocol exist and are tested; no payload is encrypted and no perception source is sensitive |
| Automatic retention expiry | retention classes are carried; nothing acts on a lifetime |
| Semantic file index | not started |
| Backups | `BackupState` reports `NoneDeclared`, which is true: no backup software or rotation is configured |
| Inter-node transport | no replication, no partition handling, no distributed anything |
| Learning promotion in practice | the gate is implemented and evaluated; no candidate has ever been promoted |

## Known limitations of what is built

- **Telemetry has no persistence.** Everything it holds is in memory and bounded; a restart starts
  the window again, and the organ says it has not watched long enough rather than answering.
- **A confirmed action has run on a gate host, not on a deployment.** `test-confirmation-gate.sh`
  carries one from a finding through a question, an answer, a permit, a systemd restart and an
  independent re-observation, against a disposable unit and a Journal of its own. No deployment has
  carried one through from a finding its own telemetry reached, and no browser has been the thing
  that answered — the gate stands in for the gateway, because it is the only party that can
  establish a seat.
- **The agent vertical has no real-provider evidence yet.** The real OpenCode ACP entrypoint has run
  inside a capsule and completed the credential-free handshake, but no configured provider has
  returned an answer through the full path. Multi-turn streaming is also not built.
- **The context projection has no checkpoint.** It replays from the Journal on every start. Correct,
  and slower than it needs to be on a long biography.
- **`Predictor1` is domain-neutral.** It forecasts a level — where a subject sits relative to its own
  history — which is not what an operator asks. The operational projections are computed in
  `Telemetry1`, where the series lives, and cover every watched subject that has a threshold at all,
  declared certificates and services included. What `Predictor1` still cannot answer is *when*: it
  says where a level is going, not when it arrives.
- **Continuity is proven across process restart, not machine reboot.** See
  [reboot continuity](evidence/reboot-continuity.md).
- **Nothing is proven under load.** The integration gate proves the system comes up and is coherent,
  not that it stays so under conditions nobody has applied. See
  [Debian integration](evidence/debian-integration.md).
