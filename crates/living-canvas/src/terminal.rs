// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! What the browser knows about a terminal's screen.
//!
//! The ANSI renderer beside this one ([`crate::ansi`]) turns colour escapes into styled spans, and
//! that is the right shape for command output arriving a line at a time. It is not a terminal. It
//! has no cursor, no screen, and no memory of where anything was put, so `\r`, backspace, "clear
//! and draw at row five column ten", and every program that repaints — `vim`, `top`, `less` — come
//! out as their escape sequences flattened into a stream.
//!
//! [ADR-0047](../../../docs/adr/ADR-0047-interactive-terminal-under-the-authenticated-account.md)
//! argues for a real terminal precisely by naming those programs. Drawing their output through a
//! span parser would be shipping the thing the ADR says the Safe Shell already is.
//!
//! So the browser keeps a screen: a grid of cells with a cursor, fed bytes, read back as rows. The
//! state lives here and not in the DOM, which also means the whole of it is testable without one.

use std::fmt::Write as _;

use serde::{Deserialize, Serialize};

/// How many lines of scrollback one terminal keeps.
///
/// Held in this tab and nowhere else. A terminal buffer is the single most likely place for a
/// password typed at a prompt to end up on disk in a browser profile, so it is never persisted —
/// not to `localStorage`, not to the draft store that carries editor buffers.
pub const SCROLLBACK_LINES: usize = 1000;

/// How many columns and rows fit in a panel of this size.
///
/// The whole of the resize decision, kept away from the browser so it can be checked. A terminal
/// that never resized would leave every program laying out for eighty by twenty-four inside a panel
/// somebody has since dragged to twice that: `top` would draw a quarter of the screen, and `vim`
/// would leave the rest of it holding whatever was there before.
///
/// Clamped rather than trusted. A panel measured mid-animation, or before the browser has laid it
/// out, reports zero or something enormous; both are sizes no program should be told it has, and
/// the bounds are the protocol's so the host and the browser refuse the same ones.
#[must_use]
pub fn fitting_window(
    panel_width: f64,
    panel_height: f64,
    cell_width: f64,
    cell_height: f64,
) -> (u16, u16) {
    // A cell with no width is a font that has not loaded. Falling back keeps the terminal at a size
    // programs understand instead of dividing by it.
    if !(cell_width.is_finite() && cell_width > 0.0 && cell_height.is_finite() && cell_height > 0.0)
    {
        return (DEFAULT_COLUMNS, DEFAULT_ROWS);
    }
    if !(panel_width.is_finite() && panel_height.is_finite()) {
        return (DEFAULT_COLUMNS, DEFAULT_ROWS);
    }

    let columns = (panel_width / cell_width).floor();
    let rows = (panel_height / cell_height).floor();

    // Bounded as a float first, so the bound is arithmetic rather than a cast that happens to
    // land in range. A raw cast is where a very large panel would otherwise wrap into a very
    // small terminal.
    let clamp = |value: f64, most: u16| -> u16 {
        let bounded = value.clamp(1.0, f64::from(most));
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped into 1..=most on the line above, and most is a u16"
        )]
        let whole = bounded.trunc() as u16;
        whole.clamp(1, most)
    };

    (
        clamp(columns, cybou_protocol::terminal::MAX_COLUMNS),
        clamp(rows, cybou_protocol::terminal::MAX_ROWS),
    )
}

/// The size a terminal opens at, before anything has measured the panel.
///
/// The shape every program that draws still assumes when nothing tells it otherwise, and what a
/// terminal falls back to when the panel cannot be measured at all.
pub const DEFAULT_COLUMNS: u16 = 80;
/// See [`DEFAULT_COLUMNS`].
pub const DEFAULT_ROWS: u16 = 24;

