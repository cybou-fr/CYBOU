// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Safe zero-unsafe ANSI escape sequence parser and Leptos component renderer.

#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// Supported ANSI foreground colors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnsiColor {
    /// Black
    Black,
    /// Red
    Red,
    /// Green
    Green,
    /// Yellow
    Yellow,
    /// Blue
    Blue,
    /// Magenta
    Magenta,
    /// Cyan
    Cyan,
    /// White
    White,
    /// Bright Black (Gray)
    BrightBlack,
    /// Bright Red
    BrightRed,
    /// Bright Green
    BrightGreen,
    /// Bright Yellow
    BrightYellow,
    /// Bright Blue
    BrightBlue,
    /// Bright Magenta
    BrightMagenta,
    /// Bright Cyan
    BrightCyan,
    /// Bright White
    BrightWhite,
}

impl AnsiColor {
    /// CSS class name for this color.
    #[must_use]
    pub const fn class_name(self) -> &'static str {
        match self {
            Self::Black => "ansi-fg-black",
            Self::Red => "ansi-fg-red",
            Self::Green => "ansi-fg-green",
            Self::Yellow => "ansi-fg-yellow",
            Self::Blue => "ansi-fg-blue",
            Self::Magenta => "ansi-fg-magenta",
            Self::Cyan => "ansi-fg-cyan",
            Self::White => "ansi-fg-white",
            Self::BrightBlack => "ansi-fg-bright-black",
            Self::BrightRed => "ansi-fg-bright-red",
            Self::BrightGreen => "ansi-fg-bright-green",
            Self::BrightYellow => "ansi-fg-bright-yellow",
            Self::BrightBlue => "ansi-fg-bright-blue",
            Self::BrightMagenta => "ansi-fg-bright-magenta",
            Self::BrightCyan => "ansi-fg-bright-cyan",
            Self::BrightWhite => "ansi-fg-bright-white",
        }
    }
}

/// One styled span of terminal text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnsiSpan {
    /// Plain text characters.
    pub text: String,
    /// Active foreground color.
    pub color: Option<AnsiColor>,
    /// Whether bold text weight is enabled.
    pub bold: bool,
    /// Whether dimmed text is enabled.
    pub dim: bool,
}

/// Parse a raw terminal output string containing ANSI escape sequences into styled spans.
#[must_use]
pub fn parse_ansi(input: &str) -> Vec<AnsiSpan> {
    let mut spans = Vec::new();
    let mut current_text = String::new();
    let mut current_color = None;
    let mut current_bold = false;
    let mut current_dim = false;

    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' && chars.peek() == Some(&'[') {
            chars.next(); // Consume '['

            // Collect parameters until command byte
            let mut param_str = String::new();
            let mut cmd = ' ';
            while let Some(&next_ch) = chars.peek() {
                if next_ch.is_ascii_alphabetic() {
                    cmd = next_ch;
                    chars.next();
                    break;
                }
                param_str.push(next_ch);
                chars.next();
            }

            // SGR (Select Graphic Rendition) ends in 'm'
            if cmd == 'm' {
                if !current_text.is_empty() {
                    spans.push(AnsiSpan {
                        text: std::mem::take(&mut current_text),
                        color: current_color,
                        bold: current_bold,
                        dim: current_dim,
                    });
                }

                let codes: Vec<u32> = if param_str.trim().is_empty() {
                    vec![0]
                } else {
                    param_str
                        .split(';')
                        .filter_map(|s| s.parse::<u32>().ok())
                        .collect()
                };

                for code in codes {
                    match code {
                        0 => {
                            current_color = None;
                            current_bold = false;
                            current_dim = false;
                        }
                        1 => current_bold = true,
                        2 => current_dim = true,
                        22 => {
                            current_bold = false;
                            current_dim = false;
                        }
                        30 => current_color = Some(AnsiColor::Black),
                        31 => current_color = Some(AnsiColor::Red),
                        32 => current_color = Some(AnsiColor::Green),
                        33 => current_color = Some(AnsiColor::Yellow),
                        34 => current_color = Some(AnsiColor::Blue),
                        35 => current_color = Some(AnsiColor::Magenta),
                        36 => current_color = Some(AnsiColor::Cyan),
                        37 => current_color = Some(AnsiColor::White),
                        39 => current_color = None,
                        90 => current_color = Some(AnsiColor::BrightBlack),
                        91 => current_color = Some(AnsiColor::BrightRed),
                        92 => current_color = Some(AnsiColor::BrightGreen),
                        93 => current_color = Some(AnsiColor::BrightYellow),
                        94 => current_color = Some(AnsiColor::BrightBlue),
                        95 => current_color = Some(AnsiColor::BrightMagenta),
                        96 => current_color = Some(AnsiColor::BrightCyan),
                        97 => current_color = Some(AnsiColor::BrightWhite),
                        _ => {}
                    }
                }
            }
        } else {
            current_text.push(ch);
        }
    }

    if !current_text.is_empty() {
        spans.push(AnsiSpan {
            text: current_text,
            color: current_color,
            bold: current_bold,
            dim: current_dim,
        });
    }

    spans
}

/// Reactive component rendering parsed ANSI terminal text.
#[cfg(target_arch = "wasm32")]
#[component]
pub fn AnsiOutput(
    /// Output text to parse and render.
    #[prop(into)]
    content: Signal<String>,
    /// Whether the output represents an error state.
    #[prop(optional)]
    is_error: bool,
) -> impl IntoView {
    use leptos::prelude::*;

    let parsed_spans = Memo::new(move |_| parse_ansi(&content.get()));

    view! {
        <pre class="shell-out-text" class:error=is_error>
            <For
                each=move || parsed_spans.get().into_iter().enumerate()
                key=|(idx, span)| format!("{idx}-{}-{:?}-{}", span.text, span.color, span.bold)
                children=move |(_, span)| {
                    let color_cls = span.color.map(AnsiColor::class_name).unwrap_or_default();
                    view! {
                        <span
                            class=color_cls
                            class:ansi-bold=span.bold
                            class:ansi-dim=span.dim
                        >
                            {span.text}
                        </span>
                    }
                }
            />
        </pre>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_text_without_escapes() {
        let spans = parse_ansi("hello world\n");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "hello world\n");
        assert_eq!(spans[0].color, None);
        assert!(!spans[0].bold);
    }

    #[test]
    fn parses_colored_text_and_reset() {
        let input = "\x1b[31mError:\x1b[0m File not found";
        let spans = parse_ansi(input);
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].text, "Error:");
        assert_eq!(spans[0].color, Some(AnsiColor::Red));
        assert_eq!(spans[1].text, " File not found");
        assert_eq!(spans[1].color, None);
    }

    #[test]
    fn parses_bold_and_bright_colors() {
        let input = "\x1b[1;92mSUCCESS\x1b[0m";
        let spans = parse_ansi(input);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "SUCCESS");
        assert_eq!(spans[0].color, Some(AnsiColor::BrightGreen));
        assert!(spans[0].bold);
    }
}
