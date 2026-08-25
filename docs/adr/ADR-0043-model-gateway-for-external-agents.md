<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0043: A Model Gateway for External Agents

## Status

Proposed

## Context

[ADR-0035](ADR-0035-governed-model-brokerage.md) decides how *Mind* reaches a model, and its 2026-08-23
amendment made the request vocabulary typed: `InterpretAct`, `RealizeResponsePlan`, `EmbedText`,
`Rerank`, `SummarizeEvidence`, `ProposeDesktopPlan`, `DiagnoseSystem`. That is right for Mind. A
typed task is what lets `NoModel` be answered per task, lets a route be chosen deterministically, and
lets an answer be attributed to the model that gave it.

An external agent under [ADR-0042](ADR-0042-agent-capsule-platform.md) does not speak that
vocabulary and never will. It expects a chat-completions API, because every agent worth hosting was
built against one.

There are two wrong ways to resolve this and they fail differently.

**Widening `ModelBroker1` to carry arbitrary prompts** destroys the property that makes it worth
having. A broker whose request type is *a string* cannot match a task to a route, cannot refuse one
capability while permitting another, and cannot say which of its answers was degraded. The typed
vocabulary is not decoration; it is the whole mechanism.

**Giving the agent a provider key** puts a durable credential inside the least trusted process on
the machine, which is the thing capsules exist to prevent.

## Decision

### A second surface, not a wider one

```text
Mind                                 external agent
  │ typed cognitive task               │ chat completions
  ▼                                    ▼
ModelBroker1                      Model Gateway
  │                                    │
  └──────────► provider workers ◄──────┘
```

Two surfaces, one set of provider workers, one policy, one cost ledger. `ModelBroker1` keeps its
typed vocabulary unchanged. The Model Gateway speaks the shape agents already speak.

They are not layered one on the other. Making the gateway a client of `ModelBroker1` would require
inventing a typed task meaning *whatever this agent asked for*, which is the widening this decision
refuses, wearing a different name.

### What an agent receives is a lease, never a key

```text
CYBOU_EPHEMERAL_MODEL_TOKEN
  capsule      agent-8472
  model class  Strong
  expires      24 hours
  ceiling      5M tokens · a fixed spend
```

The provider secret stays outside every capsule. An agent that is compromised, confused, or simply
badly written leaks something that expires and is bounded, rather than something that is neither.

The ceiling is enforced at the gateway, where the accounting is, and not asked of the agent.

### A model class, not a model name

An agent asks for a capability; the gateway decides what serves it.

```text
Fast · Strong · Free · Local
```

An agent naming a specific model would pin a capsule to a provider's naming, break when that name is
retired, and route around whatever policy the class encodes. A person may still pin one deliberately
— that is a decision they are making, and it is recorded as one.

### Provider breadth is a worker, not an architecture

The first implementation is one worker in front of a multi-provider proxy, because writing and
maintaining a hundred provider adapters is not this project's contribution. As researched on
2026-08-24, LiteLLM offers one OpenAI-compatible interface to more than a hundred providers with
virtual keys, budgets, rate limits and cost tracking already in it.

```text
Model Gateway
  └── cybou-provider-litellm  ──► 100+ providers
```

This is a worker behind an interface Cybou owns. Native workers may replace it per provider later —
to drop a runtime dependency, or because one provider's behaviour deserves handling this project
controls. Nothing above the worker changes when that happens, and if the research above turns out to
be wrong, what is wrong is one worker.

### Volatile facts are catalogue entries, never code

Free tiers exist across several providers and their limits change without notice. Cybou must not
contain the sentence *this model is free*.

```text
Mistral       free tier available          checked 3h ago
Gemini        free tier available          checked 3h ago   ⚠ free-tier data may train the provider
Groq          free quota available         checked 3h ago
OpenRouter    free models available        checked 3h ago
```

A catalogue with a timestamp, and a warning where a provider's free tier carries a condition a person
would want to know about before sending their code through it. A hardcoded free-tier claim becomes a
lie on the provider's schedule rather than on ours.

