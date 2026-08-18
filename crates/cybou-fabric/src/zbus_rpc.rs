// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Bounded asynchronous zbus method execution using the shared fabric policy.

use std::time::Instant;

use crate::{
    BusEndpoint,
    rpc::{
        CircuitBreaker, Deadline, OperationSemantics, RetryPolicy, RpcOutcome, retry_delay_ms,
        should_retry,
    },
};
use serde::Serialize;
use tokio::{sync::Mutex, time};
use zbus::{Connection, Message, Proxy, zvariant::DynamicType};

/// Completed D-Bus call or a typed terminal infrastructure outcome.
#[derive(Debug)]
pub struct RpcCallResult {
    /// Policy outcome after all permitted attempts.
    pub outcome: RpcOutcome,
    /// Successful raw reply for owner-specific decoding.
    pub reply: Option<Message>,
    /// Number of actual bus dispatches.
    pub attempts: u32,
    /// Transport diagnostic; never used as an authorization decision.
    pub diagnostic: Option<String>,
}

/// Async zbus client sharing one circuit per owner endpoint.
pub struct ResilientZbusClient {
    connection: Connection,
    endpoint: BusEndpoint,
    policy: RetryPolicy,
    breaker: Mutex<CircuitBreaker>,
    clock_origin: Instant,
}

impl ResilientZbusClient {
    /// Create a client around an established session-bus connection.
    ///
    /// # Panics
    ///
    /// Panics when `policy` contains invalid retry or circuit bounds.
    #[must_use]
    pub fn new(connection: Connection, endpoint: BusEndpoint, policy: RetryPolicy) -> Self {
        assert!(policy.is_valid(), "RPC retry policy must be valid");
        Self {
            connection,
            endpoint,
            policy,
            breaker: Mutex::new(CircuitBreaker::new(policy)),
            clock_origin: Instant::now(),
        }
    }

    fn now_ms(&self) -> u64 {
        u64::try_from(self.clock_origin.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    /// Execute a method within one outer deadline, including retry delays.
    pub async fn call<B>(
        &self,
        method: &str,
        body: &B,
        semantics: OperationSemantics,
        budget_ms: u64,
        deterministic_seed: u32,
    ) -> RpcCallResult
    where
        B: Serialize + DynamicType + Sync,
    {
        let deadline = Deadline::new(self.now_ms(), budget_ms.max(1));
        let mut attempts = 0;
        loop {
            let now = self.now_ms();
            let remaining_ms = deadline.remaining_ms(now);
            if remaining_ms == 0 {
                return timeout_result(semantics, attempts);
            }
            if !self.breaker.lock().await.allow(now) {
                return RpcCallResult {
                    outcome: RpcOutcome::CircuitOpen,
                    reply: None,
                    attempts,
                    diagnostic: Some("RPC circuit is open".into()),
                };
            }
            attempts += 1;
            let attempted = time::timeout(
                time::Duration::from_millis(remaining_ms),
                self.dispatch(method, body),
            )
            .await;
            let (outcome, reply, diagnostic) = match attempted {
                Ok(Ok(reply)) => (RpcOutcome::Succeeded, Some(reply), None),
                Ok(Err(error)) => {
                    let diagnostic = error.to_string();
                    (classify_zbus_error(&diagnostic), None, Some(diagnostic))
                }
                Err(_) if semantics == OperationSemantics::NonIdempotentMutation => (
                    RpcOutcome::UnknownOutcome,
                    None,
                    Some("outer RPC deadline expired after dispatch".into()),
                ),
                Err(_) => (
                    RpcOutcome::TimedOut,
                    None,
                    Some("outer RPC deadline expired".into()),
                ),
            };
            self.breaker.lock().await.record(outcome, self.now_ms());
            if !should_retry(outcome, semantics, attempts, self.policy) {
                return RpcCallResult {
                    outcome,
                    reply,
                    attempts,
                    diagnostic,
                };
            }
            let delay_ms = u64::from(retry_delay_ms(attempts, deterministic_seed, self.policy));
            let remaining_ms = deadline.remaining_ms(self.now_ms());
            if delay_ms >= remaining_ms {
                return timeout_result(semantics, attempts);
            }
            time::sleep(time::Duration::from_millis(delay_ms)).await;
        }
    }

    async fn dispatch<B>(&self, method: &str, body: &B) -> zbus::Result<Message>
    where
        B: Serialize + DynamicType,
    {
        Proxy::new(
            &self.connection,
            self.endpoint.service,
            self.endpoint.object_path,
            self.endpoint.interface,
        )
        .await?
        .call_method(method, body)
        .await
    }
}

fn timeout_result(semantics: OperationSemantics, attempts: u32) -> RpcCallResult {
    RpcCallResult {
        outcome: if semantics == OperationSemantics::NonIdempotentMutation {
            RpcOutcome::UnknownOutcome
        } else {
            RpcOutcome::TimedOut
        },
        reply: None,
        attempts,
        diagnostic: Some("outer RPC deadline exhausted".into()),
    }
}

fn classify_zbus_error(diagnostic: &str) -> RpcOutcome {
    let diagnostic = diagnostic.to_ascii_lowercase();
    if diagnostic.contains("noreply") || diagnostic.contains("timeout") {
        RpcOutcome::TimedOut
    } else if diagnostic.contains("serviceunknown")
        || diagnostic.contains("namehasnoowner")
        || diagnostic.contains("disconnected")
    {
        RpcOutcome::Unavailable
    } else {
        RpcOutcome::Rejected
    }
}

#[cfg(test)]
mod tests {
    use crate::rpc::RpcOutcome;

    use super::classify_zbus_error;

    #[test]
    fn predecessor_error_names_keep_typed_outcomes() {
        assert_eq!(
            classify_zbus_error("org.freedesktop.DBus.Error.NoReply"),
            RpcOutcome::TimedOut
        );
        assert_eq!(
            classify_zbus_error("org.freedesktop.DBus.Error.ServiceUnknown"),
            RpcOutcome::Unavailable
        );
        assert_eq!(
            classify_zbus_error("org.cybou.Mind.Error.Refused"),
            RpcOutcome::Rejected
        );
    }
}
