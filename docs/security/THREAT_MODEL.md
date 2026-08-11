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
- cognition/planning to M9 authorization and executor.

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

## Controls

- single Journal writer;
- contribution origin bound to the calling process: eventd resolves the caller's executable and
  refuses any contribution claiming an organ identity that caller is not;
- canonical full-envelope hashing;
- stable D-Bus policy;
- message uniqueness;
- explicit capabilities;
- privacy inheritance;
- transactional migration;
- process isolation and explicit state ownership;
- typed action boundary;
- security and concurrency tests.

## Current limitations

- same-user D-Bus callers do not yet have capability tokens or method-level authorization. Event1
  now refuses organ impersonation, which closes forged provenance specifically; it does not
  constrain what a non-organ caller may contribute under a name of its own, and it is not a general
  authorization model;
- Journal hashing detects inconsistency but is not an external signature/trust anchor;
- retention and replica erasure are design targets, not implemented controls;
- lifecycle ownership, recovery, scheduling, and bounded transport are active enforcement paths;
  epistemic governance and the M9 authorized-action boundary remain proposed;
- user-service hardening does not yet define a least-privilege filesystem/network sandbox for
  every daemon;
- Mind binaries retain environment-triggered fault-injection hooks, which the reboot and
  split-commit gates set against the installed package. This is accepted rather than outstanding: a
  same-user process can already terminate any daemon with a signal, so the hooks grant no capability
  the same-user boundary does not already concede, and removing them would move the recovery
  evidence onto a binary that is not the shipped one.
