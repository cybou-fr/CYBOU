// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! A lightweight, zero-unsafe Markdown parser and Leptos renderer for Living Canvas.
//!
//! Converts Markdown text into structured DOM elements directly without string-based
//! `innerHTML` insertion. Link destinations are restricted to explicitly supported schemes.

#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// Structured Markdown block elements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MdBlock {
    /// Heading (# to ######)
    Heading {
        /// Heading depth level (1 to 6).
        level: u8,
        /// Raw heading title text.
        text: String,
    },
    /// Fenced code block (``` ... ```)
    CodeBlock {
        /// Optional programming language specifier.
        language: Option<String>,
        /// Monospace source code contents.
        content: String,
    },
    /// Blockquote (> ...)
    BlockQuote {
        /// Contiguous quoted lines.
        lines: Vec<String>,
    },
    /// Unordered or ordered list
    List {
        /// Whether the list is numbered.
        ordered: bool,
        /// List item entries.
        items: Vec<String>,
    },
    /// Standard text paragraph
    Paragraph {
        /// Paragraph body text.
        text: String,
    },
    /// Horizontal separator (---, ***, ___)
    ThematicBreak,
}

/// Inline Markdown elements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MdInline {
    /// Plain text
    Text(String),
    /// Bold text (**text** or __text__)
    Bold(String),
    /// Italic text (*text* or _text_)
    Italic(String),
    /// Inline monospace code (`code`)
    Code(String),
    /// Hyperlink [text](url)
    Link {
        /// Anchor link label text.
        text: String,
        /// Target URL destination.
        url: String,
    },
}

/// Return a link destination only when its scheme is explicitly safe for desktop navigation.
#[must_use]
pub fn allowed_link_url(url: &str) -> Option<&str> {
    let trimmed = url.trim();
    let scheme = trimmed.split_once(':')?.0;
    if scheme.eq_ignore_ascii_case("https")
        || scheme.eq_ignore_ascii_case("http")
        || scheme.eq_ignore_ascii_case("mailto")
        || scheme.eq_ignore_ascii_case("cybou")
    {
        Some(trimmed)
    } else {
        None
    }
}