### Cost and attribution are the gateway's, not the agent's

Every completion is attributed to a capsule, an agent, a task, a model and a provider, and spending
accrues against the capsule's lease. An agent reporting its own usage is the executor grading its own
homework, which [ADR-0022](ADR-0022-authorized-action-boundary.md) already refuses in the case where
it matters most.

### Egress remains a capsule question

A capsule's network grant governs what it may reach. The gateway is one of those destinations, and
the fact that a request carries a model prompt does not exempt it: what an agent may send outward is
[ADR-0030](ADR-0030-transparent-context-delivery.md)'s question, and routing it through
Cybou is what makes it answerable at all.

## Consequences

An agent is configured with one endpoint and one token and works with every provider Cybou can
reach. A person changes provider without reconfiguring their agents. A capsule cannot outspend its
lease, cannot exfiltrate a key it never held, and cannot silently switch to a provider policy forbids.

Cybou takes on a request path that must stay available, since an agent whose gateway is down is an
agent that cannot work, and a compatibility surface defined by other people's APIs.

## Acceptance gates

| | Gate |
|---|---|
| **N1** | `ModelBroker1`'s typed vocabulary is unchanged by the existence of this gateway |
| **N2** | No provider credential is readable from inside any capsule |
| **N3** | A token is scoped to one capsule and stops working when that capsule ends |
| **N4** | A spending or token ceiling is enforced at the gateway and cannot be exceeded by the agent |
| **N5** | Every completion is attributable to capsule, agent, task, model and provider |
| **N6** | Provider availability and free-tier status are catalogue data with an observation time, never compiled in |
| **N7** | A provider being unreachable degrades a class to a named alternative or reports absence; it never silently substitutes |
| **N8** | Removing the multi-provider worker changes no interface above it |

## Alternatives Considered

### Widen `ModelBroker1` to accept arbitrary prompts

Rejected. It removes task matching, per-task `NoModel`, deterministic routing and honest attribution
— every property the typed vocabulary was introduced to gain.

### Give agents provider keys directly

Rejected. A durable credential inside the least trusted process on the machine.

### Require Ollama as the model layer

Rejected as a requirement, welcome as a worker. Cybou already has a brokerage design with routing,
sensitivity ceilings, attribution and a real `NoModel` answer; replacing that with a runtime that
does not model those concerns would be trading an architecture for an installer. Ollama sits beside
llama.cpp, a hosted provider and the rest, as one worker among several.

### Let each agent keep its own provider configuration

Rejected. It is the status quo, and it means the machine's owner cannot answer what was sent where,
at what cost, under whose key.

## Related documents

- [ADR-0035](ADR-0035-governed-model-brokerage.md) — Mind's own typed model access
- [ADR-0042](ADR-0042-agent-capsule-platform.md) — the capsule a lease is scoped to
- [ADR-0030](ADR-0030-transparent-context-delivery.md) — what may cross a boundary
- [ADR-0021](ADR-0021-language-models-are-optional-faculties.md) — why no model owns anything, on either surface

## Implementation status

B4 implements the compatibility router, ephemeral lease token, shared worker registration, route
policy, accounting and attribution ledger. B5 adds the replaceable `cybou-provider-litellm` worker
without adding it as a gateway dependency. It maps classes to operator-owned proxy model groups and
mints one short-lived, model/budget/concurrency-scoped virtual key per completion, keeping the proxy
master key out of capsules. Proxy-observed cost is rounded upward into the integer lease currency;
model group, deployment id, response model and call id join each answer to proxy spend evidence.
The deployment contract requires database-backed budget reservation and known token pricing for
every mapped route: only then can the forwarded `max_tokens` become a maximum-cost reservation
before the provider sees the request. A proxy that cannot price the route is not admissible.

The B5 gate uses a fake HTTP proxy and no provider credential. No listener, LiteLLM service or real
provider is deployed implicitly; endpoint binding, token injection and the first live provider call
belong to the first agent pack.