/// One character cell, as the screen holds it.
///
/// Four independent attributes rather than one state. Bold, dim, underline and inverse are set by
/// separate escape sequences, combine freely, and a program that asked for three of them means
/// three of them; collapsing them would have to invent a precedence the terminal standard does not
/// have.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "one field per independent SGR attribute"
)]
pub struct Cell {
    /// What is in it. Empty for a blank cell.
    pub text: String,
    /// Foreground colour as a CSS value, when the cell sets one.
    pub color: Option<String>,
    /// Background colour as a CSS value, when the cell sets one.
    pub background: Option<String>,
    /// Whether the cell is bold.
    pub bold: bool,
    /// Whether the cell is dim.
    pub dim: bool,
    /// Whether the cell is underlined.
    pub underline: bool,
    /// Whether foreground and background are swapped.
    pub inverse: bool,
}

/// A terminal screen the browser can draw.
pub struct TerminalScreen {
    parser: vt100::Parser,
    columns: u16,
    rows: u16,
}

impl TerminalScreen {
    /// A screen of this size, with nothing on it.
    #[must_use]
    pub fn new(columns: u16, rows: u16) -> Self {
        Self {
            parser: vt100::Parser::new(rows, columns, SCROLLBACK_LINES),
            columns,
            rows,
        }
    }

    /// Feed bytes from the host.
    ///
    /// Bytes, not text. Output is whatever the program wrote, and decoding it as UTF-8 first would
    /// mean a `less` over a binary file, or a locale this browser did not expect, arriving as
    /// replacement characters that the screen then lays out as though they were real.
    pub fn feed(&mut self, bytes: &[u8]) {
        self.parser.process(bytes);
    }

    /// Tell the screen the window changed size.
    ///
    /// The host is told separately, over the terminal's own wire. Both have to hear it: the host
    /// so programs re-lay-out, and this so the grid the browser draws is the grid they drew into.
    pub fn resize(&mut self, columns: u16, rows: u16) {
        self.columns = columns;
        self.rows = rows;
        self.parser.screen_mut().set_size(rows, columns);
    }

    /// Where the cursor is, as a row and column.
    #[must_use]
    pub fn cursor(&self) -> (u16, u16) {
        self.parser.screen().cursor_position()
    }

    /// Whether the program asked for the cursor to be hidden.
    #[must_use]
    pub fn cursor_hidden(&self) -> bool {
        self.parser.screen().hide_cursor()
    }

    /// The screen, row by row.
    #[must_use]
    pub fn rows(&self) -> Vec<Vec<Cell>> {
        let screen = self.parser.screen();
        (0..self.rows)
            .map(|row| {
                (0..self.columns)
                    .map(|column| {
                        screen
                            .cell(row, column)
                            .map_or_else(Cell::default, |cell| Cell {
                                text: cell.contents().to_owned(),
                                color: css_colour(cell.fgcolor()),
                                background: css_colour(cell.bgcolor()),
                                bold: cell.bold(),
                                dim: cell.dim(),
                                underline: cell.underline(),
                                inverse: cell.inverse(),
                            })
                    })
                    .collect()
            })
            .collect()
    }

    /// What is on the screen as plain text, for a test or a copy.
    #[must_use]
    pub fn contents(&self) -> String {
        self.parser.screen().contents()
    }
}

/// A terminal colour as something CSS can use.
///
/// `Default` is `None` rather than a colour, so a cell that set nothing inherits the panel's own
/// foreground instead of a black this theme never chose.
fn css_colour(colour: vt100::Color) -> Option<String> {
    match colour {
        vt100::Color::Default => None,
        vt100::Color::Idx(index) => {
            Some(format!("var(--term-{index}, {})", indexed_fallback(index)))
        }
        vt100::Color::Rgb(r, g, b) => Some(format!("rgb({r} {g} {b})")),
    }
}

