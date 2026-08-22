// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Perception sources implementations.

pub mod linux;
pub mod nixos;

pub use linux::{LinuxHostSource, LinuxSystemSource, parse_os_release};
pub use nixos::{NixosSystemSource, SystemGenerationSource};
