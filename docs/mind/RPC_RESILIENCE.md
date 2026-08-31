<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# RPC Resilience

## Scope

There is one bounded asynchronous D-Bus transport policy. It is the required path for shell
or owner operations that must not block their event loop. Both RpcClient and EventClient move
synchronous APIs onto explicitly timed asynchronous pending calls. Their public helpers retain a
five-second default and accept a shorter remaining budget from compound command coordinators.

## Operation semantics

Every resilient call declares one of:

```text
ReadOnly
IdempotentMutation
NonIdempotentMutation
```

Only read-only and explicitly idempotent operations may retry transport unavailability or timeout.
A non-idempotent mutation that times out is `UnknownOutcome`: it may have committed after the caller
lost the reply and must not be replayed automatically.

## Typed outcomes

```text
Succeeded
Unavailable
TimedOut
Rejected
UnknownOutcome
CircuitOpen
```

`Rejected` means a reply or D-Bus error explicitly declined the operation. It does not count as an
infrastructure failure and does not open the circuit. `Unavailable`, `TimedOut`, and
`UnknownOutcome` contribute to circuit state. A boolean-acceptance reply can be required by the
call contract; empty or false replies then classify as `Rejected`.

## Retry and backoff

The default policy is bounded to three attempts with exponential delay, a maximum delay, and
deterministic bounded jitter. Callers can provide another validated policy, but cannot make
non-idempotent mutations retry-safe by increasing the attempt count.

Deterministic jitter makes tests reproducible while preventing equal methods from sharing an
unbounded immediate retry loop. Retry state lives in the client operation, not in cognitive owner
storage.

## Circuit breaker

The circuit begins `Closed`, opens after a bounded number of infrastructure failures, and permits
one `HalfOpen` probe after its open interval. Success or an explicit rejection closes the circuit;
another infrastructure failure reopens it. An open circuit returns `CircuitOpen` without sending a
D-Bus message.

The current breaker is process-local transport protection. It is not durable cognitive state and
does not replace Health1 capability projection.
Fault tests may crash a selected method at `after-retryable-failure` or `after-circuit-open` via
`CYBOU_RPC_FAILPOINT` plus `CYBOU_RPC_FAILPOINT_METHOD`; these controls are inert unless explicitly
set and do not alter retry classification.

## First production consumer

The Living Canvas lifecycle interruption command uses `AsyncRpcClient` with
`NonIdempotentMutation`, a five-second deadline, and boolean acceptance. Timeout therefore:

- does not block the client event loop;
- reports `unknown-outcome` rather than invented failure;
- does not retry the mutation;
- leaves terminal lifecycle ownership in lifecycled;
- requires later state refresh/reconciliation to learn the actual outcome.

Automatic lifecycle owner dispatch uses `IdempotentMutation` because every operation carries a
durable deterministic `runId:capability:highWaterMark` key. Its production deadline is five
seconds and may be reduced in fault tests with `CYBOU_LIFECYCLE_OWNER_TIMEOUT_MS` (bounded to
50–60000 ms). Exhausted retries fail required work closed without advancing Event1 consumer
progress; callbacks remain fenced by active run identity, including delayed replies.

## Automated evidence

`rpc-resilience` covers retry eligibility, deterministic bounded backoff, D-Bus outcome
classification, unknown mutation outcome, and circuit transitions. The process-level lifecycle
timeout test proves the migrated shell call remains non-blocking and does not mutate or duplicate
the active run when the server exceeds its deadline. Scheduled-owner process coverage additionally
proves bounded idempotent timeout, late-reply convergence, preserved backlog, and recovery by a new
evidence-bound run. The focused KVM gate suspends Event1 and proves Promise rejects within an
explicit one-second server budget and three-second client deadline without Journal growth.
Required-owner preflight and one monotonic command deadline prevent later compound steps from
accumulating independent RPC budgets. Promise, Reflect, Observe, Predict, Fulfill, and Abandon now
share this model: each reads one Health1 snapshot and passes only the remaining budget to subsequent
owners. Event-producing commands preflight Event1; read-only Predict does not acquire an artificial
Journal dependency. `InterruptLifecycle` also shares one server-side budget across Lifecycle1 state
validation and terminal mutation; exhaustion before `FinishRun` prevents that mutation from being
sent. Its shell transport remains asynchronous and non-idempotent, so a lost reply is still
`UnknownOutcome`. Read-only `Snapshot`, `Activity`, and `DetailedObligations` also own one monotonic
budget. Snapshot preserves its stable projection schema with typed empty/default values when the
budget expires and never starts another owner call afterward. This completes the compound
Presence rollout.
