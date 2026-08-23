<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Threat Model

## Assets

Identity, Journal, intentions, private observations, provenance, epistemic state, retention/erasure
obligations, lifecycle records, Nix configuration, credentials, D-Bus interfaces, migration
backups, and future inter-node messages.

## Trust boundaries

- Body/perception adapter to accepted Observation;
- one user-session process to another over D-Bus;
- organ owner to lifecycle coordinator;
- Journal history to derived epistemic projection;
- local node to future trusted peer;
- cognition/planning to the authorization boundary and executor;
- actors, models and tools to the grant and broker boundaries;
- unattended response to standing security policy, which must hold with models unavailable.
- proposed browser session to `cybou-web-gateway`, over local loopback or remote TLS;
- proposed `cybou-web-gateway` to the typed Presence and Mind service interfaces.

## Threats

- Journal modification, insertion, deletion, or truncation;
- privacy weakening;
- false organ impersonation;
- replay of old messages;
- duplicate or invalid lifecycle outcomes;
- migration rewriting or partial completion;
- QML authority confusion and duplicate Presence instances;
- action escalation from uncertain cognition to privileged mutation.
- forged provenance or stale input presented as current observation;
- consolidation rewriting history or reporting false completion;
- sensitive content surviving through summaries, backups, or replicas after claimed erasure;
- value/priority scoring being mistaken for execution permission;
- resource exhaustion through event, contradiction, or consolidation backlog.
- script injection, unsafe browser dependencies, or compromised rendered content escaping its
  intended object boundary;
- cross-site request forgery, clickjacking, hostile origins, session theft, or replay of a local
  desktop bootstrap credential;
- one browser consumer receiving another consumer's private context, cursor, or delivery state;
- stale snapshots, resumptions, or caches presenting invalid state as current;
- sensitive state surviving browser storage after retention or erasure obligations apply;
- gateway connection, subscription, or mutation floods exhausting bounded Mind services.

## Controls

- single Journal writer;
- contribution origin bound to the calling process: eventd resolves the caller's executable and
  refuses any contribution claiming an organ identity that caller is not. The executable must both
  be named for the organ and sit alongside eventd's own binary, so a look-alike built elsewhere is
  refused; the trusted location is derived from eventd's own path rather than configured, because a
  configured one would be settable by anyone able to restart the service;
- canonical full-envelope hashing;
- stable D-Bus policy;
- message uniqueness;
- explicit capabilities;
- privacy inheritance;
- transactional migration;
- process isolation and explicit state ownership;
- typed action boundary;
- security and concurrency tests.

The proposed web-first Presence adds the following target controls. They are architectural gates,
not claims about the currently shipped Plasma surface:

- a dedicated `cybou-web-gateway` with an explicit, versioned HTTP/event contract rather than a
  generic D-Bus, shell, or filesystem bridge;
- loopback-only local binding, a short-lived single-use desktop bootstrap exchange, and a
  host/origin allowlist;
- authenticated remote sessions, secure and same-site cookies, CSRF protection, TLS, rate limits,
  and per-session capability and consumer identity;
- strict Content Security Policy, no inline script, dependency pinning, output encoding, frame
  denial, and isolation of untrusted rich content;
- server-authoritative privacy filtering, context allocation, authorization, and mutation
  idempotency; hiding a control in the frontend is never an authorization decision;
- cursor-based resumption with explicit reset on stale or unauthorized cursors, bounded queues,
  and visible degraded/stale state;
- browser storage minimization, privacy-class-aware cache policy, and erasure propagation tests;
- separate budgets for sessions, subscriptions, payloads, mutations, and downstream calls.

## Current limitations

- same-user D-Bus callers do not yet have capability tokens or method-level authorization. Event1
  now refuses organ impersonation, which closes forged provenance specifically; it does not
  constrain what a non-organ caller may contribute under a name of its own, and it is not a general
  authorization model;
- Journal hashing detects inconsistency but is not an external signature/trust anchor;
- retention and replica erasure are design targets, not implemented controls;
- lifecycle ownership, recovery, scheduling, and bounded transport are active enforcement paths;
  epistemic governance and the authorized-action boundary remain proposed;
- user-service hardening does not yet define a least-privilege filesystem/network sandbox for
  every daemon;
- the web gateway, Chromium desktop session, remote authentication, cache/erasure enforcement, and
  the web-specific controls above are proposed and are not present in the current Plasma/QML
  implementation;
- Mind binaries retain environment-triggered fault-injection hooks, which the reboot and
  split-commit gates set against the installed package. This is accepted rather than outstanding: a
  same-user process can already terminate any daemon with a signal, so the hooks grant no capability
  the same-user boundary does not already concede, and removing them would move the recovery
  evidence onto a binary that is not the shipped one.
