// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The model brokerage faculty (ADR-0035), and deliberately not an organ of Mind.
//!
//! It exports `org.cybou.Faculty.ModelBroker1`, in a different namespace from every `Mind1`
//! interface, because the namespace is the claim. An organ of Mind owns part of what Mind is; this
//! owns none of it. It holds no biography, reads no Journal, touches no filesystem, authorizes
//! nothing, executes nothing, and decides nothing about what is true.
//!
//! What it does is four things: select who answers, hold them to the budget, put the request, and
//! attribute what comes back.
//!
//! The claim about what it does not own is checkable rather than promised. This crate depends on
//! the protocol and the fabric and on no organ, and `scripts/validate-organ-layering.py` fails if
//! that ever changes.
//!
//! **No inference runtime is implemented here.** A broker whose only backend was written beside it
//! would have that backend's assumptions built into it, and the first real one would arrive as a
//! rewrite. `llama.cpp`, `mistral.rs` and an ONNX runtime are three different shapes of process,
//! and [`worker::Worker`] is the interface they share. On an installation with none of them
//! registered, the faculty answers every request by saying what happens instead — which is the
//! whole of what ADR-0021 means by `NoModel` being a configuration.

pub mod core;
pub mod worker;

#[cfg(target_os = "linux")]
pub mod service;

pub use core::{
    AgentChatRequest, AgentChatResult, Attempt, BrokerCore, BrokerRefused, ChatRefused, Registered,
    UsageRecord, UsageSubject,
};
pub use worker::{ChatMessage, ProviderChatOutput, ProviderChatRequest, Worker, WorkerFailed};
