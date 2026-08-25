# Provider catalogue

The provider catalogue is observation data, not a provider list compiled into Cybou. A fresh
installation starts with an empty catalogue and therefore claims that no provider is eligible.

`cybou-provider-catalogue` accepts schema v1 JSON. Each provider carries two independent claims:

- `availability`: whether a bounded probe through the configured provider path succeeded;
- `zeroCostAccess`: whether the cited terms and pricing exposed a route whose usage accrued no
  charge when checked.

Both claims have `observedAt`, `validUntil` and at least one credential-free HTTPS evidence URL.
Observation timestamps must be UTC and cannot be in the future when loaded. Conditions such as data
use, payment-method, regional or quota restrictions carry the same time and evidence fields. A
condition is a warning a person sees, not a permission silently inferred by the catalogue.

After `validUntil`, a claim remains present and displayable as stale evidence but cannot make a
provider eligible. `unknown`, `unavailable` and `stale` remain different answers: retry observation,
choose another provider, and refresh expired evidence are different remedies.

Route priority is not catalogue data. The caller supplies one preferred provider and an ordered list
of alternatives from operator policy. Resolution returns `Preferred`, `NamedAlternative` or
`Absent`; the alternative variant includes the rejected provider and its eligibility reason. An
unlisted provider is never discovered or substituted implicitly.

The checked-in [schema example](../fixtures/provider-catalogue-v1.example.json) uses reserved
`.invalid` domains and asserts no real provider fact. A deployment-specific observer and the first
live provider configuration belong to B7; provider facts must be refreshed outside the binary.
