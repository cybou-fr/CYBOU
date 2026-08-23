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

use cybou_protocol::model::{ModelManifest, ModelOutput, ModelRequest};

/// Why a worker could not answer.
///
/// A worker failing is an ordinary event, not an error condition of Mind. ADR-0035 MB6: provider
/// failure becomes a capability deficit — the faculty reports that it cannot currently answer, and
/// nothing about identity, biography or policy changes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorkerFailed {
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
}

impl core::fmt::Display for WorkerFailed {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotReady => write!(formatter, "the model is not loaded"),
            Self::TimedOut { after_ms } => write!(formatter, "no answer within {after_ms}ms"),
            Self::Unusable { detail } => write!(formatter, "unusable answer: {detail}"),
            Self::OutOfResources => write!(formatter, "not enough resources to run"),
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
}
