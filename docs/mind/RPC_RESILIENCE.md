<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# RPC Resilience

## Scope

P6.3 introduces one bounded asynchronous D-Bus transport policy. It is the required path for shell
or owner operations that must not block their event loop. Existing synchronous read paths are not
implicitly converted and remain a documented limitation until migrated deliberately.

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

## First production consumer

The Plasma Presence lifecycle interruption command uses `AsyncRpcClient` with
`NonIdempotentMutation`, a five-second deadline, and boolean acceptance. Timeout therefore:

- does not block the Plasma event loop;
- reports `unknown-outcome` rather than invented failure;
- does not retry the mutation;
- leaves terminal lifecycle ownership in lifecycled;
- requires later state refresh/reconciliation to learn the actual outcome.

## Automated evidence

`rpc-resilience` covers retry eligibility, deterministic bounded backoff, D-Bus outcome
classification, unknown mutation outcome, and circuit transitions. The process-level lifecycle
timeout test proves the migrated shell call remains non-blocking and does not mutate or duplicate
the active run when the server exceeds its deadline.
