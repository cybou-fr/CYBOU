// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

use leptos::prelude::*;
use living_canvas::{MindClient, MockMindClient};

#[component]
pub fn App() -> impl IntoView {
    let (selected, set_selected) = signal("release");
    let fixture = MockMindClient::nominal_fixture().expect("checked W0 fixtures");
    let session = fixture.session().expect("fixture session");
    let snapshot = fixture.snapshot().expect("fixture snapshot");
    let mode = match session.mode {
        cybou_web_contracts::SessionMode::LocalDesktop => "Local",
        cybou_web_contracts::SessionMode::RemoteBrowser => "Remote",
    };
    let fixture_status = format!("Fixture · projection {}", snapshot.projection_version);

    view! {
        <main class="app-shell">
            <header class="topbar">
                <a class="brand" href="#canvas" aria-label="Living Canvas home">
                    <span class="brand-mark" aria-hidden="true">"◌"</span>
                    <span>"Living Canvas"</span>
                </a>
                <p class="path">"Cybou Workspace / Programs / Cybou 0.8 release"</p>
                <div class="runtime" aria-label="Runtime connection">
                    <span class="status-dot" aria-hidden="true"></span>
                    <strong>{mode}</strong>
                    <small>{fixture_status}</small>
                </div>
            </header>

            <section id="canvas" class="canvas" aria-label="Cybou living canvas">
                <div class="ambient" aria-hidden="true"></div>
                <button
                    class:selected=move || selected.get() == "artifact"
                    class="object artifact"
                    on:click=move |_| set_selected.set("artifact")
                >
                    <small>"Artifact"</small>
                    <strong>"Release evidence"</strong>
                    <span>"12 verified sources"</span>
                </button>

                <button
                    class:selected=move || selected.get() == "release"
                    class="object release"
                    on:click=move |_| set_selected.set("release")
                >
                    <small>"Release plan"</small>
                    <h1>"Cybou 0.8 release"</h1>
                    <p>"Stable release with local-first guarantees, improved reliability, and rollback safety."</p>
                    <div class="progress-label"><span>"Progress"</span><strong>"68%"</strong></div>
                    <div class="progress" aria-label="Release progress 68 percent"><span></span></div>
                    <footer><span>"Target · May 30"</span><span class="nominal">"On track"</span></footer>
                </button>

                <button
                    class:selected=move || selected.get() == "suggestion"
                    class="object suggestion"
                    on:click=move |_| set_selected.set("suggestion")
                >
                    <small>"Mind suggestion"</small>
                    <strong>"Verify rollback path"</strong>
                    <span>"Proposed · not authorized"</span>
                </button>

                <aside class="system-state" aria-label="System state">
                    <span class="status-dot" aria-hidden="true"></span>
                    "System nominal · deterministic fixture"
                </aside>
            </section>
        </main>
    }
}
