// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Agents card: what is running inside a capsule, on whose say-so, and against which ceilings.
//!
//! The launch form carries only a selection to `Agent1`: profile, agent, workspace, offered model
//! class and initial prompt. It carries no authority. The owner reads operator-approved bounds and
//! atomically admits the session against whole-host capacity; the card merely asks and shows the
//! answer.
//!
//! Four things it draws that a process list would leave out:
//!
//! - **What the lease granted**, not what the capsule is using. `memory_mib` is a ceiling the kernel
//!   enforces, and printing it beside a bar that looked like consumption would be inventing the one
//!   number a person is actually watching for.
//! - **Exactly which hosts it may reach.** An agent's egress is the part of it a person most wants
//!   enumerated, and a summary — "network: yes" — answers a question nobody asked.
//! - **Spending as of when it was seen.** *Has spent* and *had spent when last observed* are
//!   different claims, and only the second is true of anything read out of a snapshot.
//! - **The units**, so a person can go and look in their own service manager rather than take this
//!   card's word for any of it.
//!
//! ## No countdown
//!
//! [`SessionView`] carries both ends of the lease so a surface can keep its own clock. This one does
//! not: the browser build has no wall clock this crate can read, and a countdown driven by a clock
//! that is not there would tick confidently and be wrong. The two instants are shown instead, which
//! is less and is true.

use cybou_protocol::agent::{LaunchRequest, SessionView, SpendView, Standing};
use leptos::prelude::*;
use leptos::task::spawn_local;
use lucide_leptos::UsersRound;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use time::format_description::well_known::Rfc3339;
use uuid::Uuid;

use crate::{
    CardId, DesktopItemId, DesktopLayout, GatewayMindClient, MindClient,
    components::card_frame::CardFrame,
    instant::instant_label,
    interaction::{DragState, ResizeState},
    state::RuntimeState,
};

#[cfg(target_arch = "wasm32")]
async fn async_sleep(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        if let Some(window) = web_sys::window() {
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms);
        }
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

#[cfg(not(target_arch = "wasm32"))]
async fn async_sleep(_ms: i32) {}

fn replace_agents(runtime: RwSignal<RuntimeState>, sessions: Vec<SessionView>) {
    runtime.update(|state| {
        if let RuntimeState::Ready { agents, .. } = state {
            *agents = Some(sessions);
        }
    });
}

fn request_stop(
    runtime: RwSignal<RuntimeState>,
    capsule_id: Uuid,
    error: RwSignal<Option<String>>,
    mounted: Arc<AtomicBool>,
) {
    error.set(None);
    spawn_local(async move {
        let result = GatewayMindClient.stop_agent(capsule_id).await;
        if !mounted.load(Ordering::Acquire) {
            return;
        }
        match result {
            Ok(()) => match GatewayMindClient.agents().await {
                Ok(sessions) if mounted.load(Ordering::Acquire) => {
                    replace_agents(runtime, sessions);
                }
                Ok(_) => {}
                Err(why) => {
                    runtime.update(|state| {
                        if let RuntimeState::Ready { agents, .. } = state {
                            *agents = None;
                        }
                    });
                    error.set(Some(why.to_string()));
                }
            },
            Err(why) => error.set(Some(why.to_string())),
        }
    });
}

/// What the runtime holds, if it could be asked at all.
fn sessions_of(runtime: RwSignal<RuntimeState>) -> Option<Vec<SessionView>> {
    match runtime.get() {
        RuntimeState::Ready { agents, .. } => agents,
        RuntimeState::Loading | RuntimeState::Error(_) | RuntimeState::SignInRequired => None,
    }
}

/// One line saying where this host stands on agents.
///
/// Three states rather than two. *Nobody could be asked* is not *nothing is running*: on a host
/// where the agent runtime was never installed, collapsing them would tell somebody their agents
/// had stopped.
fn headline(sessions: Option<&Vec<SessionView>>) -> String {
    match sessions {
        None => "Runtime did not answer".to_owned(),
        Some(sessions) => match sessions.iter().filter(|view| view.is_live()).count() {
            0 if sessions.is_empty() => "Nothing running".to_owned(),
            0 => format!("{} finished", sessions.len()),
            1 => "1 running".to_owned(),
            live => format!("{live} running"),
        },
    }
}

/// How a standing reads to a person.
const fn standing_text(standing: Standing) -> &'static str {
    match standing {
        Standing::Launching => "starting",
        Standing::Running => "running",
        Standing::Ending => "ending",
        Standing::Ended => "ended",
    }
}

