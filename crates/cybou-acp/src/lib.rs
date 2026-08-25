// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! ACP client and read-only browser for the upstream ACP agent registry.

mod client;
mod registry;
mod session;

pub use client::{AcpClient, AcpClientError, AgentHandshake, AuthenticationMethod};
pub use registry::{
    RegistryAgent, RegistryBrowser, RegistryError, RegistryIndex, RegistrySnapshot,
    UPSTREAM_REGISTRY_URL,
};
pub use session::{AcpSession, AgentTurn};
