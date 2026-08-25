// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Authentication, lease accounting and the handoff to the shared provider pool.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use cybou_capsule::{Lease, SpendPolicy};
use cybou_model_brokerd::{
    AgentChatRequest, AgentChatResult, BrokerCore, ChatMessage, ChatRefused,
};
use cybou_protocol::model::ModelUsageSnapshot;
use sha2::{Digest as _, Sha256};
use time::OffsetDateTime;
use uuid::Uuid;

/// How much authority the token issuer gives one external-agent task.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TokenPolicy {
    /// Whether the prompt must stay on this device.
    pub local_only: bool,
    /// Most exposing content the token may carry; route policy must accept at least this class.
    pub sensitivity: u8,
    /// Per-request output ceiling.
    pub max_output_tokens: u32,
    /// Total input plus output tokens this token may consume.
    pub token_limit: u64,
}

/// A chat request after the HTTP compatibility shape has been parsed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GatewayRequest {
    /// Request identity generated at the gateway.
    pub request_id: Uuid,
    /// Model capability class, which must equal the lease grant.
    pub model_class: String,
    /// Conversation in original order.
    pub messages: Vec<ChatMessage>,
    /// Requested output ceiling.
    pub max_output_tokens: u32,
}

/// A completion with the attribution needed to build the compatibility response.
#[derive(Clone, Debug, PartialEq)]
pub struct GatewayCompletion {
    /// Shared-provider result.
    pub result: AgentChatResult,
    /// Capability class the agent requested and was granted.
    pub model_class: String,
}

/// The one time a newly issued bearer secret is visible.
pub struct IssuedToken {
    secret: String,
    /// Capsule-lease expiry; the token can end earlier if the lease is revoked.
    pub expires_at: OffsetDateTime,
}

impl IssuedToken {
    /// Reveal the secret for injection into the capsule environment.
    #[must_use]
    pub fn expose_secret(&self) -> &str {
        &self.secret
    }
}

impl core::fmt::Debug for IssuedToken {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("IssuedToken")
            .field("secret", &"[REDACTED]")
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

/// Why an ephemeral model token could not be issued.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IssueTokenError {
    /// Capsule lease already ended.
    LeaseEnded,
    /// Capsule profile grants no model.
    NoModelGrant,
    /// A zero token or request ceiling would create unusable authority.
    EmptyBudget,
    /// The operating system did not provide secret-grade randomness.
    RandomnessUnavailable,
    /// The bounded in-memory token table is full of live entries.
    TokenTableFull,
}

impl core::fmt::Display for IssueTokenError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::LeaseEnded => formatter.write_str("the capsule lease has ended"),
            Self::NoModelGrant => formatter.write_str("the capsule lease grants no model"),
            Self::EmptyBudget => formatter.write_str("the model token budget permits nothing"),
            Self::RandomnessUnavailable => {
                formatter.write_str("the operating system did not provide token randomness")
            }
            Self::TokenTableFull => formatter.write_str("the live model token table is full"),
        }
    }
}

impl core::error::Error for IssueTokenError {}

/// Why a chat request was refused.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GatewayRefused {
    /// Missing, unknown, expired or revoked bearer. Kept indistinguishable at the boundary.
    Unauthorized,
    /// The request named something other than the lease's model class.
    ModelClassNotGranted,
    /// The request shape cannot be given to a provider.
    InvalidRequest(&'static str),
    /// Token, lease or per-request budget would be exceeded.
    BudgetExceeded,
    /// Shared provider policy or capability could not answer.
    Provider(ChatRefused),
}

impl core::fmt::Display for GatewayRefused {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Unauthorized => formatter.write_str("invalid or expired model token"),
            Self::ModelClassNotGranted => formatter.write_str("model class is not granted"),
            Self::InvalidRequest(reason) => formatter.write_str(reason),
            Self::BudgetExceeded => formatter.write_str("model lease budget would be exceeded"),
            Self::Provider(reason) => write!(formatter, "{reason}"),
        }
    }
}

impl core::error::Error for GatewayRefused {}

struct TokenAccount {
    digest: [u8; 32],
    lease: Arc<Mutex<Lease>>,
    task_id: Uuid,
    policy: TokenPolicy,
    tokens_spent: u64,
}

/// Authenticated gateway core sharing the broker's provider registrations and usage ledger.
pub struct GatewayCore {
    providers: Arc<BrokerCore>,
    tokens: Mutex<Vec<TokenAccount>>,
    /// How many completions this gateway has served, across every token it issued.
    ///
    /// Counted here rather than derived from the token table, because a token can be dropped when a
    /// task ends and the completions it served still happened. A figure that fell when a table was
    /// tidied would be a count of what is remembered rather than of what was done.
    completions: AtomicU64,
}

