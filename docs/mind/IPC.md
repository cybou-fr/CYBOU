<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Local Cognitive IPC

## Transport

Use versioned Qt D-Bus interfaces locally and keep service logic behind a transport abstraction.

## Proposed names

```text
org.cybou.Mind.Event1
org.cybou.Mind.Identity1
org.cybou.Mind.Intention1
org.cybou.Mind.Presence1
```

## Error names

```text
org.cybou.Error.InvalidEnvelope
org.cybou.Error.MissingCause
org.cybou.Error.MissingEvidence
org.cybou.Error.DuplicateOutcome
org.cybou.Error.UnsupportedVersion
org.cybou.Error.UnavailableCapability
```

Use typed arguments or versioned CBOR. Free-form organ-to-organ agent chat is prohibited.