/// An instant as this card shows it, or nothing when it cannot be rendered at all.
fn moment(at: time::OffsetDateTime) -> String {
    at.format(&Rfc3339)
        .map_or_else(|_| String::new(), |text| instant_label(&text))
}

/// What was granted for money and what had been charged when somebody last looked.
///
/// An unknown figure stays unknown. The reason nobody knows is that no ledger was read, and a card
/// printing nought would be answering with the one number that is indistinguishable from a fact.
fn spend_line(session: &SessionView) -> Option<String> {
    let seen = session
        .spend_observed_at
        .map(|at| format!(" as of {}", moment(at)))
        .unwrap_or_default();
    match session.spend? {
        SpendView::Capped { limit, spent } => Some(match spent {
            Some(spent) => format!("spent {spent} of {limit}{seen}"),
            None => format!("ceiling {limit}, spending not read"),
        }),
        SpendView::ZeroCost { spent } => Some(match spent {
            // Under this policy it should be nought, and anything else means a route declared free
            // billed anyway — which a person who selected no spending is entitled to see rather
            // than have summarised away.
            Some(spent) if spent > 0 => format!("zero-cost only, yet {spent} was charged{seen}"),
            Some(_) => format!("zero-cost only, nothing charged{seen}"),
            None => "zero-cost only, spending not read".to_owned(),
        }),
    }
}

/// One session, drawn.
fn session_line(
    session: SessionView,
    runtime: RwSignal<RuntimeState>,
    error: RwSignal<Option<String>>,
    mounted: Arc<AtomicBool>,
) -> impl IntoView {
    let spend = spend_line(&session);
    let standing = standing_text(session.standing);
    let ceilings = format!(
        "{} MiB · {} cpu · {} tasks",
        session.memory_mib, session.cpus, session.tasks_max
    );
    let reach = if session.hosts.is_empty() {
        "reaches nothing".to_owned()
    } else {
        session.hosts.join(", ")
    };
    let model = session
        .model_class
        .clone()
        .unwrap_or_else(|| "no model granted".to_owned());
    let started = moment(session.started_at);
    let expires = moment(session.expires_at);
    let ended = session.ended_at.map(moment);
    let because = session.ended_because.clone();
    let capsule_id = session.capsule_id;
    let stop = session.is_live().then(|| {
        view! {
            <button
                type="button"
                class="agent-stop"
                on:click=move |_| {
                    request_stop(runtime, capsule_id, error, Arc::clone(&mounted));
                }
            >
                "Stop"
            </button>
        }
    });

    view! {
        <div class="agent-line">
            <span class="agent-head">
                <b>{session.agent.clone()}</b>
                <small class="agent-standing">{standing}</small>
                <small class="agent-profile">{session.profile.clone()}</small>
                {stop}
            </span>
            <span class="agent-workspace">
                <code>{session.workspace.clone()}</code>
            </span>
            <span class="agent-grant">
                <small class="agent-ceilings">{ceilings}</small>
                <small class="agent-model">{model}</small>
            </span>
            <span class="agent-reach">
                <small>"may reach"</small>
                <code>{reach}</code>
            </span>
            {spend.map(|spend| view! { <small class="agent-spend">{spend}</small> })}
            <span class="agent-clock">
                <small>{format!("started {started}")}</small>
                {ended
                    .map_or_else(
                        || view! { <small>{format!("lease until {expires}")}</small> },
                        |ended| view! { <small>{format!("ended {ended}")}</small> },
                    )}
            </span>
            {because.map(|because| view! { <small class="agent-because">{because}</small> })}
            <div class="agent-units">
                {session
                    .units
                    .into_iter()
                    .map(|unit| view! { <code class="agent-unit">{unit}</code> })
                    .collect_view()}
            </div>
        </div>
    }
}

