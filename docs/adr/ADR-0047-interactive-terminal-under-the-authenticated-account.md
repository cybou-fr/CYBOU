<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0047: An interactive terminal that runs as the person who signed in

## Status

Proposed

Supersedes the shell half of [ADR-0040](ADR-0040-spatial-card-desktop-and-bounded-body-capabilities.md).
Everything else in ADR-0040 — the spatial card model, the zone diagram, the Mind boundary, the
read-only Body observation — stands.

## Context

ADR-0040 bounded the shell surface to read-only builtins over one demonstration root, and said so
plainly: *Zone 3 (Shell) cannot execute arbitrary Zone 4 actions*. That is what
`cybou-shelld` implements. It answers `pwd`, `cd`, `ls`, `cat`, `echo` and `help` inside
`cybou-jailfs`, and it is not a terminal. It has no process, no pseudoterminal, no signals, no job
control, and no way for a program to know it is talking to a screen. `vim` cannot run in it. Neither
can `top`, `less`, `apt`, `git rebase -i`, or anything that draws.

[ADR-0045](ADR-0045-cybou-core-desktop-pack-and-workspace-primitives.md) put *Interactive Linux PTY
Terminal* in the Core Desktop Pack as CP7, and the two documents have disagreed ever since. This one
settles it rather than leaving the code to settle it at whichever call site needed it first, which is
how a boundary becomes a habit.

### What is actually being decided

Not *should the terminal be nicer*. The decision is whether a person who has signed in through the
browser may run programs on the host as themselves.

Saying yes is the largest single expansion of capability in this system. It is worth being exact
about what it does and does not change:

- **It grants nothing that account did not already have.** Every deployment of this system is
  reached over a network by a person who holds a Linux account and, in practice, an SSH key. The
  terminal does not create that authority. It provides a second door to it.
- **It is therefore a door, and doors are what get attacked.** The gateway is an HTTP surface
  reachable from a browser, and an authenticated session on it would become equivalent to a shell.
  Every weakness in session handling stops being an information-disclosure question and becomes a
  remote-execution one.
- **It does not extend to root.** The account is the boundary. A person who wants privileged action
  goes through Action1, which is the whole reason that boundary exists, and a terminal is not a way
  around it.
- **Mind is not in this path.** Cognition does not read the terminal, does not sit between a
  keystroke and the host, and gains no ability to run anything. A person is typing; the host is
  answering.

### Why not keep it in the sandbox

A pseudoterminal confined to `cybou-jailfs` was considered and rejected as the primary form. It
would deliver the fidelity — colours, cursor addressing, `Ctrl-C`, window size — while keeping
ADR-0040's boundary intact, and it is genuinely useful. It is not what CP7 is for. An operator
reaches a server to look at *that server*: its units, its disks, its packages, its logs. A terminal
that can do everything except reach the machine it is running on is a demonstration of a terminal.

The sandboxed form remains available and remains the default surface for anyone who has not been
granted an account instance, because a deployment that has not enabled a terminal for an account
must still have a shell card that does something honest.

## Decision

**A person authenticated to the gateway may open an interactive pseudoterminal that runs as their
own Linux account.**

The following constraints are part of the decision, not implementation detail. Each exists because
without it the door above becomes worse than an SSH port.

### One owner per account, and never root

The terminal is owned by `cybou-ptyd`, one instance per explicitly enabled Linux account, on the
model [`cybou-host-filesd@.service`](../../systemd/system/cybou-host-filesd@.service) already
establishes: the process runs as that account, refuses to start as root, binds a private per-UID
socket, and the gateway addresses `<directory>/<authenticated uid>/owner.sock`.

The gateway never spawns a shell. It cannot: it runs as `cybou` and has no business becoming
anybody. What it does is prove which account is at the keyboard and connect the two ends.

### Off until an operator turns it on, per account

`cybou-ptyd@.service` ships disabled. Enabling it for an account is a deliberate act naming that
account. There is no global switch that turns terminals on for everyone who can sign in, because the
set of people who may read a projection and the set who may run programs are not the same set and
must not be made the same by a default.

