// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! What a thing that can actually answer looks like from here.
//!
//! Deliberately narrow. A worker is handed a request and returns an output or fails; it is given no
//! way to ask for more input, no handle to the Journal, no path, and no callback. Everything it
//! could possibly use, it was given — the same discipline the realizer has, one process further
//! out.
//!
//! No worker is implemented in this crate. That is not an omission: a broker whose only backend was
//! written beside it would have the backend's assumptions built into it, and the first real one
//! would arrive as a rewrite. `llama.cpp`, `mistral.rs` and an ONNX runtime are three different
//! shapes of process, and the interface they share is this one.

use cybou_protocol::model::{ModelManifest, ModelOutput, ModelRequest, SpendPolicy};
use uuid::Uuid;

/// One turn in an external agent's chat-completions request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChatMessage {
    /// OpenAI-compatible role name: `system`, `user`, `assistant`, or `tool`.
    pub role: String,
    /// Text already disclosed to the model gateway.
    pub content: String,
}

/// The provider-facing form of an external agent completion.
///
/// It is deliberately not a [`ModelRequest`]. Mind's closed task vocabulary remains closed; this
/// is the neighbouring surface ADR-0043 requires.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderChatRequest {
    /// Identity of this completion.
    pub request_id: Uuid,
    /// Capability class selected by the capsule lease, not a provider model name.
    pub model_class: String,
    /// Conversation turns in their original order.
    pub messages: Vec<ChatMessage>,
    /// Hard output ceiling the worker must pass to the provider.
    pub max_output_tokens: u32,
    /// What this request may spend, and whether it may spend anything.
    ///
    /// A policy rather than a remaining ceiling. `SpendPolicy::ZeroCostOnly` is a routing constraint
    /// a worker has to honour — it may only be served by a route declared to cost nothing — and no
    /// integer could have carried that: zero read as an exhausted budget, so the one request a
    /// person makes when they want a free model was the one every worker refused.
    pub spend: SpendPolicy,
}

/// What a provider worker returns for an external agent completion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderChatOutput {
    /// Assistant text.
    pub content: String,
    /// Provider-observed input usage.
    pub input_tokens: u32,
    /// Provider-observed output usage.
    pub output_tokens: u32,
    /// Provider-observed cost in the operator's smallest configured unit.
    pub spend_units: u64,
    /// Proxy-side attribution for the concrete routed deployment.
    pub upstream: Option<UpstreamAttribution>,
}

/// Provider-proxy evidence attached to one completion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpstreamAttribution {
    /// Model group requested from the proxy.
    pub model_group: String,
    /// Concrete deployment identifier returned by the proxy.
    pub deployment_id: String,
    /// Model string in the OpenAI-compatible response.
    pub response_model: String,
    /// Proxy call identifier used to join its spend log.
    pub call_id: String,
}

/// Why a worker could not answer.
///
/// A worker failing is an ordinary event, not an error condition of Mind. ADR-0035 MB6: provider
/// failure becomes a capability deficit — the faculty reports that it cannot currently answer, and
/// nothing about identity, biography or policy changes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorkerFailed {
    /// The provider answered, billed for it, and the answer may not be delivered.
    ///
    /// Its own variant rather than a plain refusal, because the two differ in the one way that
    /// matters to a ledger: nothing was spent on a refusal, and something was spent on this. A
    /// zero-cost route that bills has broken a promise — the content is withheld, and the charge is
    /// reported anyway, because money a person was not told about is worse than an answer they did
    /// not receive.
    PolicyViolatedAfterCharge {
        /// What the provider charged despite the policy.
        spend_units: u64,
        /// What was violated, in words a person can act on.
        detail: String,
    },
    /// The artifact is not loaded, or the process behind it is not running.
    NotReady,
    /// It ran and did not finish inside the time it was given.
    TimedOut {
        /// How long it was given.
        after_ms: u32,
    },
    /// It ran and produced something this task cannot use.
    ///
    /// A generative worker can always produce *something*; the failure worth reporting is that what
    /// it produced does not fit the shape the task promised. Swallowing it and returning empty
    /// output would make an unusable answer indistinguishable from an empty one.
    Unusable {
        /// What was wrong with it, for an operator rather than for a person.
        detail: String,
    },
    /// It could not run within the resources it was allowed.
    OutOfResources,
    /// This worker only implements the other model surface.
    UnsupportedSurface,
}

impl core::fmt::Display for WorkerFailed {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotReady => write!(formatter, "the model is not loaded"),
            Self::TimedOut { after_ms } => write!(formatter, "no answer within {after_ms}ms"),
            Self::Unusable { detail } => write!(formatter, "unusable answer: {detail}"),
            Self::OutOfResources => write!(formatter, "not enough resources to run"),
            Self::UnsupportedSurface => write!(formatter, "this model worker does not serve chat"),
            Self::PolicyViolatedAfterCharge {
                spend_units,
                detail,
            } => write!(
                formatter,
                "{detail}; {spend_units} unit(s) were charged and the answer was withheld"
            ),
        }
    }
}

impl core::error::Error for WorkerFailed {}

/// Something that can answer a request.
pub trait Worker: Send + Sync {
    /// What this worker runs, precisely enough to attribute an answer to it.
    fn manifest(&self) -> &ModelManifest;

    /// Answer, or say why not.
    ///
    /// # Errors
    ///
    /// Returns [`WorkerFailed`] when the artifact cannot answer this request.
    fn answer(&self, request: &ModelRequest) -> Result<ModelOutput, WorkerFailed>;

    /// Answer an external agent's chat request under the hard ceilings supplied by the gateway.
    ///
    /// A default refusal keeps a typed-only local worker valid while still letting one registered
    /// provider implement both neighbouring surfaces. A worker is never selected for chat unless
    /// its registration explicitly names the requested model class.
    ///
    /// # Errors
    ///
    /// Returns [`WorkerFailed`] when the provider cannot answer under those ceilings.
    fn answer_chat(
        &self,
        _request: &ProviderChatRequest,
    ) -> Result<ProviderChatOutput, WorkerFailed> {
        Err(WorkerFailed::UnsupportedSurface)
    }
}