/// Agents domain content presentation.
#[component]
pub fn AgentsContent(runtime: RwSignal<RuntimeState>) -> impl IntoView {
    let sessions = move || sessions_of(runtime);
    let label = move || headline(sessions().as_ref());
    let profile = RwSignal::new(String::new());
    let agent = RwSignal::new("opencode".to_owned());
    let workspace = RwSignal::new(String::new());
    let model = RwSignal::new(String::new());
    let prompt = RwSignal::new(String::new());
    let submitting = RwSignal::new(false);
    let launch_error = RwSignal::new(None::<String>);
    let mounted = Arc::new(AtomicBool::new(true));
    on_cleanup({
        let mounted = Arc::clone(&mounted);
        move || mounted.store(false, Ordering::Release)
    });
    let submit_mounted = Arc::clone(&mounted);
    let refresh_mounted = Arc::clone(&mounted);

    let submit = move |_| {
        if submitting.get_untracked() {
            return;
        }
        let request = LaunchRequest {
            profile: profile.get_untracked(),
            agent: agent.get_untracked(),
            workspace: workspace.get_untracked(),
            model_class: match model.get_untracked() {
                value if value.trim().is_empty() => None,
                value => Some(value),
            },
            prompt: prompt.get_untracked(),
        };
        let mounted = Arc::clone(&submit_mounted);
        submitting.set(true);
        launch_error.set(None);
        spawn_local(async move {
            let result = GatewayMindClient.launch_agent(&request).await;
            if !mounted.load(Ordering::Acquire) {
                return;
            }
            match result {
                Ok(launched) => {
                    let capsule_id = launched.capsule_id;
                    runtime.update(|state| {
                        if let RuntimeState::Ready { agents, .. } = state {
                            let sessions = agents.get_or_insert_with(Vec::new);
                            sessions.retain(|session| session.capsule_id != capsule_id);
                            sessions.push(launched);
                        }
                    });
                    submitting.set(false);

                    // Agent1 is the owner of every later transition. Refresh while this launch is
                    // live so `launching`, `running`, a newly observed spend, and disappearance
                    // after teardown are never replaced by the optimistic receipt above. Bounded
                    // to five minutes: long sessions keep their last truthful view and can be
                    // refreshed explicitly without turning one click into permanent polling.
                    for _ in 0..300 {
                        async_sleep(1_000).await;
                        if !mounted.load(Ordering::Acquire) {
                            break;
                        }
                        match GatewayMindClient.agents().await {
                            Ok(sessions) => {
                                let still_held = sessions.iter().any(|session| {
                                    session.capsule_id == capsule_id && session.is_live()
                                });
                                replace_agents(runtime, sessions);
                                if !still_held {
                                    break;
                                }
                            }
                            Err(error) => {
                                runtime.update(|state| {
                                    if let RuntimeState::Ready { agents, .. } = state {
                                        *agents = None;
                                    }
                                });
                                launch_error.set(Some(error.to_string()));
                                break;
                            }
                        }
                    }
                }
                Err(error) => {
                    launch_error.set(Some(error.to_string()));
                    submitting.set(false);
                }
            }
        });
    };

    let refresh = move |_| {
        let mounted = Arc::clone(&refresh_mounted);
        launch_error.set(None);
        spawn_local(async move {
            let result = GatewayMindClient.agents().await;
            if !mounted.load(Ordering::Acquire) {
                return;
            }
            match result {
                Ok(sessions) => replace_agents(runtime, sessions),
                Err(error) => {
                    runtime.update(|state| {
                        if let RuntimeState::Ready { agents, .. } = state {
                            *agents = None;
                        }
                    });
                    launch_error.set(Some(error.to_string()));
                }
            }
        });
    };
    let list_mounted = Arc::clone(&mounted);

    view! {
        <div class="card-body agents-body">
            <div class="agents-headline">
                <b>{label}</b>
            </div>
            <div class="agent-launch-form">
                <input
                    aria-label="Profile"
                    placeholder="Profile"
                    prop:value=move || profile.get()
                    on:input=move |event| profile.set(event_target_value(&event))
                />
                <input
                    aria-label="Agent"
                    placeholder="Agent"
                    prop:value=move || agent.get()
                    on:input=move |event| agent.set(event_target_value(&event))
                />
                <input
                    aria-label="Workspace"
                    placeholder="Workspace"
                    prop:value=move || workspace.get()
                    on:input=move |event| workspace.set(event_target_value(&event))
                />
                <input
                    aria-label="Model class"
                    placeholder="Model class"
                    prop:value=move || model.get()
                    on:input=move |event| model.set(event_target_value(&event))
                />
                <textarea
                    aria-label="Initial prompt"
                    placeholder="What should the agent do?"
                    prop:value=move || prompt.get()
                    on:input=move |event| prompt.set(event_target_value(&event))
                />
                <button type="button" on:click=submit disabled=move || submitting.get()>
                    {move || if submitting.get() { "Launching…" } else { "Launch agent" }}
                </button>
                <button type="button" on:click=refresh>"Refresh sessions"</button>
                {move || launch_error.get().map(|error| view! { <small class="agent-launch-error">{error}</small> })}
            </div>
            <div class="agent-list">
                {move || {
                    let mounted = Arc::clone(&list_mounted);
                    sessions()
                        .unwrap_or_default()
                        .into_iter()
                        .map(move |session| {
                            session_line(
                                session,
                                runtime,
                                launch_error,
                                Arc::clone(&mounted),
                            )
                        })
                        .collect_view()
                }}
            </div>
        </div>
    }
}