const TOKEN_BYTES: usize = 32;
const MAX_LIVE_TOKENS: usize = 1024;

impl GatewayCore {
    /// Create the neighbouring surface over the exact provider core used by `ModelBroker1`.
    #[must_use]
    pub fn new(providers: Arc<BrokerCore>) -> Self {
        Self {
            providers,
            tokens: Mutex::new(Vec::new()),
            completions: AtomicU64::new(0),
        }
    }

    /// The shared provider core, exposed for attribution and operational inspection.
    #[must_use]
    pub fn providers(&self) -> &Arc<BrokerCore> {
        &self.providers
    }

    /// What this gateway has actually spent, for whoever owns the session.
    ///
    /// The only truthful source for that figure. A session owner holds the grant a person approved
    /// and never the ledger — this process received the lease as bytes and charges its own copy — so
    /// an owner reading its own lease would report nought for a session that had been billed.
    ///
    /// Stamped with the instant it was taken, because *has spent* and *had spent when last observed*
    /// are different claims and only the second is true of anything read elsewhere afterwards.
    ///
    /// # Errors
    ///
    /// Returns nothing and reports `None` when the lease or the token table cannot be reached; a
    /// figure that guessed past a poisoned lock would be a figure nobody measured.
    #[must_use]
    pub fn usage(
        &self,
        lease: &Arc<Mutex<Lease>>,
        now: OffsetDateTime,
    ) -> Option<ModelUsageSnapshot> {
        let lease = lease.lock().ok()?;
        let tokens = self
            .tokens
            .lock()
            .ok()?
            .iter()
            .fold(0_u64, |total, account| {
                total.saturating_add(account.tokens_spent)
            });
        Some(ModelUsageSnapshot {
            capsule_id: lease.grant().capsule_id,
            spend_units: lease.model_spent(),
            tokens,
            completions: self.completions.load(Ordering::Relaxed),
            observed_at: now,
        })
    }

    /// Mint an ephemeral bearer for one task under one live capsule lease.
    ///
    /// # Errors
    ///
    /// Returns [`IssueTokenError`] when the lease is ended, grants no model, the policy permits
    /// nothing, secure randomness is unavailable, or the bounded live-token table is full.
    pub fn issue_token(
        &self,
        lease: Arc<Mutex<Lease>>,
        task_id: Uuid,
        policy: TokenPolicy,
        now: OffsetDateTime,
    ) -> Result<IssuedToken, IssueTokenError> {
        if policy.token_limit == 0 || policy.max_output_tokens == 0 {
            return Err(IssueTokenError::EmptyBudget);
        }
        let expires_at = {
            let lease = lease.lock().map_err(|_| IssueTokenError::LeaseEnded)?;
            if !lease.is_live(now) {
                return Err(IssueTokenError::LeaseEnded);
            }
            if lease.grant().model.is_none() {
                return Err(IssueTokenError::NoModelGrant);
            }
            lease.expires_at()
        };

        let mut random = [0_u8; TOKEN_BYTES];
        getrandom::getrandom(&mut random).map_err(|_| IssueTokenError::RandomnessUnavailable)?;
        let secret = random.iter().fold(
            String::with_capacity(6 + TOKEN_BYTES * 2),
            |mut value, byte| {
                use std::fmt::Write as _;
                let _ = write!(value, "{byte:02x}");
                value
            },
        );
        let secret = format!("cybou_{secret}");

        let mut tokens = self.tokens.lock().unwrap_or_else(PoisonError::into_inner);
        tokens.retain(|account| account.lease.lock().is_ok_and(|lease| lease.is_live(now)));
        if tokens.len() >= MAX_LIVE_TOKENS {
            return Err(IssueTokenError::TokenTableFull);
        }
        tokens.push(TokenAccount {
            digest: digest(&secret),
            lease,
            task_id,
            policy,
            tokens_spent: 0,
        });
        Ok(IssuedToken { secret, expires_at })
    }