/// A readable default for one of the 256 indexed colours.
///
/// The first sixteen are the ones a shell prompt and `ls` actually use, so they are named; the rest
/// fall back to the cube and greyscale ramp the standard defines. A theme can override any of them
/// through the `--term-N` custom property.
fn indexed_fallback(index: u8) -> String {
    const BASE: [&str; 16] = [
        "#1e1e2e", "#f38ba8", "#a6e3a1", "#f9e2af", "#89b4fa", "#cba6f7", "#94e2d5", "#bac2de",
        "#585b70", "#f38ba8", "#a6e3a1", "#f9e2af", "#89b4fa", "#cba6f7", "#94e2d5", "#e6e9ef",
    ];
    if let Some(named) = BASE.get(index as usize) {
        return (*named).to_owned();
    }
    if index >= 232 {
        let level = 8 + (u16::from(index) - 232) * 10;
        return format!("rgb({level} {level} {level})");
    }
    let index = u16::from(index) - 16;
    let step = |value: u16| if value == 0 { 0 } else { value * 40 + 55 };
    format!(
        "rgb({} {} {})",
        step(index / 36),
        step((index / 6) % 6),
        step(index % 6)
    )
}

/// Turn one key press into the bytes a program expects to read.
///
/// Here rather than in the card, because it is the part of a terminal most worth being sure about
/// and it needs no browser to check: every one of these is a key that does nothing useful if it
/// arrives as its own name.
///
/// A terminal's input is not text. Arrow keys, Home, End and the control characters are escape
/// sequences and single bytes, and a card that sent only `key()` would deliver a terminal in which
/// nothing can be edited, interrupted or scrolled — which is most of what a terminal is for.
///
/// Returns `None` for a key that produces nothing, so a modifier held on its own does not arrive
/// as an empty write.
#[must_use]
pub fn key_to_bytes(key: &str, ctrl: bool, alt: bool) -> Option<Vec<u8>> {
    if ctrl && key.len() == 1 {
        let character = key.chars().next()?.to_ascii_uppercase();
        // Ctrl-A through Ctrl-Z, and the handful above them that a shell actually uses.
        if character.is_ascii_uppercase() {
            return Some(vec![character as u8 - b'A' + 1]);
        }
        return match character {
            '[' => Some(vec![0x1b]),
            '\\' => Some(vec![0x1c]),
            ']' => Some(vec![0x1d]),
            '@' | ' ' => Some(vec![0x00]),
            _ => None,
        };
    }

    let bytes = match key {
        "Enter" => b"\r".to_vec(),
        "Tab" => b"\t".to_vec(),
        "Backspace" => vec![0x7f],
        "Escape" => vec![0x1b],
        "Delete" => b"\x1b[3~".to_vec(),
        "ArrowUp" => b"\x1b[A".to_vec(),
        "ArrowDown" => b"\x1b[B".to_vec(),
        "ArrowRight" => b"\x1b[C".to_vec(),
        "ArrowLeft" => b"\x1b[D".to_vec(),
        "Home" => b"\x1b[H".to_vec(),
        "End" => b"\x1b[F".to_vec(),
        "PageUp" => b"\x1b[5~".to_vec(),
        "PageDown" => b"\x1b[6~".to_vec(),
        // Anything else is a character the browser already decoded for us. Keys that name
        // themselves — "Shift", "Control", "F5" — are longer than one character and produce
        // nothing, which is what stops a held modifier arriving as an empty write.
        other if other.chars().count() == 1 => other.as_bytes().to_vec(),
        _ => return None,
    };

    if alt && !bytes.is_empty() && bytes[0] != 0x1b {
        // Meta is an escape prefix, which is how every terminal has carried it since before it was
        // called Alt.
        let mut prefixed = vec![0x1b];
        prefixed.extend_from_slice(&bytes);
        return Some(prefixed);
    }
    Some(bytes)
}