/// Agents cognitive card component.
#[component]
pub fn AgentsCard(
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<Option<DesktopItemId>>,
    set_selected: WriteSignal<Option<DesktopItemId>>,
    dragging: RwSignal<Option<DragState>>,
    resizing: RwSignal<Option<ResizeState>>,
    runtime: RwSignal<RuntimeState>,
) -> impl IntoView {
    let collapsed = move || {
        let sessions = sessions_of(runtime);
        let label = headline(sessions.as_ref());
        view! {
            <div class="card-collapsed-summary">
                <b>"Agents"</b>
                <span>{label}</span>
            </div>
        }
        .into_any()
    };

    view! {
        <CardFrame
            card=CardId::Agents
            layout=layout
            selected=selected
            set_selected=set_selected
            dragging=dragging
            resizing=resizing
            kicker_title="Agent1"
            kicker_icon=Arc::new(|| view! { <UsersRound size=14 /> }.into_any())
            collapsed_summary=Arc::new(collapsed)
        >
            <AgentsContent runtime=runtime />
        </CardFrame>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn at(offset: i64) -> time::OffsetDateTime {
        time::OffsetDateTime::from_unix_timestamp(1_787_000_000 + offset).expect("a fixed instant")
    }

    fn view() -> SessionView {
        SessionView {
            capsule_id: Uuid::from_u128(0x0c01),
            agent: "opencode".to_owned(),
            profile: "sandboxed-autonomous".to_owned(),
            workspace: "/srv/project".to_owned(),
            standing: Standing::Running,
            ended_because: None,
            started_at: at(0),
            expires_at: at(4 * 60 * 60),
            ended_at: None,
            model_class: Some("Strong".to_owned()),
            spend: Some(SpendView::Capped {
                limit: 100,
                spent: Some(42),
            }),
            spend_observed_at: Some(at(120)),
            memory_mib: 4096,
            cpus: 2,
            tasks_max: 512,
            hosts: vec!["github.com".to_owned()],
            units: vec!["cybou-capsule-x.service".to_owned()],
        }
    }

    #[test]
    fn nobody_answering_is_not_nothing_running() {
        // The distinction the whole card rests on. On a host without the agent runtime, one line
        // says the surface could not be asked and the other says somebody's agents have stopped.
        assert_eq!(headline(None), "Runtime did not answer");
        assert_eq!(headline(Some(&Vec::new())), "Nothing running");
    }

    #[test]
    fn a_finished_session_is_not_counted_as_running() {
        let mut ended = view();
        ended.standing = Standing::Ended;
        assert_eq!(headline(Some(&vec![ended])), "1 finished");
        assert_eq!(headline(Some(&vec![view()])), "1 running");
    }

    #[test]
    fn an_unread_ledger_says_so_rather_than_nought() {
        let mut unread = view();
        unread.spend = Some(SpendView::Capped {
            limit: 100,
            spent: None,
        });
        let line = spend_line(&unread).expect("a ceiling was granted");
        assert!(line.contains("not read"), "{line}");
        assert!(!line.contains('0'), "{line}");
    }

    #[test]
    fn a_free_route_that_billed_is_shown_rather_than_summarised_away() {
        // The one number worth drawing under this policy: it should be nought, and anything else
        // means something declared free charged for it.
        let mut billed = view();
        billed.spend = Some(SpendView::ZeroCost { spent: Some(7) });
        let line = spend_line(&billed).expect("a policy was granted");
        assert!(line.contains('7'), "{line}");
    }

    #[test]
    fn a_figure_carries_when_it_was_seen() {
        let line = spend_line(&view()).expect("a ceiling was granted");
        assert!(line.contains("as of"), "{line}");
    }

    #[test]
    fn an_instant_reads_as_a_person_would_write_it() {
        assert_eq!(moment(at(0)), "2026-08-13 18:13:20 UTC");
    }
}
