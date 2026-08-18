// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Retry, deadline, and circuit semantics shared by bounded D-Bus clients.

/// Whether repeating an RPC could duplicate an externally meaningful effect.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationSemantics {
    /// A repeat only reads the same owner contract.
    ReadOnly,
    /// A mutation carries an owner-enforced idempotency key.
    IdempotentMutation,
    /// A repeat could apply the operation twice.
    NonIdempotentMutation,
}

/// Typed terminal or retryable result of one RPC attempt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RpcOutcome {
    /// The owner returned a valid accepted reply.
    Succeeded,
    /// Transport or owner activation was unavailable.
    Unavailable,
    /// No reply arrived inside the attempt's remaining outer deadline.
    TimedOut,
    /// The owner explicitly refused the request.
    Rejected,
    /// A non-idempotent mutation timed out after dispatch.
    UnknownOutcome,
    /// No dispatch occurred because the circuit is open.
    CircuitOpen,
}

/// Circuit-breaker state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CircuitState {
    /// Calls flow normally.
    Closed,
    /// Calls fail without dispatch.
    Open,
    /// Exactly one recovery probe may be dispatched.
    HalfOpen,
}

/// Frozen predecessor-compatible retry policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RetryPolicy {
    /// Total attempts including the initial dispatch.
    pub maximum_attempts: u32,
    /// Initial retry delay.
    pub base_delay_ms: u32,
    /// Exponential-backoff ceiling.
    pub maximum_delay_ms: u32,
    /// Symmetric deterministic jitter percentage.
    pub jitter_percent: u32,
    /// Infrastructure failures required to open the circuit.
    pub circuit_failure_threshold: u32,
    /// Time before one half-open probe is permitted.
    pub circuit_open_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            maximum_attempts: 3,
            base_delay_ms: 100,
            maximum_delay_ms: 2_000,
            jitter_percent: 20,
            circuit_failure_threshold: 3,
            circuit_open_ms: 5_000,
        }
    }
}

impl RetryPolicy {
    /// Whether all bounds can be applied without ambiguity.
    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.maximum_attempts > 0
            && self.maximum_delay_ms >= self.base_delay_ms
            && self.jitter_percent <= 100
            && self.circuit_failure_threshold > 0
    }
}

/// One monotonic outer deadline shared by every attempt and retry delay.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Deadline {
    expires_at_ms: u64,
}

impl Deadline {
    /// Create a deadline from a monotonic start and total budget.
    #[must_use]
    pub const fn new(started_at_ms: u64, budget_ms: u64) -> Self {
        Self {
            expires_at_ms: started_at_ms.saturating_add(budget_ms),
        }
    }

    /// Remaining budget, or zero after expiry.
    #[must_use]
    pub const fn remaining_ms(self, now_ms: u64) -> u64 {
        self.expires_at_ms.saturating_sub(now_ms)
    }
}

/// Whether another attempt is legal under the frozen predecessor policy.
#[must_use]
pub const fn should_retry(
    outcome: RpcOutcome,
    semantics: OperationSemantics,
    completed_attempts: u32,
    policy: RetryPolicy,
) -> bool {
    policy.is_valid()
        && !matches!(semantics, OperationSemantics::NonIdempotentMutation)
        && completed_attempts < policy.maximum_attempts
        && matches!(outcome, RpcOutcome::Unavailable | RpcOutcome::TimedOut)
}

/// Deterministic predecessor-compatible exponential backoff with bounded jitter.
#[must_use]
pub fn retry_delay_ms(completed_attempts: u32, seed: u32, policy: RetryPolicy) -> u32 {
    if !policy.is_valid() || completed_attempts == 0 {
        return 0;
    }
    let exponent = completed_attempts.saturating_sub(1).min(31);
    let delay = policy
        .base_delay_ms
        .saturating_mul(1_u32 << exponent)
        .min(policy.maximum_delay_ms);
    let spread = u64::from(delay) * u64::from(policy.jitter_percent) / 100;
    if spread == 0 {
        return delay;
    }
    let mixed = seed ^ 0x9e37_79b9_u32.wrapping_mul(completed_attempts);
    let width = spread * 2 + 1;
    let offset = i64::from(mixed) % i64::try_from(width).unwrap_or(i64::MAX)
        - i64::try_from(spread).unwrap_or(i64::MAX);
    u32::try_from((i64::from(delay) + offset).clamp(0, i64::from(policy.maximum_delay_ms)))
        .unwrap_or(policy.maximum_delay_ms)
}