    /// Complete one authenticated request and atomically charge its capsule lease.
    ///
    /// # Errors
    ///
    /// Returns [`GatewayRefused`] for authentication, request, budget, policy or provider failure.
    pub fn complete(
        &self,
        bearer: &str,
        request: &GatewayRequest,
        now: OffsetDateTime,
    ) -> Result<GatewayCompletion, GatewayRefused> {
        validate_request(request)?;
        let wanted = digest(bearer);
        let mut tokens = self
            .tokens
            .lock()
            .map_err(|_| GatewayRefused::Unauthorized)?;
        let account = tokens
            .iter_mut()
            .find(|account| account.digest == wanted)
            .ok_or(GatewayRefused::Unauthorized)?;
        let mut lease = account
            .lease
            .lock()
            .map_err(|_| GatewayRefused::Unauthorized)?;
        if !lease.is_live(now) {
            return Err(GatewayRefused::Unauthorized);
        }
        let grant = lease
            .grant()
            .model
            .as_ref()
            .ok_or(GatewayRefused::Unauthorized)?;
        if grant.class != request.model_class {
            return Err(GatewayRefused::ModelClassNotGranted);
        }
        if request.max_output_tokens > account.policy.max_output_tokens {
            return Err(GatewayRefused::BudgetExceeded);
        }

        let input_tokens = reserved_input_tokens(&request.messages);
        let reservation = u64::from(input_tokens) + u64::from(request.max_output_tokens);
        if reservation
            > account
                .policy
                .token_limit
                .saturating_sub(account.tokens_spent)
        {
            return Err(GatewayRefused::BudgetExceeded);
        }
        // What is left of the grant, expressed the same way the grant expresses it. Handing the
        // broker a remaining *number* was how "spend nothing, on something that costs nothing"
        // arrived at the transport indistinguishable from "you have spent everything".
        let spend = match grant.spend {
            SpendPolicy::ZeroCostOnly => SpendPolicy::ZeroCostOnly,
            SpendPolicy::Capped(_) => {
                SpendPolicy::Capped(grant.spend.remaining(lease.model_spent()))
            }
        };

        let result = self
            .providers
            .submit_agent_chat(&AgentChatRequest {
                request_id: request.request_id,
                capsule_id: lease.grant().capsule_id,
                agent: lease.grant().agent.clone(),
                task_id: account.task_id,
                model_class: request.model_class.clone(),
                messages: request.messages.clone(),
                input_tokens,
                max_output_tokens: request.max_output_tokens,
                spend,
                local_only: account.policy.local_only,
                sensitivity: account.policy.sensitivity,
            })
            .map_err(|refused| {
                // A refusal that cost money is charged before it is returned. Otherwise a provider
                // that billed and then broke its policy would leave the ledger reading nought, and
                // a person who selected a spending bound would be told they had spent nothing while
                // their account said otherwise. Money already gone is a fact about the session, not
                // a property of whether the answer was delivered.
                if let cybou_model_brokerd::ChatRefused::WorkerFailed {
                    failure:
                        cybou_model_brokerd::WorkerFailed::PolicyViolatedAfterCharge {
                            spend_units, ..
                        },
                    ..
                } = &refused
                {
                    lease.charge(*spend_units);
                }
                GatewayRefused::Provider(refused)
            })?;

        let charged_tokens = u64::from(result.input_tokens) + u64::from(result.output_tokens);
        account.tokens_spent = account.tokens_spent.saturating_add(charged_tokens);
        lease.charge(result.spend_units);
        self.completions.fetch_add(1, Ordering::Relaxed);
        Ok(GatewayCompletion {
            result,
            model_class: request.model_class.clone(),
        })
    }
}

fn validate_request(request: &GatewayRequest) -> Result<(), GatewayRefused> {
    if request.messages.is_empty() {
        return Err(GatewayRefused::InvalidRequest("messages must not be empty"));
    }
    if request.max_output_tokens == 0 {
        return Err(GatewayRefused::InvalidRequest(
            "max completion tokens must be positive",
        ));
    }
    if request.model_class.trim().is_empty() || request.model_class.trim() != request.model_class {
        return Err(GatewayRefused::InvalidRequest("model must name one class"));
    }
    for message in &request.messages {
        if !matches!(
            message.role.as_str(),
            "system" | "user" | "assistant" | "tool"
        ) {
            return Err(GatewayRefused::InvalidRequest(
                "message role is unsupported",
            ));
        }
    }
    Ok(())
}

fn reserved_input_tokens(messages: &[ChatMessage]) -> u32 {
    // UTF-8 bytes are a conservative upper bound for ordinary provider tokenizers. Reserving the
    // larger number is intentional: accepting the agent's own token count would make the executor
    // grade its own budget.
    messages
        .iter()
        .map(|message| message.role.len().saturating_add(message.content.len()))
        .fold(0_usize, usize::saturating_add)
        .try_into()
        .unwrap_or(u32::MAX)
}

fn digest(secret: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hasher.finalize().into()
}