Where no instance is enabled, the capability is absent and says so, the way
`hostUserFilesystemUnavailable` already does. It does not silently fall back to the sandbox: a
person who believes they are on the host and is not would run the right command in the wrong place.

### The session belongs to the seat, and dies with it

A terminal is bound to the browser session that opened it. When that session ends — sign-out,
expiry, the socket closing — the pseudoterminal closes and the process group is signalled. A shell
that outlived the authentication that opened it would be an unauthenticated shell.

### Bounded in every direction that can be unbounded

Output is a stream from a program that may produce it faster than anything can consume it, so it is
bounded by rate and by backlog, and a session that outruns its backlog is closed rather than
buffered. Input frames are bounded. Sessions per seat and per host are bounded. Idle sessions
expire. None of these is a security control by itself; together they are what stops one tab from
being a way to spend the host.

### It is a terminal, and it is not an authorization boundary

Nothing about what runs inside the pseudoterminal is inspected, filtered, or approved. Command
filtering on a real shell is theatre — there is always another spelling — and a filter that can be
defeated is worse than an absent one because it is believed. The boundary is the account, enforced
by the kernel, exactly as it is for SSH.

Consequently a terminal session is **not** a route for Action1 operations and must never be used as
one. An action that requires authorization requires it because of what it does, and doing it by
hand in a terminal is a person acting with their own authority, which is a different thing that the
Journal records differently: the host did not do it.

### What the browser holds

Bytes on their way to a screen and keystrokes on their way to a program. No credential, no token
beyond the ordinary session cookie, and no persistent scrollback in `localStorage` — a terminal
buffer is the single most likely place for a password typed at a prompt to end up on disk in a
browser profile.

## Consequences

### Positive

- CP7 becomes buildable, and the Core Desktop Pack stops containing a tool that cannot exist.
- An operator can do on the desktop what they reach the server to do, without a second client.
- The bidirectional transport this needs is the same one multi-turn agent sessions need
  ([NEXT_STEPS B7c](../NEXT_STEPS.md)), so it is built once.

### Negative

- **The gateway becomes security-critical in a new way.** A session-fixation or CSRF weakness that
  was worth a projection is now worth a shell. This is the real cost and it is not mitigated by
  anything in this document; it is accepted, and it is the reason for the per-account enablement and
  the seat-bound lifetime above.
- **`unsafe_code = "forbid"` cannot allocate a pseudoterminal.** Neither `forkpty` nor a
  `pre_exec` that calls `setsid` and `TIOCSCTTY` can be written under it, so this brings in an
  external crate to do it. That is a supply-chain surface in the one component that spawns
  processes as a person, and it is the strongest argument the rejected sandbox option had.
- ADR-0040's zone diagram is now half-true and must be read together with this document.

### Neutral

- ~~The sandboxed Safe Shell does not go away and is not deprecated. It remains what a deployment
  serves where no terminal has been enabled.~~ **Reversed on 2026-09-01.** The Safe Shell has been
  removed: the card, the two routes, `cybou-shelld` and its unit.

  The argument for keeping it was that a host with no terminal enabled should still have a command
  surface that does something honest. What it produced was two cards a person cannot tell apart by
  looking, one of which answers six builtins inside a demonstration root and calls itself a shell.
  That is not honest; it is a thing shaped like a shell, and the confusion it causes is the same
  confusion this ADR refused to accept in the other direction — "a person who believes they are on
  the host and is not would run the right command in the wrong place".

  A deployment with no terminal enabled now says so, which is what the paragraph above this one
  already promised it would do. The seat identity that lived in the same module stays: telling a
  local desktop session from a network one is a different thing that was wearing the same word.

## Open

- Whether a terminal session should be recorded in the Journal at all, and if so what — that a
  session happened and for how long is biography; what was typed is not something this system should
  hold, and the two are easy to conflate into a keylogger with good intentions.
- Whether an enabled account may open a terminal from a public-preview deployment. Currently no
  seat means no terminal, which answers it for now by refusing.
