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

use serde::{Deserialize, Serialize};

/// How many lines of scrollback one terminal keeps.
///
/// Held in this tab and nowhere else. A terminal buffer is the single most likely place for a
/// password typed at a prompt to end up on disk in a browser profile, so it is never persisted —
/// not to `localStorage`, not to the draft store that carries editor buffers.
pub const SCROLLBACK_LINES: usize = 1000;

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

#[cfg(test)]
mod tests {
    use super::*;

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
