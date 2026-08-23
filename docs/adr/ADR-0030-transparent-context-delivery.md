<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0030: Transparent Context Selection and Delivery

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

> **The subsections below are the original decision, and parts of them are superseded.** The
> amendments at the end of this document are the current rule: `Delivered` means supplied to a
> named consumer rather than sent off the machine, every destination is filtered by its own trust
> rather than by locality, and durable recording follows whether a consumer retains or adapts.
>
> The original text is kept rather than rewritten. What was decided, and why it changed, is itself
> part of the record â the same reason an erasure record is permanent.

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

**Available** and **Delivered** must be independently inspectable, and â by the amendment below â
to the *person*, through an inspector. A consumer is not shown what was withheld from it: that a
concept exists is frequently the sensitive part of it.

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

### Amendment: a consumer is not trusted for being local

The decision above was written around one question — *what left the machine?* — and answered it with
one bit, `remote`. That was too weak, and the weakness became visible as soon as
[ADR-0021](ADR-0021-language-models-are-optional-faculties.md) moved cognition off remote models
entirely: once every consequential consumer is local, a policy that only filters remote ones filters
nothing that matters.

The question becomes:

```text
what did Mind supply to this consumer, and under what policy?
```

The four sets are unchanged. `Delivered` now means supplied to a named consumer, which does not
necessarily mean network egress.

**Locality does not imply unrestricted cognitive access.** A parser, a local model, a planner, an
inspector and a future plugin have genuinely different trust, and one boolean cannot express that. A
destination is described by what it is permitted to consume:

```text
Destination {
    id,
    trust,              // how much of the person's context this consumer may see
    retains,            // whether what it receives outlives the request
    externalBoundary    // whether delivery crosses a network or trust boundary
}
```

The rule that local destinations are unfiltered is superseded. Every destination is filtered by its
own trust, and a consumer gains context by being permitted, never by being nearby.

### Amendment: recording follows retention, not distance

The original B4 recorded every delivery. The tempting correction is to record only what crosses a
network boundary, on the grounds that local use is cheap.

That is the wrong axis, and this package is what shows it.
[ADR-0032](ADR-0032-layered-lifelong-learning.md) and
[ADR-0033](ADR-0033-learned-artifact-governance.md) make local consumption durable: a local model
that adapts on delivered context has written it into parameters that ADR-0033 itself admits cannot
be surgically unlearned. Under erasure, ADR-0033's A6 has to find every artifact a payload
influenced — and the delivery record is the only evidence of how the contamination travelled.

So the durable record follows **whether a consumer retains or adapts on what it receives**, not
whether it is far away. An inspector that renders and forgets needs no contribution. A learning
consumer needs one wherever it runs. A delivery crossing an external boundary is durably recorded
regardless, because irreversibility is its own reason.

```text
DeliveryPlan            inspectable for every destination, always
delivery contribution   when the consumer retains, adapts, or crosses an external boundary
```

The cost argument was real but it was an argument about which consumers are consequential, and
distance is not what makes them so.

### Amended acceptance gates

| | Gate |
|---|---|
| **B1** | Available and delivered context are independently inspectable for a named consumer |
| **B2** | Different destination policies narrow delivery without mutating the local `ContextBundle` |
| **B3** | The view can request, and cannot act: no script path reaches the Journal or the network |
| **B4** | A retaining, adapting or externally-bound delivery is durably recorded with destination and provenance, and no content copy |
| **B5** | The inspector works with no language or generative model configured |
| **B6** | A held-back item is shown as held back, never silently omitted |
| **B7** | A local consumer does not gain unrestricted context solely because it is local |

B6 still decides whether the surface is honest. B7 decides whether the policy is: a boundary that
every consumer on the machine walks straight through is a boundary in name only.

## Related documents

- [ADR-0018: Privacy Classification and Replication](ADR-0018-privacy-classification-and-replication.md)
- [ADR-0021: Language Models Are Optional Faculties](ADR-0021-language-models-are-optional-faculties.md)
- [ADR-0022: Authorized Action Boundary](ADR-0022-authorized-action-boundary.md)
- [ADR-0029: Associative Context Projection and Semantic Activation](ADR-0029-associative-context-projection.md)
