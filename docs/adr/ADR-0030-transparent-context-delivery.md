<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0030: Transparent Context Selection and Prompt Delivery

## Status

Accepted

Separate from [ADR-0029](ADR-0029-associative-context-projection.md) on purpose. That decision is
about what Mind associates; this one is about what leaves Mind. The second is a privacy boundary,
and a privacy boundary buried inside a retrieval ADR is a privacy boundary nobody reads.

Being Accepted, this outranks [Current State](../CURRENT_STATE.md): where an implementation and this
document disagree, the implementation is wrong.

## Context

Once Mind can assemble relevant context, something has to decide which part of it is sent somewhere
else — possibly to a remote model, over a network, irreversibly.

That decision cannot belong to whatever renders the interface, and it cannot belong to the model.
A view that assembles its own prompt is a second context owner with no rules; a model that retrieves
its own context is the memory architecture, whatever the documents say.

## Decision

### Four sets, and the person sees at least two of them

```
Activated Context     everything associative retrieval surfaced
        ↓
Available Context     what this request is permitted to consider
        ↓ policy
Selected Context      what the person and policy agreed on
        ↓
Delivered Context     what actually left the machine
```

**Available** and **Delivered** must be independently inspectable. Showing only what was retrieved
tells a person what Mind thought about; showing only what was sent tells them nothing about what was
withheld. The gap between the two is the interesting part, and it is exactly what is invisible in
every system that assembles prompts silently.

A worked example:

```
Prompt: "What should I drink this evening?"

Activated
  ✓ lemon            0.94  personal preference   evidence: …
  ✓ honey            0.87  via lemon → UsedWith
  ✓ ginger           0.81
  ○ previous episode        Private — held back
  ✕ medical context         blocked for remote model

Sent to Mistral: 7 items / 936 tokens
```

The person may `include`, `exclude`, `pin`, `expand why`, and `inspect evidence`. They may **not**
edit history through this surface: selection decides what is considered, never what happened.

### The view renders; it does not own

```
contextd → ContextBundle → native controller → local WebView
```

A WebView is used because a graph is genuinely easier to render there, and it is granted nothing
else:

```
network access   disabled
remote URLs      disabled
assets           local only
CSP              strict
Journal access   none
```

Its entire outward surface is four intents:

```
selectNode(id)   excludeNode(id)   expandNode(id)   requestEvidence(id)
```

Every one is a request to native code, which decides. Script inside the view can ask; it cannot act.
This is the same rule the substrate already applies to organs — *a proposal is not permission to
execute* — applied to a rendering surface, because a surface that could act would be an organ that
nobody audited.

### Policy removes items; it does not rewrite Mind

A remote-model policy may hold back private items. It does so by producing a **different delivered
set**, never by altering the local `ContextBundle`. Mind's own view of what is relevant does not
change because of where an answer is going, and a person inspecting the local bundle afterwards sees
what Mind actually considered rather than what was convenient to send.

### Delivery is recorded

What left the machine is a durable fact about the person's data, so it is a contribution: the
request id, the destination, the item ids and their evidence — never a second copy of the content.
A system that could not say what it had sent would be asking to be trusted about the one thing it
had made irreversible.

## Consequences

- The context inspector is useful with no language faculty present, because Activated and Available
  exist without one. That is a feature and a test: a surface that only works once a model is
  attached was really a prompt debugger.
- Policy becomes a visible, reviewable object rather than a filter buried in a request builder.
- Recording deliveries makes the Journal grow with use of remote models. Accepted: the alternative
  is a system whose most consequential action is the one it keeps no record of.

## Acceptance gates

| | Gate |
|---|---|
| **B1** | Available context and delivered context are independently inspectable |
| **B2** | A remote-model policy can remove private items without altering the local `ContextBundle` |
| **B3** | The view can request, and cannot act: no script path reaches the Journal or the network |
| **B4** | Every delivery is recorded with destination and item provenance, and no content copy |
| **B5** | The inspector works with no language faculty configured |
| **B6** | A held-back item is shown as held back, never silently omitted |

B6 is the one that decides whether this surface is honest. An item quietly dropped for policy
reasons and an item that was never relevant look identical unless the interface insists on the
difference — and the whole point of building this before a model exists is that afterwards, nobody
would notice which one they were looking at.

## Related documents

- [ADR-0018: Privacy Classification and Replication](ADR-0018-privacy-classification-and-replication.md)
- [ADR-0021: Language Models Are Optional Faculties](ADR-0021-language-models-are-optional-faculties.md)
- [ADR-0022: Authorized Action Boundary](ADR-0022-authorized-action-boundary.md)
- [ADR-0023: Mind Dock Discoverability and Access](ADR-0023-mind-dock-discoverability-and-access.md)
- [ADR-0029: Associative Context Projection and Semantic Activation](ADR-0029-associative-context-projection.md)
