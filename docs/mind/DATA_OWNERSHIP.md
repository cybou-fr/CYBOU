<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Data Ownership

## Current M1–M6 owners

| Resource | Owner |
|---|---|
| canonical `journal.db` | `cybou-eventd` |
| capability dependency graph and current health snapshot | `cybou-healthd` |
| lifecycle mode and consolidation run state | `cybou-lifecycled` |
| `identity.json` | `cybou-identityd` |
| volatile identity login marker | `cybou-identityd` |
| bounded Workspace | `cybou-workspaced` |
| presentation aggregation | `cybou-presenced` |
| visual cache | QML Presence proxy |

Intentions, Predictor, and Self derive their state from Event1 plus their narrow operation logic.

## Locations

Persistent:

```text
$XDG_STATE_HOME/cybou
```

Runtime:

```text
$XDG_RUNTIME_DIR/cybou
```

The runtime identity marker prevents a daemon restart from being confused with a new logical login.

## Invariants

- Plasma does not own cognitive persistence.
- presenced does not open `journal.db`.
- only identityd writes `identity.json`;
- only workspaced owns live bounded attention;
- opening another UI surface does not create another Mind;
- process isolation does not introduce duplicate authoritative copies.

## Current M6 extension and proposed M7–M9 ownership

| Resource | Target owner/boundary |
|---|---|
| lifecycle mode and consolidation run state | current `cybou-lifecycled`; no organ state ownership |
| capability graph and health policy | current `cybou-healthd`; Presence remains read-only |
| perception acquisition state | replaceable adapter/faculty |
| provenance-bearing accepted observation | `cybou-eventd` Journal history |
| current epistemic claim projection | dedicated owner to be selected by an implementation ADR |
| retention policy and outstanding erasure obligations | dedicated policy owner to be selected before M7 |
| homeostatic pressure projection | typed aggregation over owner metrics, not direct owner mutation |
| executive focus/deferral state | evolution of `cybou-workspaced` unless a later ADR separates it |
| authorization decision | future M9 policy boundary, never the value or language faculty |

ADR-0024 and ADR-0025 deliberately do not assign new daemon names before persistence, failure, and
privacy contracts justify process boundaries.
