<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Failure Modes

| Failure | Required behavior |
|---|---|
| Plasma crashes | Mind remains alive; Presence reconnects |
| eventd unavailable | no durable writes; read-only degraded mode |
| Journal fails verification | stop writes and report integrity failure |
| identity state missing | do not overwrite silently |
| intentiond unavailable | commitment operations unavailable |
| workspaced unavailable | no attention projection; biography intact |
| disk full | reject append atomically |
| migration fails | rollback and preserve backup |
| protocol mismatch | reject and report unavailable capability |
| network partition | use only locally authorized behavior |

A component failure causes a specific capability deficit, not invented success.
