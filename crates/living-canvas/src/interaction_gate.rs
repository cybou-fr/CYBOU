// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The desktop, exercised in a browser.
//!
//! Everything under `components` is `cfg(target_arch = "wasm32")`, so `cargo test --workspace`
//! compiles none of it. That is not a small gap: three separate faults found on 2026-08-22 lived
//! entirely inside it and were invisible to every existing test. Clicking one Shell card selected
//! every Shell card, because selection compared a kind key. Collapsing a card destroyed the
//! terminal session inside it, because its state belonged to the component's lifetime. The minimap
//! drew cards that were docked inside decks, and its stylesheet rules did not exist at all.
//!
//! These run under `wasm-bindgen-test` in a headless Chromium, which is the only place they can
//! run. They mount real components against real signals and assert on the DOM the person would
//! have seen. They deliberately do not reach the gateway: what is under test is the desktop's own
//! behaviour, and a test that needed a Mind behind it would be a test nobody runs.

#![cfg(target_arch = "wasm32")]
#![cfg(test)]

use leptos::prelude::*;
use leptos::reactive::owner::Owner;
use wasm_bindgen::JsCast as _;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

use crate::{
    CardId, DesktopItemId, DesktopLayout, LayoutHistory,
    components::cards::{FileManagerCard, GenericToolCard},
    interaction::{DragState, ResizeState},
    state::RuntimeState,
    tool_state::ToolCardStates,
};

wasm_bindgen_test_configure!(run_in_browser);

/// A container of its own for each test, so one mount cannot see another's DOM.
fn stage() -> web_sys::HtmlElement {
    let document = web_sys::window()
        .and_then(|window| window.document())
        .expect("a document");
    let host = document.create_element("div").expect("a host element");
    document
        .body()
        .expect("a body")
        .append_child(&host)
        .expect("attach the host");
    host.dyn_into::<web_sys::HtmlElement>().expect("an element")
}

/// Everything a card component needs, with nothing behind it.
struct Desk {
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<Option<DesktopItemId>>,
    set_selected: WriteSignal<Option<DesktopItemId>>,
    dragging: RwSignal<Option<DragState>>,
    resizing: RwSignal<Option<ResizeState>>,
    runtime: RwSignal<RuntimeState>,
    auth: RwSignal<bool>,
    history: RwSignal<LayoutHistory>,
}

impl Desk {
    fn new(layout: DesktopLayout) -> Self {
        let (selected, set_selected) = signal(None);
        Self {
            layout: RwSignal::new(layout),
            selected,
            set_selected,
            dragging: RwSignal::new(None),
            resizing: RwSignal::new(None),
            runtime: RwSignal::new(RuntimeState::Loading),
            auth: RwSignal::new(false),
            history: RwSignal::new(LayoutHistory::new()),
        }
    }
}

/// An owner of a test's own, and a tool-card store inside it.
///
/// A bare browser test has no reactive owner, and `mount_to` starts a root of its own that does not
/// inherit context, so the store is created here and provided again inside whatever is mounted. The
/// returned owner must stay alive for the length of the test: dropping it disposes everything it
/// owns, including the store.
fn desk_owner() -> (Owner, ToolCardStates) {
    let owner = Owner::new();
    let states = owner.with(|| {
        let states = ToolCardStates::new();
        provide_context(states);
        states
    });
    (owner, states)
}

/// Wait for the DOM to catch up with the signals.
///
/// Leptos applies changes in effects that run after the current task, so a test that read the DOM
/// immediately after a click would be reading the frame before it. Every assertion here is about
/// what a person would have seen, which means after this.
async fn settled() {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        let _ = web_sys::window()
            .expect("a window")
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 0);
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

/// Click the header of the nth rendered card of a kind.
fn click_card(host: &web_sys::HtmlElement, kind: &str, index: usize) {
    let cards = AsRef::<web_sys::Element>::as_ref(host)
        .query_selector_all(&format!(".object.{kind}"))
        .expect("query the stage");
    let node = cards
        .item(u32::try_from(index).unwrap_or(u32::MAX))
        .expect("that card is rendered");
    let element = node.dyn_into::<web_sys::HtmlElement>().expect("an element");
    let header = element
        .query_selector("header")
        .ok()
        .flatten()
        .and_then(|header| header.dyn_into::<web_sys::HtmlElement>().ok())
        .unwrap_or(element);
    header.click();
}

