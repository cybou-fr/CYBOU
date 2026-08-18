// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

use leptos::prelude::*;
use leptos::task::spawn_local;
use living_canvas::{GatewayMindClient, MindClient};

#[derive(Clone, Debug)]
enum RuntimeState {
    Loading,
    Ready {
        mode: cybou_web_contracts::SessionMode,
        projection_version: u64,
    },
    Error(String),
}

#[component]
pub fn App() -> impl IntoView {
    let (selected, set_selected) = signal("release");
    let runtime = RwSignal::new(RuntimeState::Loading);
    spawn_local(async move {
        let client = GatewayMindClient;
        let result = async {
            let session = client.session().await?;
            let snapshot = client.snapshot().await?;
            Ok::<_, living_canvas::ClientError>((session.mode, snapshot.projection_version))
        }
        .await;
        runtime.set(match result {
            Ok((mode, projection_version)) => RuntimeState::Ready {
                mode,
                projection_version,
            },
            Err(error) => RuntimeState::Error(error.to_string()),
        });
    });

    let runtime_label = move || match runtime.get() {
        RuntimeState::Loading => "Connecting".to_owned(),
        RuntimeState::Ready { mode, .. } => match mode {
            cybou_web_contracts::SessionMode::LocalDesktop => "Local".to_owned(),
            cybou_web_contracts::SessionMode::RemoteBrowser => "Remote".to_owned(),
        },
        RuntimeState::Error(_) => "Unavailable".to_owned(),
    };
    let projection_label = move || match runtime.get() {
        RuntimeState::Loading => "Loading projection…".into(),
        RuntimeState::Ready {
            projection_version, ..
        } => format!("Gateway · projection {projection_version}"),
        RuntimeState::Error(error) => error,
    };
    let system_label = move || match runtime.get() {
        RuntimeState::Loading => "Connecting to local gateway…".into(),
        RuntimeState::Ready {
            projection_version, ..
        } => format!("System nominal · projection {projection_version}"),
        RuntimeState::Error(_) => "Gateway unavailable · canvas remains read-only".into(),
    };

    view! {
        <main class="app-shell">
            <header class="topbar">
                <a class="brand" href="#canvas" aria-label="Living Canvas home">
                    <span class="brand-mark" aria-hidden="true">"◌"</span>
                    <span>"Living Canvas"</span>
                </a>
                <p class="path">"Cybou Workspace / Programs / Cybou 0.8 release"</p>
                <div class="runtime" aria-label="Runtime connection" aria-live="polite">
                    <span class="status-dot" aria-hidden="true"></span>
                    <strong>{runtime_label}</strong>
                    <small>{projection_label}</small>
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
                    {system_label}
                </aside>
            </section>
        </main>
    }
}
