<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Testing Strategy

## Current suites

The Mind package runs eleven suites:

```text
protocol
journal
identity
intentions
predictor
selfmodel
workspace
presence
presence-extended
m1-runtime
eventd-integration
```

## M3 integration test

`eventd-integration` is executed through `dbus-run-session`, giving it a private user bus.

It proves:

```text
eventd registers Event1
Event1 reports Journal schema v2
Submit -> COMMIT -> Accepted
invalid Submit -> no Accepted
Recent / Contribution / Count / Verify round-trip over IPC
default Presence uses Event1
two default Presence surfaces share one session/runtime
second eventd cannot acquire the service name
eventd failure does not trigger a local SQLite fallback
```

The existing unit/domain tests continue using direct temporary Journals through the same
`EventStore` contract. That keeps domain tests fast while production topology is integration-tested
separately.

## Build

```bash
nix build .#packages.x86_64-linux.cybou-mind --print-build-logs
```

Expected suite count after M3:

```text
11/11
```

Full repository validation remains:

```bash
nix flake check --print-build-logs
```