/// Which of the rendered cards of a kind carry the selected class.
fn selected_flags(host: &web_sys::HtmlElement, kind: &str) -> Vec<bool> {
    let cards = AsRef::<web_sys::Element>::as_ref(host)
        .query_selector_all(&format!(".object.{kind}"))
        .expect("query the stage");
    (0..cards.length())
        .map(|index| {
            cards
                .item(index)
                .and_then(|node| node.dyn_into::<web_sys::Element>().ok())
                .is_some_and(|element| element.class_list().contains("selected"))
        })
        .collect()
}

#[wasm_bindgen_test]
async fn clicking_one_shell_card_does_not_select_the_others() {
    let (_owner, states) = desk_owner();
    // The fault this whole module exists for. Selection compared `CardId::key()`, and every Shell
    // card answers `"terminal"`, so one click marked all of them and the action bar acted on the
    // first. Nothing native could see it: all of it is component code.
    let mut layout = DesktopLayout::canonical(None);
    layout.open_card(CardId::Terminal(0), 100.0, 100.0);
    layout.open_card(CardId::Terminal(2), 900.0, 600.0);
    let desk = Desk::new(layout);
    let host = stage();

    let (layout, selected, set_selected, dragging, resizing, runtime, auth) = (
        desk.layout,
        desk.selected,
        desk.set_selected,
        desk.dragging,
        desk.resizing,
        desk.runtime,
        desk.auth,
    );
    mount_to(host.clone(), move || {
        provide_context(states);
        view! {
            <GenericToolCard card=CardId::Terminal(0) layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing auth_modal_open=auth runtime=runtime />
            <GenericToolCard card=CardId::Terminal(2) layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing auth_modal_open=auth runtime=runtime />
        }
    })
    .forget();
    settled().await;

    assert_eq!(selected_flags(&host, "terminal"), vec![false, false]);

    click_card(&host, "terminal", 1);
    settled().await;
    assert_eq!(
        selected_flags(&host, "terminal"),
        vec![false, true],
        "clicking the second Shell card selected more than the second Shell card"
    );
    assert_eq!(
        selected.get_untracked(),
        Some(DesktopItemId::Card(CardId::Terminal(2))),
        "selection did not name the card that was clicked"
    );

    click_card(&host, "terminal", 0);
    settled().await;
    assert_eq!(selected_flags(&host, "terminal"), vec![true, false]);
}

#[wasm_bindgen_test]
async fn collapsing_a_card_does_not_destroy_what_was_typed_into_it() {
    let (_owner, states) = desk_owner();
    // `CardFrame` wraps its body in a `Show`, so collapsing unmounts the content entirely. The
    // Terminal state used to be created inside that content, which made tidying the desktop a way
    // to erase a terminal session with no warning and nothing to undo.
    let mut layout = DesktopLayout::canonical(None);
    layout.open_card(CardId::Terminal(0), 100.0, 100.0);
    let desk = Desk::new(layout);
    let host = stage();

    let terminal_state = states.terminal(CardId::Terminal(0));
    terminal_state.status.set("Connected to owner".to_owned());

    let (layout, selected, set_selected, dragging, resizing, runtime, auth) = (
        desk.layout,
        desk.selected,
        desk.set_selected,
        desk.dragging,
        desk.resizing,
        desk.runtime,
        desk.auth,
    );
    mount_to(host.clone(), move || {
        provide_context(states);
        view! {
            <GenericToolCard card=CardId::Terminal(0) layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing auth_modal_open=auth runtime=runtime />
        }
    })
    .forget();
    settled().await;

    assert!(
        AsRef::<web_sys::Element>::as_ref(&host)
            .query_selector(".terminal-screen")
            .ok()
            .flatten()
            .is_some(),
        "the shell body was not rendered"
    );

    // Collapse: the body goes away entirely.
    layout.update(|layout| layout.set_collapsed(CardId::Terminal(0), true));
    settled().await;
    assert!(
        AsRef::<web_sys::Element>::as_ref(&host)
            .query_selector(".terminal-screen")
            .ok()
            .flatten()
            .is_none(),
        "collapsing did not unmount the body, so this test proves nothing"
    );

    // Expand: what the person had done is still there.
    layout.update(|layout| layout.set_collapsed(CardId::Terminal(0), false));
    settled().await;
    let output = AsRef::<web_sys::Element>::as_ref(&host)
        .query_selector(".terminal-status")
        .ok()
        .flatten()
        .expect("the shell body came back");
    let text = output.text_content().unwrap_or_default();
    assert!(
        text.contains("Connected to owner"),
        "the terminal state did not survive a collapse: {text}"
    );
    assert_eq!(terminal_state.status.get_untracked(), "Connected to owner");
}

