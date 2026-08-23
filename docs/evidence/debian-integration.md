<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Debian integration

## The claim

The Mind owners start together on Debian 13, take their bus names, answer each other, and behave as
one system rather than as fourteen processes that happen to compile.

## Why it constrains today

Every organ is a separate process that fails separately, which is the property that makes a silent
organ a gap on a page rather than an outage. That property is only real if it has been observed: a
system whose organs have only ever been unit-tested has no evidence that they can be started at the
same time, let alone that they agree.

Debian 13 is the integration authority for this reason. The daemons need a session bus and systemd
user units, so a check that ran anywhere else would be checking something else.

## The evidence

```bash
bash scripts/test-multi-daemon-integration.sh
```

It starts every owner, exercises Event1, Identity1, Intention1, Presence1 and Meaning1 against each
other, raises an erasure epoch, and fails if any of them answers with a shape the contract does not
allow. It runs in CI on `ubuntu-latest` and on the deployment host.

The organ layering the architecture depends on is checked as a fact about the manifests rather than
as an assurance:

```bash
python3 scripts/validate-organ-layering.py
```

## What this does not prove

Behaviour under load, or across a machine reboot. It proves the system comes up and is coherent, not
that it stays that way under conditions nobody has applied.
