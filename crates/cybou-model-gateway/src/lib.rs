// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Lease-bound chat completions for external agents (ADR-0043).
//!
//! This is beside `ModelBroker1`, never a widening of it. Mind keeps its closed typed vocabulary;
//! an agent speaks the compatibility protocol it already knows. Both surfaces meet only below
//! their request types, at `cybou-model-brokerd`'s registered provider workers and shared usage
//! ledger.
//!
//! The bearer token held by a capsule is ephemeral authority scoped to one capsule, agent, task,
//! model class, lifetime and budget. Provider credentials never enter this crate's public types and
//! therefore cannot be handed to a capsule through this surface.

mod core;
mod http;

pub use core::{
    GatewayCompletion, GatewayCore, GatewayRefused, GatewayRequest, IssueTokenError, IssuedToken,
    TokenPolicy,
};
pub use http::router;