#[wasm_bindgen_test]
fn two_tool_cards_of_one_kind_keep_separate_state() {
    let (_owner, states) = desk_owner();
    // `CardSpec` says these are not singletons. Two Terminal cards are two places a person is
    // standing, and the store is keyed by `CardId` so they cannot share one.
    let first = states.terminal(CardId::Terminal(0));
    let second = states.terminal(CardId::Terminal(1));

    first.status.set("Connected".to_owned());
    assert_eq!(second.status.get_untracked(), "Not connected");

    // And the same card asked for twice is the same state, not a new one.
    assert_eq!(
        states.terminal(CardId::Terminal(0)).status.get_untracked(),
        "Connected"
    );
}

#[wasm_bindgen_test]
fn closing_a_card_releases_what_it_had_done() {
    let (_owner, states) = desk_owner();
    // Closing is the one action that is a person saying they are finished. Everything else that
    // unmounts a card deliberately does not reach this.
    states
        .terminal(CardId::Terminal(0))
        .status
        .set("Connected".into());
    states.forget(CardId::Terminal(0));
    assert_eq!(
        states.terminal(CardId::Terminal(0)).status.get_untracked(),
        "Not connected"
    );
}

#[wasm_bindgen_test]
async fn a_card_docked_into_a_deck_is_not_drawn_standing_on_its_own() {
    let (_owner, states) = desk_owner();
    // Invariant L8. The standalone frame hides itself when its card is docked; a desktop that drew
    // both would show one card in two places at once.
    let mut layout = DesktopLayout::canonical(None);
    layout.open_card(CardId::FileManager(0), 200.0, 200.0);
    layout.open_card(CardId::Terminal(0), 100.0, 100.0);
    let desk = Desk::new(layout);
    let host = stage();

    let (layout, selected, set_selected, dragging, resizing, runtime, auth) = (
        desk.layout,
        desk.selected,
        desk.set_selected,
        desk.dragging,
        desk.resizing,
        desk.runtime,
        desk.auth,
    );
    mount_to(host.clone(), move || {
        provide_context(states);
        view! {
            <GenericToolCard card=CardId::Terminal(0) layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing auth_modal_open=auth runtime=runtime />
            <FileManagerCard layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing auth_modal_open=auth runtime=runtime instance=0 />
        }
    })
    .forget();
    settled().await;

    assert_eq!(selected_flags(&host, "terminal").len(), 1);

    layout.update(|layout| {
        let _ = layout.create_deck(
            "Shell + Files",
            vec![CardId::Terminal(0), CardId::FileManager(0)],
            120.0,
            120.0,
        );
    });
    settled().await;

    assert_eq!(
        selected_flags(&host, "terminal").len(),
        0,
        "a docked card was still drawn standing on its own"
    );
    assert_eq!(selected_flags(&host, "files").len(), 0);

    let _ = desk.history;
}

/// How many elements of a selector this mount drew.
fn count(host: &web_sys::HtmlElement, selector: &str) -> u32 {
    AsRef::<web_sys::Element>::as_ref(host)
        .query_selector_all(selector)
        .expect("query the stage")
        .length()
}

#[wasm_bindgen_test]
async fn a_card_with_no_component_of_its_own_is_still_drawn() {
    let (_owner, states) = desk_owner();
    // The defect this exists for. Twenty-one card kinds were reachable from the Dock and the
    // command palette and had no component in the viewport, so opening one added it to the layout,
    // moved the selection onto it, saved, and drew nothing at all. Not an error and not an empty
    // panel: nothing, on a desktop that had just been told to open it.
    //
    // The native test beside this one asserts that these kinds claim no component of their own.
    // Only a browser can say whether that means they are drawn.
    let mut layout = DesktopLayout::canonical(None);
    layout.open_card(CardId::SystemLogs(0), 400.0, 300.0);
    let desk = Desk::new(layout);
    let host = stage();

    let (layout, selected, set_selected, dragging, resizing, runtime, auth) = (
        desk.layout,
        desk.selected,
        desk.set_selected,
        desk.dragging,
        desk.resizing,
        desk.runtime,
        desk.auth,
    );
    mount_to(host.clone(), move || {
        provide_context(states);
        view! {
            <crate::components::cards::GenericToolCard
                card=CardId::SystemLogs(0)
                layout=layout
                selected=selected
                set_selected=set_selected
                dragging=dragging
                resizing=resizing
                auth_modal_open=auth
                runtime=runtime
            />
        }
    })
    .forget();
    settled().await;

    assert_eq!(
        count(&host, ".object.system-logs"),
        1,
        "the card is on the canvas"
    );
    // And it is the panel rather than an empty frame: the generic card dispatches through the same
    // component a Deck has always used, so what appears is the System Logs surface itself.
    assert_eq!(
        count(&host, ".system-logs-panel"),
        1,
        "with its own contents in it"
    );
}

