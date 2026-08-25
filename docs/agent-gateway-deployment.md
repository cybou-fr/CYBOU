# Per-capsule model gateway deployment

`cybou-agent-gateway@.service` is one host process for one capsule lease. It registers the configured
LiteLLM worker, mints one gateway bearer bound to that lease and task, writes it mode `0600`, and
serves the OpenAI-compatible router on a mode `0600` Unix socket. It opens no host TCP listener.

There are four different inputs and they deliberately have different owners:

- `/etc/cybou/provider.env` is root-owned, non-secret operator routing policy. Deployment creates a
  blank, fail-closed file once and never overwrites it.
- `/etc/cybou/litellm-master-key` is the root-only proxy credential. systemd presents it to the
  unprivileged service through `LoadCredential`; it is not placed in a process environment or an
  argument.
- `/run/cybou-agent-leases/<instance>.lease` is the standing lease itself, CBOR-encoded, minted once
  by the session owner from the profile a person selected. It carries the capsule UUID, agent,
  workspace, network grant, resource budget, lifetime, model class and spending ceiling. The gateway
  reads it; it never rebuilds one.
- `/run/cybou-agent-leases/<instance>.env` carries what is *not* authority: the task UUID, the
  per-token ceilings and the sensitivity class. It contains no provider secret.

The provider file must explicitly define `CYBOU_LITELLM_BASE_URL`, `CYBOU_LITELLM_PROVIDER`,
`CYBOU_LITELLM_MODEL_GROUP`, `CYBOU_LITELLM_DEPLOYMENT_SHA256`, `CYBOU_LITELLM_ZERO_COST`,
`CYBOU_LITELLM_TIMEOUT_MS`, and `CYBOU_MODEL_MICROUSD_PER_UNIT`. The digest identifies the deployed
proxy build used for attribution; it must not be guessed from a URL.

`CYBOU_LITELLM_ZERO_COST` is exactly `yes` or `no`, and there is no default. Only an operator knows
what their deployment charges — Cybou cannot see a price list — and a default either way would be
Cybou deciding on their behalf: one direction silently forbids the free models a person selected, the
other silently spends their money. A lease whose policy is `ZeroCostOnly` may only be served by a
route declared `yes`, and a completion on such a route that bills anything is refused rather than
returned, because handing back an answer somebody has now been charged for — having asked for none —
would make the refusal cosmetic.

The launch file must explicitly define `CYBOU_AGENT_TASK_ID`, `CYBOU_MODEL_TOKEN_LIMIT`,
`CYBOU_MODEL_MAX_OUTPUT_TOKENS` and `CYBOU_MODEL_SENSITIVITY`. It defines no capsule id, workspace,
lifetime, model class or spending ceiling, because those are the lease and the lease already says
them.

That split is the point. When the gateway rebuilt its own lease from environment values, a launch
file and a running capsule could each be internally valid and still describe different permissions —
four hours here, one hour there; `Strong` here, `Fast` there — and nothing downstream could say which
one a person had approved. One human selection produces one lease, and every component derives from
that same object: the capsule spec is compiled from it, the model token is issued against it, and its
clock is the one that ends both.

The lifecycle owner writes both files, then starts `cybou-agent-gateway@<instance>.service` and gives
the capsule backend only:

```text
CYBOU_MODEL_SOCKET=/run/cybou-agent-<instance>/model.sock
CYBOU_MODEL_TOKEN_FILE=/run/cybou-agent-<instance>/model-token
```

The unit has no `[Install]` section and deployment never starts it. A boot target cannot manufacture
a capsule grant. An empty provider file, empty credential, missing or unreadable lease, a lease that
grants no model, an expired or withdrawn lease, a reused runtime path, or a malformed ceiling fails
before the Unix listener exists.

Until the session owner exists, `cargo run -p cybou-capsule --example issue-lease -- <path>` mints a
lease through the same public mint the owner will call. It is a bring-up and gate tool, not a launch
surface: it holds no lifecycle and cannot stop what it starts.

`scripts/test-agent-gateway-gate.sh` proves this lifecycle against a fake LiteLLM peer on Debian.
`scripts/test-opencode-pack-live.sh` remains the completion gate against an operator-selected real
provider. It is `NOT RUN`, not passed, until that external deployment and credential exist.
