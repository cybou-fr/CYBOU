# Per-capsule model gateway deployment

`cybou-agent-gateway@.service` is one host process for one capsule lease. It registers the configured
LiteLLM worker, mints one gateway bearer bound to that lease and task, writes it mode `0600`, and
serves the OpenAI-compatible router on a mode `0600` Unix socket. It opens no host TCP listener.

There are three different inputs and they deliberately have different owners:

- `/etc/cybou/provider.env` is root-owned, non-secret operator routing policy. Deployment creates a
  blank, fail-closed file once and never overwrites it.
- `/etc/cybou/litellm-master-key` is the root-only proxy credential. systemd presents it to the
  unprivileged service through `LoadCredential`; it is not placed in a process environment or an
  argument.
- `/run/cybou-agent-leases/<instance>.env` is a short-lived launch decision written by the capsule
  coordinator. It binds an instance to the capsule UUID, task UUID, workspace, lifetime, model
  class, sensitivity, token/output/spend ceilings. It contains no provider secret.

The provider file must explicitly define `CYBOU_LITELLM_BASE_URL`, `CYBOU_LITELLM_PROVIDER`,
`CYBOU_LITELLM_MODEL_GROUP`, `CYBOU_LITELLM_DEPLOYMENT_SHA256`,
`CYBOU_LITELLM_TIMEOUT_MS`, and `CYBOU_MODEL_MICROUSD_PER_UNIT`. The digest identifies the deployed
proxy build used for attribution; it must not be guessed from a URL.

A lease file must explicitly define `CYBOU_CAPSULE_ID`, `CYBOU_AGENT_TASK_ID`,
`CYBOU_AGENT_WORKSPACE`, `CYBOU_AGENT_LEASE_SECONDS`, `CYBOU_MODEL_CLASS`,
`CYBOU_MODEL_SPEND_LIMIT`, `CYBOU_MODEL_TOKEN_LIMIT`, `CYBOU_MODEL_MAX_OUTPUT_TOKENS`, and
`CYBOU_MODEL_SENSITIVITY`. The lifecycle owner then starts `cybou-agent-gateway@<instance>.service`
and gives the capsule backend only:

```text
CYBOU_MODEL_SOCKET=/run/cybou-agent-<instance>/model.sock
CYBOU_MODEL_TOKEN_FILE=/run/cybou-agent-<instance>/model-token
```

The unit has no `[Install]` section and deployment never starts it. A boot target cannot manufacture
a capsule grant. An empty provider file, empty credential, missing lease file, expired lease, reused
runtime path, or malformed ceiling fails before the Unix listener exists.

`scripts/test-agent-gateway-gate.sh` proves this lifecycle against a fake LiteLLM peer on Debian.
`scripts/test-opencode-pack-live.sh` remains the completion gate against an operator-selected real
provider. It is `NOT RUN`, not passed, until that external deployment and credential exist.