#[wasm_bindgen_test]
async fn a_card_panned_out_of_sight_keeps_its_frame_and_drops_its_contents() {
    let (_owner, states) = desk_owner();
    // Culling, from the outside. The arithmetic is checked natively; what only a browser can say is
    // that the frame survives — the minimap, hit-testing and the tests that click a card by index
    // all depend on `.object` still being there.
    let mut layout = DesktopLayout::canonical(None);
    layout.open_card(CardId::SystemLogs(0), 400.0, 300.0);
    layout.set_position(CardId::SystemLogs(0), 90_000.0, 90_000.0);
    let desk = Desk::new(layout);
    let host = stage();

    let (layout, selected, set_selected, dragging, resizing, runtime, auth) = (
        desk.layout,
        desk.selected,
        desk.set_selected,
        desk.dragging,
        desk.resizing,
        desk.runtime,
        desk.auth,
    );
    let camera = crate::components::camera_context::CanvasCamera {
        pan: signal((0.0, 0.0)).0,
        zoom: signal(1.0).0,
        viewport: signal((1280.0, 800.0)).0,
    };
    mount_to(host.clone(), move || {
        provide_context(states);
        provide_context(camera);
        view! {
            <crate::components::cards::GenericToolCard
                card=CardId::SystemLogs(0)
                layout=layout
                selected=selected
                set_selected=set_selected
                dragging=dragging
                resizing=resizing
                auth_modal_open=auth
                runtime=runtime
            />
        }
    })
    .forget();
    settled().await;

    assert_eq!(
        count(&host, ".object.system-logs"),
        1,
        "the frame stays where the layout put it"
    );
    assert_eq!(
        count(&host, ".system-logs-panel"),
        0,
        "and its contents are not built"
    );
}

#[wasm_bindgen_test]
async fn a_narrow_window_draws_every_card_wherever_the_layout_holds_it() {
    let (_owner, states) = desk_owner();
    // The same card at the same absurd coordinate, and a phone-sized window. On a stack the cards
    // have left their coordinates — the column lays them out in order — so a panel the layout
    // happens to hold at ninety thousand pixels is simply the next one down. Culling it by where it
    // would have been on a plane would take away a card the person is looking at, which is what
    // this build would have done without the check this asserts.
    let mut layout = DesktopLayout::canonical(None);
    layout.open_card(CardId::SystemLogs(0), 400.0, 300.0);
    layout.set_position(CardId::SystemLogs(0), 90_000.0, 90_000.0);
    let desk = Desk::new(layout);
    let host = stage();

    let (layout, selected, set_selected, dragging, resizing, runtime, auth) = (
        desk.layout,
        desk.selected,
        desk.set_selected,
        desk.dragging,
        desk.resizing,
        desk.runtime,
        desk.auth,
    );
    let camera = crate::components::camera_context::CanvasCamera {
        pan: signal((0.0, 0.0)).0,
        zoom: signal(1.0).0,
        viewport: signal((390.0, 844.0)).0,
    };
    mount_to(host.clone(), move || {
        provide_context(states);
        provide_context(camera);
        view! {
            <crate::components::cards::GenericToolCard
                card=CardId::SystemLogs(0)
                layout=layout
                selected=selected
                set_selected=set_selected
                dragging=dragging
                resizing=resizing
                auth_modal_open=auth
                runtime=runtime
            />
        }
    })
    .forget();
    settled().await;

    assert_eq!(
        count(&host, ".system-logs-panel"),
        1,
        "a stacked card is drawn wherever the layout happens to hold it"
    );
}