/// Pure monotonic circuit breaker; callers supply the clock.
#[derive(Clone, Debug)]
pub struct CircuitBreaker {
    policy: RetryPolicy,
    failures: u32,
    opened_at_ms: Option<u64>,
    half_open_probe_used: bool,
}

impl CircuitBreaker {
    /// Create a closed breaker for a valid policy.
    #[must_use]
    pub const fn new(policy: RetryPolicy) -> Self {
        Self {
            policy,
            failures: 0,
            opened_at_ms: None,
            half_open_probe_used: false,
        }
    }

    /// Current state at the supplied monotonic instant.
    #[must_use]
    pub const fn state(&self, now_ms: u64) -> CircuitState {
        match self.opened_at_ms {
            None => CircuitState::Closed,
            Some(opened) if now_ms.saturating_sub(opened) >= self.policy.circuit_open_ms => {
                CircuitState::HalfOpen
            }
            Some(_) => CircuitState::Open,
        }
    }

    /// Permit a normal call or claim the single half-open probe.
    pub const fn allow(&mut self, now_ms: u64) -> bool {
        match self.state(now_ms) {
            CircuitState::Closed => true,
            CircuitState::HalfOpen if !self.half_open_probe_used => {
                self.half_open_probe_used = true;
                true
            }
            CircuitState::Open | CircuitState::HalfOpen => false,
        }
    }

    /// Record one attempt outcome.
    pub const fn record(&mut self, outcome: RpcOutcome, now_ms: u64) {
        if matches!(outcome, RpcOutcome::Succeeded | RpcOutcome::Rejected) {
            self.failures = 0;
            self.opened_at_ms = None;
            self.half_open_probe_used = false;
            return;
        }
        if !matches!(
            outcome,
            RpcOutcome::Unavailable | RpcOutcome::TimedOut | RpcOutcome::UnknownOutcome
        ) {
            return;
        }
        if matches!(self.state(now_ms), CircuitState::HalfOpen) {
            self.opened_at_ms = Some(now_ms);
            self.half_open_probe_used = false;
            return;
        }
        self.failures = self.failures.saturating_add(1);
        if self.failures >= self.policy.circuit_failure_threshold {
            self.opened_at_ms = Some(now_ms);
            self.half_open_probe_used = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CircuitBreaker, CircuitState, Deadline, OperationSemantics, RetryPolicy, RpcOutcome,
        retry_delay_ms, should_retry,
    };

    #[test]
    fn non_idempotent_timeout_is_never_retried() {
        assert!(!should_retry(
            RpcOutcome::UnknownOutcome,
            OperationSemantics::NonIdempotentMutation,
            1,
            RetryPolicy::default(),
        ));
        assert!(!should_retry(
            RpcOutcome::TimedOut,
            OperationSemantics::NonIdempotentMutation,
            1,
            RetryPolicy::default(),
        ));
    }

    #[test]
    fn read_only_retry_count_and_delay_are_bounded() {
        let policy = RetryPolicy::default();
        assert!(should_retry(
            RpcOutcome::TimedOut,
            OperationSemantics::ReadOnly,
            1,
            policy
        ));
        assert!(should_retry(
            RpcOutcome::Unavailable,
            OperationSemantics::ReadOnly,
            2,
            policy
        ));
        assert!(!should_retry(
            RpcOutcome::TimedOut,
            OperationSemantics::ReadOnly,
            3,
            policy
        ));
        for attempt in 1..=20 {
            assert!(retry_delay_ms(attempt, 17, policy) <= policy.maximum_delay_ms);
        }
    }

    #[test]
    fn retries_consume_one_outer_deadline() {
        let deadline = Deadline::new(1_000, 900);
        assert_eq!(deadline.remaining_ms(1_100), 800);
        assert_eq!(deadline.remaining_ms(1_850), 50);
        assert_eq!(deadline.remaining_ms(2_000), 0);
    }

    #[test]
    fn circuit_allows_exactly_one_half_open_probe() {
        let mut breaker = CircuitBreaker::new(RetryPolicy::default());
        for now in 1..=3 {
            breaker.record(RpcOutcome::Unavailable, now);
        }
        assert_eq!(breaker.state(4), CircuitState::Open);
        assert!(!breaker.allow(4));
        assert_eq!(breaker.state(5_003), CircuitState::HalfOpen);
        assert!(breaker.allow(5_003));
        assert!(!breaker.allow(5_003));
        breaker.record(RpcOutcome::Succeeded, 5_004);
        assert_eq!(breaker.state(5_004), CircuitState::Closed);
    }
}
