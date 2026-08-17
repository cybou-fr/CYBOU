<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0018: Privacy Classification and Replication

## Status

Proposed

## Context

Distributed operation requires formal rules for what may leave a device.

## Decision

Use Local, Node, Household, and Public. Derived data inherits the most restrictive source.
Replication requires explicit trust and compatible policy.

### Amendment: scope, sensitivity, and retention are three axes

`PrivacyClass` has been asked to answer two different questions, and it can only answer one.

Its ordering is a **replication scope**: where a contribution may exist. `Local` is the default,
which is exactly why it cannot double as a danger marker â the overwhelming majority of ordinary
contributions are Local, so "restricted scope" carries no information about whether the content is
sensitive. [ADR-0030](ADR-0030-transparent-context-delivery.md)'s delivery policy nonetheless reads
it as a disclosure clearance, mapping consumer trust onto privacy floors. That works today only
because nothing yet holds a credential.

It breaks on [ADR-0033](ADR-0033-learned-artifact-governance.md)'s **A9**, which forbids secrets,
keys, tokens and passwords from entering an opaque training path. Nothing in the protocol can say
that a payload *is* one. As written, A9 is satisfiable by a refusal for any reason at all â the
shape of an acceptance test that proves nothing.

So three independent axes:

```text
Where may it exist?     PrivacyScope       Local | Node | Household | Public
Who may be shown it?    SensitivityClass   Ordinary | Personal | Sensitive | Secret | Credential
How long may it exist?  RetentionClass     Ephemeral | Short | Standard | Long | Permanent
```

They are independent because their answers do not correlate. An identity fact may be widely
replicable within a household, deeply personal, and needed for years. Public telemetry may be
worthless after ten minutes. A credential is not made safe by being Local, and a Local contribution
is not a credential.

The rules:

- **Sensitivity propagates like privacy**: derived data takes the most sensitive of its sources,
  and a contribution declaring less than its evidence is refused rather than silently corrected.
  This is the discipline retention already follows, and it exists for the same reason â a
  conclusion that restated its evidence at a weaker classification would launder it.
- **Delivery policy reads sensitivity, not scope.** Consumer trust maps onto a sensitivity ceiling.
  Scope keeps its own job: what may cross a device boundary.
- **`Credential` and `Secret` may never be a deliberate opaque-training target**, whatever their
  scope or retention. That is A9, and it becomes checkable rather than rhetorical: a test can
  construct a `Credential` contribution and require the training-input path to refuse it *for that
  reason*.
- **Absent means unclassified, not safe.** A contribution with no sensitivity recorded is treated
  as `Personal`, not `Ordinary`. The alternative makes every unmigrated row look harmless, which is
  the failure mode that matters when the whole point is to notice the dangerous ones.

### Migration

Sensitivity is a new envelope field, so it needs a schema version and a canonical-form extension,
and existing rows must keep the hashes they were written with. The established shape applies:
extend the canonical form under the new schema version only, add the column with a default in the
migration path, and let the v1 migration test prove no history was rehashed.

That test has caught this class of mistake three separate times while retention was being added.
It is the acceptance gate for this amendment, not an afterthought to it.

## Consequences

Privacy becomes enforceable at protocol and transport boundaries.

Classification arguments stop being fought on the wrong axis. "This is sensitive" and "this must not
leave the house" become separately answerable, and a policy can say which one it means.

A9 becomes a gate rather than a wish. Until this lands, ADR-0033 cannot honestly claim it.

Three axes are more to carry than one. Accepted: the alternative is one axis quietly meaning
different things in different places, which is how the delivery policy came to read a replication
scope as a clearance without anyone deciding that it should.

## Alternatives Considered

Treating privacy as display metadata only was rejected.

### Keep one axis and add values to it

Rejected. Ordering `Local < Node < Household < Public` alongside `Credential` requires deciding
whether a credential is more or less restricted than a household fact, and the question has no
answer because the two are not on the same scale.

### Infer sensitivity from content

Rejected as the contract. A classifier may propose, but a payload that is sensitive only when
something recognises it as such is not classified â and the moment it fails, the failure is silent
and permanent.

## Related documents

- [ADR-0028: Retention and Erasure](ADR-0028-retention-and-erasure.md)
- [ADR-0030: Transparent Context Selection and Delivery](ADR-0030-transparent-context-delivery.md)
- [ADR-0033: Learned Artifact Governance](ADR-0033-learned-artifact-governance.md)