/// Parse a raw Markdown document into a sequence of block elements.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn parse_markdown_blocks(input: &str) -> Vec<MdBlock> {
    let mut blocks = Vec::new();
    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        // Fenced code blocks
        if trimmed.starts_with("```") {
            let lang_str = trimmed.trim_start_matches('`').trim();
            let language = if lang_str.is_empty() {
                None
            } else {
                Some(lang_str.to_string())
            };
            let mut code_lines = Vec::new();
            i += 1;
            while i < lines.len() {
                if lines[i].trim().starts_with("```") {
                    i += 1;
                    break;
                }
                code_lines.push(lines[i]);
                i += 1;
            }
            blocks.push(MdBlock::CodeBlock {
                language,
                content: code_lines.join("\n"),
            });
            continue;
        }

        // Thematic break
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            blocks.push(MdBlock::ThematicBreak);
            i += 1;
            continue;
        }

        // Headings
        if let Some(rest) = trimmed.strip_prefix("###### ") {
            blocks.push(MdBlock::Heading {
                level: 6,
                text: rest.trim().to_string(),
            });
            i += 1;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("##### ") {
            blocks.push(MdBlock::Heading {
                level: 5,
                text: rest.trim().to_string(),
            });
            i += 1;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("#### ") {
            blocks.push(MdBlock::Heading {
                level: 4,
                text: rest.trim().to_string(),
            });
            i += 1;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("### ") {
            blocks.push(MdBlock::Heading {
                level: 3,
                text: rest.trim().to_string(),
            });
            i += 1;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("## ") {
            blocks.push(MdBlock::Heading {
                level: 2,
                text: rest.trim().to_string(),
            });
            i += 1;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("# ") {
            blocks.push(MdBlock::Heading {
                level: 1,
                text: rest.trim().to_string(),
            });
            i += 1;
            continue;
        }

        // Blockquote
        if trimmed.starts_with('>') {
            let mut quote_lines = Vec::new();
            while i < lines.len() {
                let curr = lines[i].trim();
                if let Some(rest) = curr.strip_prefix('>') {
                    quote_lines.push(rest.trim_start().to_string());
                    i += 1;
                } else {
                    break;
                }
            }
            blocks.push(MdBlock::BlockQuote { lines: quote_lines });
            continue;
        }

        // Lists: Unordered (- , * , + )
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
            let mut items = Vec::new();
            while i < lines.len() {
                let curr = lines[i].trim();
                let item_text = curr
                    .strip_prefix("- ")
                    .or_else(|| curr.strip_prefix("* "))
                    .or_else(|| curr.strip_prefix("+ "));

                if let Some(text) = item_text {
                    items.push(text.to_string());
                    i += 1;
                } else {
                    break;
                }
            }
            blocks.push(MdBlock::List {
                ordered: false,
                items,
            });
            continue;
        }

        // Lists: Ordered (1. , 2. etc.)
        if trimmed.chars().next().is_some_and(|c| c.is_ascii_digit()) && trimmed.contains(". ") {
            let mut items = Vec::new();
            while i < lines.len() {
                let curr = lines[i].trim();
                if let Some(dot_pos) = curr.find(". ") {
                    let prefix = &curr[..dot_pos];
                    if prefix.chars().all(|c| c.is_ascii_digit()) {
                        items.push(curr[dot_pos + 2..].to_string());
                        i += 1;
                        continue;
                    }
                }
                break;
            }
            if !items.is_empty() {
                blocks.push(MdBlock::List {
                    ordered: true,
                    items,
                });
                continue;
            }
        }

        // Standard Paragraph: combine contiguous non-empty lines
        let mut para_lines = Vec::new();
        while i < lines.len() {
            let curr = lines[i].trim();
            if curr.is_empty()
                || curr.starts_with('#')
                || curr.starts_with("```")
                || curr.starts_with('>')
                || curr.starts_with("- ")
                || curr.starts_with("* ")
                || curr.starts_with("+ ")
                || curr == "---"
            {
                break;
            }
            para_lines.push(curr);
            i += 1;
        }
        if !para_lines.is_empty() {
            blocks.push(MdBlock::Paragraph {
                text: para_lines.join(" "),
            });
        }
    }

    blocks
}

/// Parse a line of text into inline elements (bold, italic, code, links, text).
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn parse_markdown_inlines(input: &str) -> Vec<MdInline> {
    let mut inlines = Vec::new();
    let mut chars = input.chars().peekable();
    let mut current_text = String::new();

    while let Some(ch) = chars.next() {
        // Inline code `...`
        if ch == '`' {
            if !current_text.is_empty() {
                inlines.push(MdInline::Text(std::mem::take(&mut current_text)));
            }
            let mut code = String::new();
            let mut closed = false;
            for next_ch in chars.by_ref() {
                if next_ch == '`' {
                    closed = true;
                    break;
                }
                code.push(next_ch);
            }
            if closed {
                inlines.push(MdInline::Code(code));
            } else {
                current_text.push('`');
                current_text.push_str(&code);
            }
            continue;
        }

        // Links [text](url)
        if ch == '[' {
            if !current_text.is_empty() {
                inlines.push(MdInline::Text(std::mem::take(&mut current_text)));
            }
            let mut link_text = String::new();
            let mut has_close_bracket = false;
            for next_ch in chars.by_ref() {
                if next_ch == ']' {
                    has_close_bracket = true;
                    break;
                }
                link_text.push(next_ch);
            }

            if has_close_bracket && chars.peek() == Some(&'(') {
                chars.next(); // consume '('
                let mut url = String::new();
                let mut has_close_paren = false;
                for next_ch in chars.by_ref() {
                    if next_ch == ')' {
                        has_close_paren = true;
                        break;
                    }
                    url.push(next_ch);
                }
                if has_close_paren {
                    inlines.push(MdInline::Link {
                        text: link_text,
                        url,
                    });
                    continue;
                }
                current_text.push('[');
                current_text.push_str(&link_text);
                current_text.push(']');
                current_text.push('(');
                current_text.push_str(&url);
                continue;
            }

            current_text.push('[');
            current_text.push_str(&link_text);
            if has_close_bracket {
                current_text.push(']');
            }
            continue;
        }

        // Bold (** or __) vs Italic (* or _)
        if ch == '*' || ch == '_' {
            let is_double = chars.peek() == Some(&ch);
            if is_double {
                chars.next(); // consume second delimiter
                if !current_text.is_empty() {
                    inlines.push(MdInline::Text(std::mem::take(&mut current_text)));
                }
                let mut bold_content = String::new();
                let mut closed = false;
                while let Some(next_ch) = chars.next() {
                    if next_ch == ch && chars.peek() == Some(&ch) {
                        chars.next(); // consume second delimiter
                        closed = true;
                        break;
                    }
                    bold_content.push(next_ch);
                }
                if closed {
                    inlines.push(MdInline::Bold(bold_content));
                } else {
                    current_text.push(ch);
                    current_text.push(ch);
                    current_text.push_str(&bold_content);
                }
            } else {
                if !current_text.is_empty() {
                    inlines.push(MdInline::Text(std::mem::take(&mut current_text)));
                }
                let mut italic_content = String::new();
                let mut closed = false;
                for next_ch in chars.by_ref() {
                    if next_ch == ch {
                        closed = true;
                        break;
                    }
                    italic_content.push(next_ch);
                }
                if closed {
                    inlines.push(MdInline::Italic(italic_content));
                } else {
                    current_text.push(ch);
                    current_text.push_str(&italic_content);
                }
            }
            continue;
        }

        current_text.push(ch);
    }

    if !current_text.is_empty() {
        inlines.push(MdInline::Text(current_text));
    }

    inlines
}

#[cfg(target_arch = "wasm32")]
/// Render parsed inline tokens into Leptos view elements.
fn render_inlines(inlines: Vec<MdInline>) -> impl IntoView {
    view! {
        {inlines.into_iter().map(|item| {
            match item {
                MdInline::Text(t) => view! { <span>{t}</span> }.into_any(),
                MdInline::Bold(t) => view! { <strong class="md-bold">{t}</strong> }.into_any(),
                MdInline::Italic(t) => view! { <em class="md-italic">{t}</em> }.into_any(),
                MdInline::Code(t) => view! { <code class="md-inline-code">{t}</code> }.into_any(),
                MdInline::Link { text, url } => {
                    if let Some(url) = allowed_link_url(&url) {
                        view! {
                            <a class="md-link" href=url.to_owned() target="_blank" rel="noopener noreferrer">
                                {text}
                            </a>
                        }.into_any()
                    } else {
                        view! { <span class="md-link-blocked" title="Blocked unsafe link">{text}</span> }.into_any()
                    }
                }
            }
        }).collect::<Vec<_>>()}
    }
}

#[cfg(target_arch = "wasm32")]
/// Structured safe Markdown Preview component.
#[component]
pub fn MarkdownPreview(content: Signal<String>) -> impl IntoView {
    let parsed_blocks = Memo::new(move |_| parse_markdown_blocks(&content.get()));

    view! {
        <div class="md-preview">
            {move || {
                let blocks = parsed_blocks.get();
                if blocks.is_empty() {
                    view! { <div class="md-empty">"Document is empty."</div> }.into_any()
                } else {
                    view! {
                        <div class="md-blocks">
                            {blocks.into_iter().map(|block| {
                                match block {
                                    MdBlock::Heading { level, text } => {
                                        let inlines = parse_markdown_inlines(&text);
                                        match level {
                                            1 => view! { <h1 class="md-h1">{render_inlines(inlines)}</h1> }.into_any(),
                                            2 => view! { <h2 class="md-h2">{render_inlines(inlines)}</h2> }.into_any(),
                                            3 => view! { <h3 class="md-h3">{render_inlines(inlines)}</h3> }.into_any(),
                                            4 => view! { <h4 class="md-h4">{render_inlines(inlines)}</h4> }.into_any(),
                                            5 => view! { <h5 class="md-h5">{render_inlines(inlines)}</h5> }.into_any(),
                                            _ => view! { <h6 class="md-h6">{render_inlines(inlines)}</h6> }.into_any(),
                                        }
                                    }
                                    MdBlock::CodeBlock { language, content } => {
                                        view! {
                                            <div class="md-code-container">
                                                {language.map(|lang| {
                                                    view! { <div class="md-code-lang">{lang}</div> }
                                                })}
                                                <pre class="md-code-block"><code>{content}</code></pre>
                                            </div>
                                        }.into_any()
                                    }
                                    MdBlock::BlockQuote { lines } => {
                                        view! {
                                            <blockquote class="md-quote">
                                                {lines.into_iter().map(|l| {
                                                    let inlines = parse_markdown_inlines(&l);
                                                    view! { <p>{render_inlines(inlines)}</p> }
                                                }).collect::<Vec<_>>()}
                                            </blockquote>
                                        }.into_any()
                                    }
                                    MdBlock::List { ordered, items } => {
                                        if ordered {
                                            view! {
                                                <ol class="md-list ordered">
                                                    {items.into_iter().map(|it| {
                                                        let inlines = parse_markdown_inlines(&it);
                                                        view! { <li>{render_inlines(inlines)}</li> }
                                                    }).collect::<Vec<_>>()}
                                                </ol>
                                            }.into_any()
                                        } else {
                                            view! {
                                                <ul class="md-list unordered">
                                                    {items.into_iter().map(|it| {
                                                        let inlines = parse_markdown_inlines(&it);
                                                        view! { <li>{render_inlines(inlines)}</li> }
                                                    }).collect::<Vec<_>>()}
                                                </ul>
                                            }.into_any()
                                        }
                                    }
                                    MdBlock::Paragraph { text } => {
                                        let inlines = parse_markdown_inlines(&text);
                                        view! { <p class="md-para">{render_inlines(inlines)}</p> }.into_any()
                                    }
                                    MdBlock::ThematicBreak => {
                                        view! { <hr class="md-hr" /> }.into_any()
                                    }
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_headings_and_thematic_break() {
        let md = "# Title\n\n## Subtitle\n\n---\n### Section";
        let blocks = parse_markdown_blocks(md);
        assert_eq!(
            blocks,
            vec![
                MdBlock::Heading {
                    level: 1,
                    text: "Title".to_string()
                },
                MdBlock::Heading {
                    level: 2,
                    text: "Subtitle".to_string()
                },
                MdBlock::ThematicBreak,
                MdBlock::Heading {
                    level: 3,
                    text: "Section".to_string()
                }
            ]
        );
    }

    #[test]
    fn parses_fenced_code_blocks() {
        let md = "```rust\nfn main() {\n    println!(\"hello\");\n}\n```";
        let blocks = parse_markdown_blocks(md);
        assert_eq!(
            blocks,
            vec![MdBlock::CodeBlock {
                language: Some("rust".to_string()),
                content: "fn main() {\n    println!(\"hello\");\n}".to_string()
            }]
        );
    }

    #[test]
    fn parses_unordered_and_ordered_lists() {
        let md = "- Item A\n- Item B\n\n1. First\n2. Second";
        let blocks = parse_markdown_blocks(md);
        assert_eq!(
            blocks,
            vec![
                MdBlock::List {
                    ordered: false,
                    items: vec!["Item A".to_string(), "Item B".to_string()]
                },
                MdBlock::List {
                    ordered: true,
                    items: vec!["First".to_string(), "Second".to_string()]
                }
            ]
        );
    }

    #[test]
    fn parses_blockquotes() {
        let md = "> Line 1\n> Line 2";
        let blocks = parse_markdown_blocks(md);
        assert_eq!(
            blocks,
            vec![MdBlock::BlockQuote {
                lines: vec!["Line 1".to_string(), "Line 2".to_string()]
            }]
        );
    }

    #[test]
    fn parses_inlines_bold_italic_code_link() {
        let line = "Hello **bold** and *italic* and `code` and [Docs](https://cybou.org)";
        let inlines = parse_markdown_inlines(line);
        assert_eq!(
            inlines,
            vec![
                MdInline::Text("Hello ".to_string()),
                MdInline::Bold("bold".to_string()),
                MdInline::Text(" and ".to_string()),
                MdInline::Italic("italic".to_string()),
                MdInline::Text(" and ".to_string()),
                MdInline::Code("code".to_string()),
                MdInline::Text(" and ".to_string()),
                MdInline::Link {
                    text: "Docs".to_string(),
                    url: "https://cybou.org".to_string()
                }
            ]
        );
    }

    #[test]
    fn only_explicit_link_schemes_are_allowed() {
        assert_eq!(
            allowed_link_url("https://cybou.org"),
            Some("https://cybou.org")
        );
        assert_eq!(
            allowed_link_url("MAILTO:user@example.test"),
            Some("MAILTO:user@example.test")
        );
        assert_eq!(
            allowed_link_url("cybou:subject/agent"),
            Some("cybou:subject/agent")
        );
        assert_eq!(allowed_link_url("javascript:alert(1)"), None);
        assert_eq!(allowed_link_url("data:text/html,boom"), None);
        assert_eq!(allowed_link_url("//example.test/path"), None);
    }
}