/// The inline style one cell is drawn with.
///
/// A drawing decision rather than a fact about the screen, which is why it is a function over
/// a [`Cell`] rather than something the screen stores.
#[must_use]
pub fn cell_style(cell: &Cell) -> String {
    let mut style = String::new();
    // Inverse swaps the two, which is what a selection and a status line are made of. Applied here
    // rather than by the screen, because it is a drawing decision and the screen holds facts.
    let (foreground, background) = if cell.inverse {
        (cell.background.as_ref(), cell.color.as_ref())
    } else {
        (cell.color.as_ref(), cell.background.as_ref())
    };
    if let Some(colour) = foreground {
        let _ = write!(style, "color:{colour};");
    }
    if let Some(colour) = background {
        let _ = write!(style, "background:{colour};");
    }
    if cell.bold {
        style.push_str("font-weight:700;");
    }
    if cell.dim {
        style.push_str("opacity:0.6;");
    }
    if cell.underline {
        style.push_str("text-decoration:underline;");
    }
    style
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_panel_is_measured_into_whole_cells() {
        // Eight by sixteen is an ordinary monospace cell. Partial cells are dropped rather than
        // rounded up, because a column a program draws into and the panel cannot show is a column
        // that wraps.
        assert_eq!(fitting_window(800.0, 400.0, 8.0, 16.0), (100, 25));
        assert_eq!(fitting_window(807.0, 409.0, 8.0, 16.0), (100, 25));
    }

    #[test]
    fn a_panel_nobody_has_laid_out_yet_does_not_become_a_zero_sized_terminal() {
        // Zero columns is a browser that has not measured itself, and programs much older than
        // this divide by it. One is the smallest a terminal may be told it is.
        assert_eq!(fitting_window(0.0, 0.0, 8.0, 16.0), (1, 1));
        assert_eq!(fitting_window(4.0, 4.0, 8.0, 16.0), (1, 1));
    }

    #[test]
    fn a_font_that_has_not_loaded_leaves_the_terminal_at_a_size_programs_understand() {
        // Rather than dividing by it.
        assert_eq!(
            fitting_window(800.0, 400.0, 0.0, 16.0),
            (DEFAULT_COLUMNS, DEFAULT_ROWS)
        );
        assert_eq!(
            fitting_window(800.0, 400.0, f64::NAN, 16.0),
            (DEFAULT_COLUMNS, DEFAULT_ROWS)
        );
        assert_eq!(
            fitting_window(f64::INFINITY, 400.0, 8.0, 16.0),
            (DEFAULT_COLUMNS, DEFAULT_ROWS)
        );
    }

    #[test]
    fn an_enormous_panel_is_bounded_where_the_host_bounds_it() {
        // The same numbers the owner refuses past, read from the protocol rather than restated, so
        // the browser never asks for a window the host will reject.
        let (columns, rows) = fitting_window(1_000_000.0, 1_000_000.0, 8.0, 16.0);
        assert_eq!(columns, cybou_protocol::terminal::MAX_COLUMNS);
        assert_eq!(rows, cybou_protocol::terminal::MAX_ROWS);
        assert!(cybou_protocol::terminal::window_is_possible(columns, rows));
    }

    #[test]
    fn every_size_this_produces_is_one_the_host_accepts() {
        // The property behind the three tests above: whatever a panel measures, the frame that
        // follows is never one the owner closes the session over.
        for width in [0.0, 1.0, 13.0, 800.0, 5_000.0, 1_000_000.0] {
            for height in [0.0, 1.0, 9.0, 400.0, 5_000.0, 1_000_000.0] {
                let (columns, rows) = fitting_window(width, height, 8.0, 16.0);
                assert!(
                    cybou_protocol::terminal::window_is_possible(columns, rows),
                    "{width}x{height} produced {columns}x{rows}"
                );
            }
        }
    }

    #[test]
    fn a_control_key_is_a_byte_rather_than_a_letter() {
        // Ctrl-C is how a person stops a program. Sending "c" would type a letter at whatever is
        // running and leave them holding a terminal they cannot interrupt.
        assert_eq!(key_to_bytes("c", true, false), Some(vec![0x03]));
        assert_eq!(key_to_bytes("C", true, false), Some(vec![0x03]));
        assert_eq!(key_to_bytes("d", true, false), Some(vec![0x04]));
        assert_eq!(key_to_bytes("z", true, false), Some(vec![0x1a]));
        assert_eq!(key_to_bytes("[", true, false), Some(vec![0x1b]));
    }

    #[test]
    fn the_keys_that_move_and_edit_are_escape_sequences() {
        // Without these the terminal has no history, no line editing and no paging: an arrow key
        // that arrived as "ArrowUp" would type those eight letters at the prompt.
        assert_eq!(
            key_to_bytes("ArrowUp", false, false),
            Some(b"\x1b[A".to_vec())
        );
        assert_eq!(
            key_to_bytes("ArrowDown", false, false),
            Some(b"\x1b[B".to_vec())
        );
        assert_eq!(
            key_to_bytes("ArrowRight", false, false),
            Some(b"\x1b[C".to_vec())
        );
        assert_eq!(
            key_to_bytes("ArrowLeft", false, false),
            Some(b"\x1b[D".to_vec())
        );
        assert_eq!(key_to_bytes("Home", false, false), Some(b"\x1b[H".to_vec()));
        assert_eq!(
            key_to_bytes("PageUp", false, false),
            Some(b"\x1b[5~".to_vec())
        );
        assert_eq!(
            key_to_bytes("Delete", false, false),
            Some(b"\x1b[3~".to_vec())
        );
    }

    #[test]
    fn enter_is_a_carriage_return_and_backspace_is_delete() {
        // A newline instead of a carriage return gives a shell that never runs anything, and
        // backspace is 0x7f rather than 0x08 on every terminal a Linux shell expects to meet.
        assert_eq!(key_to_bytes("Enter", false, false), Some(b"\r".to_vec()));
        assert_eq!(key_to_bytes("Backspace", false, false), Some(vec![0x7f]));
        assert_eq!(key_to_bytes("Tab", false, false), Some(b"\t".to_vec()));
        assert_eq!(key_to_bytes("Escape", false, false), Some(vec![0x1b]));
    }

    #[test]
    fn alt_is_an_escape_prefix() {
        // Meta has been an escape prefix since before it was called Alt, and it is how Alt-B and
        // Alt-F move by words at a shell prompt.
        assert_eq!(key_to_bytes("b", false, true), Some(vec![0x1b, b'b']));
        // Not applied twice to something that is already an escape.
        assert_eq!(key_to_bytes("Escape", false, true), Some(vec![0x1b]));
    }

    #[test]
    fn a_key_that_is_only_a_modifier_sends_nothing() {
        // Holding shift is not input. An empty write here would be a keystroke the host has to
        // interpret, and every held modifier would produce one.
        assert_eq!(key_to_bytes("Shift", false, false), None);
        assert_eq!(key_to_bytes("Control", false, false), None);
        assert_eq!(key_to_bytes("F5", false, false), None);
        assert_eq!(key_to_bytes("", false, false), None);
    }

    #[test]
    fn an_ordinary_character_is_itself() {
        assert_eq!(key_to_bytes("a", false, false), Some(b"a".to_vec()));
        assert_eq!(key_to_bytes(" ", false, false), Some(b" ".to_vec()));
        // Decoded by the browser already, so anything it hands over arrives whole.
        assert_eq!(
            key_to_bytes("é", false, false),
            Some("é".as_bytes().to_vec())
        );
    }

    #[test]
    fn inverse_swaps_the_two_colours_rather_than_picking_one() {
        // A status line and a selection are made of this. Drawing inverse as "some other colour"
        // would make a selected line unreadable against the line beside it.
        let cell = Cell {
            text: "x".to_owned(),
            color: Some("red".to_owned()),
            background: Some("blue".to_owned()),
            inverse: true,
            ..Cell::default()
        };
        let style = cell_style(&cell);
        assert!(style.contains("color:blue;"), "{style}");
        assert!(style.contains("background:red;"), "{style}");
    }

    #[test]
    fn a_cell_that_set_nothing_is_drawn_with_nothing() {
        // So it inherits the panel, rather than this module choosing a colour the theme never did.
        assert_eq!(cell_style(&Cell::default()), "");
    }

    #[test]
    fn a_program_can_put_a_character_where_it_says_it_did() {
        // The whole reason this exists. A span parser sees these bytes as escape sequences with an
        // X after them; a terminal sees a cleared screen with an X at row five, column ten.
        let mut screen = TerminalScreen::new(80, 24);
        screen.feed(b"\x1b[2J\x1b[5;10HX");

        let rows = screen.rows();
        assert_eq!(rows[4][9].text, "X");
        assert_eq!(rows[0][0].text, "");
    }

    #[test]
    fn a_carriage_return_overwrites_rather_than_appending() {
        // A shell prompt redrawing itself, and every progress bar ever written. Flattened into a
        // stream this reads as both versions one after the other.
        let mut screen = TerminalScreen::new(20, 3);
        screen.feed(b"downloading...\rdone");

        assert!(screen.contents().starts_with("done"));
        assert!(!screen.contents().contains("downloading"));
    }

    #[test]
    fn a_backspace_removes_what_was_typed() {
        let mut screen = TerminalScreen::new(20, 3);
        screen.feed(b"cta\x08\x08at");

        assert!(screen.contents().starts_with("cat"));
    }

    #[test]
    fn colour_survives_as_something_css_can_use() {
        let mut screen = TerminalScreen::new(20, 3);
        // Bold red, then a reset, so both a set cell and an unset one are checked.
        screen.feed(b"\x1b[1;31mred\x1b[0m plain");

        let rows = screen.rows();
        assert!(rows[0][0].bold);
        assert!(rows[0][0].color.is_some());

        // A cell that set nothing inherits the panel's foreground. A colour here would be this
        // module choosing a black the theme never picked.
        assert!(!rows[0][4].bold);
        assert_eq!(rows[0][5].color, None);
    }

    #[test]
    fn a_resize_changes_the_grid_the_browser_draws() {
        let mut screen = TerminalScreen::new(80, 24);
        assert_eq!(screen.rows().len(), 24);
        assert_eq!(screen.rows()[0].len(), 80);

        screen.resize(100, 30);

        // Both numbers, because a screen that resized one of them would draw a grid no program is
        // laying out into.
        assert_eq!(screen.rows().len(), 30);
        assert_eq!(screen.rows()[0].len(), 100);
    }

    #[test]
    fn the_cursor_is_where_the_program_left_it() {
        let mut screen = TerminalScreen::new(80, 24);
        screen.feed(b"\x1b[12;40H");

        // Zero-based here, one-based in the escape sequence, which is the off-by-one every
        // terminal implementation gets to make exactly once.
        assert_eq!(screen.cursor(), (11, 39));
        assert!(!screen.cursor_hidden());

        screen.feed(b"\x1b[?25l");
        assert!(screen.cursor_hidden());
    }

    #[test]
    fn bytes_that_are_not_utf8_do_not_derail_the_screen() {
        // `less` over a binary file, or a locale this browser did not expect. Decoding to text
        // first would lay replacement characters out as though a program had drawn them.
        let mut screen = TerminalScreen::new(20, 3);
        screen.feed(&[0xff, 0xfe, b'o', b'k']);

        assert!(screen.contents().contains("ok"));
    }

    #[test]
    fn an_indexed_colour_can_be_themed_and_still_has_something_to_fall_back_to() {
        let red = css_colour(vt100::Color::Idx(1)).expect("a colour");
        assert!(red.starts_with("var(--term-1,"), "{red}");

        // Every index answers, including the greyscale ramp at the top, so a cell can never be
        // drawn from a colour this module refused to name.
        for index in 0..=255_u8 {
            assert!(css_colour(vt100::Color::Idx(index)).is_some());
        }
        assert_eq!(css_colour(vt100::Color::Default), None);
        assert_eq!(
            css_colour(vt100::Color::Rgb(1, 2, 3)).as_deref(),
            Some("rgb(1 2 3)")
        );
    }
}
