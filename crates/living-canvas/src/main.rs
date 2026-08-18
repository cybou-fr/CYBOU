// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Browser entry point for the Living Canvas WebAssembly application.

#[cfg(target_arch = "wasm32")]
mod web_app;

#[cfg(target_arch = "wasm32")]
fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(web_app::App);
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("living-canvas is a wasm32 browser application");
}
