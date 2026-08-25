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

Beside them, and deliberately not among them:

| | |
|---|---|
| `cybou-model-brokerd` | `org.cybou.Faculty.ModelBroker1` — a faculty, owning no part of Mind |
| `cybou-web-gateway` | the HTTP boundary; not a Mind owner and holds no cognitive state |

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
action from Action1, so its caller supplies neither a verb, a program nor arguments. There are three
adapters: `service.status`, fixed `/usr/bin/apt-get clean`, and `service.restart`. Services use the
systemd manager D-Bus API and only concrete names ending in `.service`; the `systemd:<unit>`
placeholder and every operation without one of those adapters are refused before a permit exists.
The layering validator rejects any dependency from the governance owner to `cybou-executord`.

The A1 gate creates a harmless disposable systemd unit, observes it inactive, passes a strong named
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

## Desktop

One Rust/WebAssembly frontend — Living Canvas — served to browsers and, as a target, to a
Chromium/Wayland session. The browser is a renderer and an untrusted client: it talks only to the
gateway and never becomes a Mind owner, D-Bus peer, or authority.

Thirteen singleton system cards and dynamic tool cards. Arrangement follows a declared relationship
graph. Every class the components render has a rule, checked by
`scripts/validate-desktop-styles.py`; interaction is exercised in real Chromium. See
[desktop and browser gate](evidence/desktop-browser-gate.md).

A stranger is served the sign-in view and nothing else where the deployment says so.

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

**No inference runtime is implemented and no model has ever been loaded.** On an installation with
no worker, every request is answered with what happens instead. That is a supported configuration.

## Action boundary

The host observes itself, concludes, explains, offers, and refuses.

A proposer **cannot choose its own risk**: operations are a closed set and risk and reversibility are
functions of the operation. Reversible does not mean harmless; irreversible does not mean forbidden.

Critical operations — deleting a service's data, formatting a filesystem, powering off — are never
offered and are refused if something else builds them. A standing policy cannot grant what the
operation table forbids, and a failed critic stops a pre-authorised operation.

**Nothing is granted on an installation nobody has configured.** The executor exists, but without a
granted Action1 decision it has no operation to perform: its public request contains only an opaque
permit identity and a missing, expired or consumed identity is refused.

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
bash scripts/test-acp-gate.sh
python3 scripts/validate-cognitive-docs.py .
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
| Agent installation and session | ACP initialization and registry discovery exist; authentication, installation, `session/new` and prompts do not |
| Native desktop session | `cybou-desktop.service` is built and ships disabled; it has never run on a machine with a seat |
| Sensitive payload storage | the AEAD primitive, key store and erasure protocol exist and are tested; no payload is encrypted and no perception source is sensitive |
| Automatic retention expiry | retention classes are carried; nothing acts on a lifetime |
| Semantic file index | not started |
| Backups | `BackupState` reports `NoneDeclared`, which is true: no backup software or rotation is configured |
| Inter-node transport | no replication, no partition handling, no distributed anything |
| Learning promotion in practice | the gate is implemented and evaluated; no candidate has ever been promoted |

## Known limitations of what is built

- **Telemetry has no persistence.** Everything it holds is in memory and bounded; a restart starts
  the window again, and the organ says it has not watched long enough rather than answering.
- **Action lifecycle records are in memory.** A restart destroys unclaimed permits, which fails
  closed, but durable proposal/decision/attempt recording through Event1 is still to be wired.
- **Confirmation has no operator surface yet.** A decision may require confirmation and therefore
  produces no permit; only an explicit standing policy can currently reach unattended execution.
- **ACP has no live agent pack yet.** The client is proven against a protocol peer and the public
  registry is read live, but no registry distribution has been installed or run inside a capsule.
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
