// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Bounded line-oriented diff construction for the read-only Diff Viewer.

const CONTEXT_LINES: usize = 3;
const MAX_EDIT_DISTANCE: usize = 512;
const MAX_LINES: usize = 20_000;

/// Semantic kind of one rendered diff line.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiffLineKind {
    /// An unchanged context line.
    Context,
    /// A line present only in the original text.
    Delete,
    /// A line present only in the proposed text.
    Add,
}

/// One line inside a unified diff hunk.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiffLine {
    /// Line number in the original text, when applicable.
    pub old_line: Option<usize>,
    /// Line number in the proposed text, when applicable.
    pub new_line: Option<usize>,
    /// How the line participates in the change.
    pub kind: DiffLineKind,
    /// Line content without its terminator.
    pub content: String,
}

/// A contiguous unified diff hunk.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiffHunk {
    /// First original line covered by the hunk.
    pub old_start: usize,
    /// Number of original lines covered by the hunk.
    pub old_len: usize,
    /// First proposed line covered by the hunk.
    pub new_start: usize,
    /// Number of proposed lines covered by the hunk.
    pub new_len: usize,
    /// Context and changed lines in display order.
    pub lines: Vec<DiffLine>,
}

/// Complete result of a bounded diff computation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextDiff {
    /// Unified hunks. Empty means the inputs are identical.
    pub hunks: Vec<DiffHunk>,
    /// Whether the safety bound required a coarse delete/add fallback.
    pub used_fallback: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Edit {
    Equal(usize, usize),
    Delete(usize),
    Add(usize),
}

/// Build a bounded, line-oriented unified diff.
#[must_use]
pub fn build_text_diff(original: &str, proposed: &str) -> TextDiff {
    let old = split_lines(original);
    let new = split_lines(proposed);
    let (edits, used_fallback) = if old.len() > MAX_LINES || new.len() > MAX_LINES {
        (coarse_edits(old.len(), new.len()), true)
    } else if let Some(edits) = myers_edits(&old, &new) {
        (edits, false)
    } else {
        (coarse_edits(old.len(), new.len()), true)
    };

    TextDiff {
        hunks: make_hunks(&edits, &old, &new),
        used_fallback,
    }
}

fn split_lines(text: &str) -> Vec<&str> {
    if text.is_empty() {
        Vec::new()
    } else {
        text.split('\n').collect()
    }
}

fn coarse_edits(old_len: usize, new_len: usize) -> Vec<Edit> {
    (0..old_len)
        .map(Edit::Delete)
        .chain((0..new_len).map(Edit::Add))
        .collect()
}

#[allow(clippy::many_single_char_names)]
fn myers_edits(old: &[&str], new: &[&str]) -> Option<Vec<Edit>> {
    let n = isize::try_from(old.len()).ok()?;
    let m = isize::try_from(new.len()).ok()?;
    let maximum = old.len().saturating_add(new.len()).min(MAX_EDIT_DISTANCE);
    let offset = isize::try_from(MAX_EDIT_DISTANCE + 1).ok()?;
    let mut frontier = vec![0_isize; MAX_EDIT_DISTANCE * 2 + 3];
    let mut trace = Vec::with_capacity(maximum + 1);

    for distance in 0..=maximum {
        trace.push(frontier.clone());
        let d = isize::try_from(distance).ok()?;
        for diagonal in (-d..=d).step_by(2) {
            let index = usize::try_from(offset + diagonal).ok()?;
            let mut x =
                if diagonal == -d || (diagonal != d && frontier[index - 1] < frontier[index + 1]) {
                    frontier[index + 1]
                } else {
                    frontier[index - 1] + 1
                };
            let mut y = x - diagonal;
            while x < n && y < m && old[usize::try_from(x).ok()?] == new[usize::try_from(y).ok()?] {
                x += 1;
                y += 1;
            }
            frontier[index] = x;
            if x >= n && y >= m {
                return backtrack(&trace, old.len(), new.len(), offset);
            }
        }
    }
    None
}

fn backtrack(
    trace: &[Vec<isize>],
    old_len: usize,
    new_len: usize,
    offset: isize,
) -> Option<Vec<Edit>> {
    let mut x = isize::try_from(old_len).ok()?;
    let mut y = isize::try_from(new_len).ok()?;
    let mut reversed = Vec::with_capacity(old_len + new_len);

    for distance in (0..trace.len()).rev() {
        let d = isize::try_from(distance).ok()?;
        let diagonal = x - y;
        let frontier = &trace[distance];
        let index = usize::try_from(offset + diagonal).ok()?;
        let previous_diagonal =
            if diagonal == -d || (diagonal != d && frontier[index - 1] < frontier[index + 1]) {
                diagonal + 1
            } else {
                diagonal - 1
            };
        let previous_x = frontier[usize::try_from(offset + previous_diagonal).ok()?];
        let previous_y = previous_x - previous_diagonal;

        while x > previous_x && y > previous_y {
            x -= 1;
            y -= 1;
            reversed.push(Edit::Equal(
                usize::try_from(x).ok()?,
                usize::try_from(y).ok()?,
            ));
        }
        if distance == 0 {
            break;
        }
        if x == previous_x {
            y -= 1;
            reversed.push(Edit::Add(usize::try_from(y).ok()?));
        } else {
            x -= 1;
            reversed.push(Edit::Delete(usize::try_from(x).ok()?));
        }
    }
    reversed.reverse();
    Some(reversed)
}

fn make_hunks(edits: &[Edit], old: &[&str], new: &[&str]) -> Vec<DiffHunk> {
    let changed: Vec<usize> = edits
        .iter()
        .enumerate()
        .filter_map(|(index, edit)| (!matches!(edit, Edit::Equal(..))).then_some(index))
        .collect();
    if changed.is_empty() {
        return Vec::new();
    }

    let mut ranges = Vec::new();
    let mut start = changed[0].saturating_sub(CONTEXT_LINES);
    let mut end = (changed[0] + CONTEXT_LINES + 1).min(edits.len());
    for &change in &changed[1..] {
        let next_start = change.saturating_sub(CONTEXT_LINES);
        let next_end = (change + CONTEXT_LINES + 1).min(edits.len());
        if next_start <= end {
            end = end.max(next_end);
        } else {
            ranges.push((start, end));
            start = next_start;
            end = next_end;
        }
    }
    ranges.push((start, end));

    ranges
        .into_iter()
        .map(|(start, end)| make_hunk(&edits[start..end], old, new))
        .collect()
}

fn make_hunk(edits: &[Edit], old: &[&str], new: &[&str]) -> DiffHunk {
    let old_start = edits
        .iter()
        .find_map(|edit| match edit {
            Edit::Equal(index, _) | Edit::Delete(index) => Some(index + 1),
            Edit::Add(_) => None,
        })
        .unwrap_or_else(|| insertion_start(edits));
    let new_start = edits
        .iter()
        .find_map(|edit| match edit {
            Edit::Equal(_, index) | Edit::Add(index) => Some(index + 1),
            Edit::Delete(_) => None,
        })
        .unwrap_or_else(|| deletion_start(edits));
    let old_len = edits
        .iter()
        .filter(|edit| !matches!(edit, Edit::Add(_)))
        .count();
    let new_len = edits
        .iter()
        .filter(|edit| !matches!(edit, Edit::Delete(_)))
        .count();
    let lines = edits
        .iter()
        .map(|edit| match *edit {
            Edit::Equal(old_index, new_index) => DiffLine {
                old_line: Some(old_index + 1),
                new_line: Some(new_index + 1),
                kind: DiffLineKind::Context,
                content: old[old_index].to_owned(),
            },
            Edit::Delete(old_index) => DiffLine {
                old_line: Some(old_index + 1),
                new_line: None,
                kind: DiffLineKind::Delete,
                content: old[old_index].to_owned(),
            },
            Edit::Add(new_index) => DiffLine {
                old_line: None,
                new_line: Some(new_index + 1),
                kind: DiffLineKind::Add,
                content: new[new_index].to_owned(),
            },
        })
        .collect();

    DiffHunk {
        old_start,
        old_len,
        new_start,
        new_len,
        lines,
    }
}

fn insertion_start(edits: &[Edit]) -> usize {
    edits
        .iter()
        .find_map(|edit| match edit {
            Edit::Add(index) => Some(*index),
            _ => None,
        })
        .unwrap_or(0)
}

fn deletion_start(edits: &[Edit]) -> usize {
    edits
        .iter()
        .find_map(|edit| match edit {
            Edit::Delete(index) => Some(*index),
            _ => None,
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insertion_does_not_mark_the_following_lines_as_replaced() {
        let diff = build_text_diff("alpha\nbeta\ngamma", "alpha\nnew\nbeta\ngamma");
        assert!(!diff.used_fallback);
        assert_eq!(diff.hunks.len(), 1);
        assert_eq!(
            diff.hunks[0]
                .lines
                .iter()
                .map(|line| line.kind)
                .collect::<Vec<_>>(),
            vec![
                DiffLineKind::Context,
                DiffLineKind::Add,
                DiffLineKind::Context,
                DiffLineKind::Context,
            ]
        );
    }

    #[test]
    fn distant_changes_form_separate_context_hunks() {
        let original = (1..=20).map(|n| format!("line {n}")).collect::<Vec<_>>();
        let mut proposed = original.clone();
        proposed[1] = "changed early".to_owned();
        proposed[18] = "changed late".to_owned();
        let diff = build_text_diff(&original.join("\n"), &proposed.join("\n"));
        assert_eq!(diff.hunks.len(), 2);
    }

    #[test]
    fn identical_inputs_have_no_hunks() {
        assert!(build_text_diff("same\ntext", "same\ntext").hunks.is_empty());
    }

    #[test]
    fn trailing_line_terminator_is_not_silently_ignored() {
        let diff = build_text_diff("same", "same\n");
        assert_eq!(diff.hunks.len(), 1);
        assert!(
            diff.hunks[0]
                .lines
                .iter()
                .any(|line| line.kind == DiffLineKind::Add)
        );
    }
}

/// Which language an editor should colour this file as, from its name.
///
/// Case-insensitive, which the chain of `ends_with` comparisons this replaces was not: a `README.MD`
/// written on a system that does not care about case, or a `SETUP.PY` from an archive, opened as
/// plain text. The extension is a fact about the name and not about who typed it.
///
/// `"text"` for anything unrecognised, because guessing a language for a file this build has never
/// heard of would colour it wrongly with confidence.
#[must_use]
pub fn language_for(file_name: &str) -> &'static str {
    let extension = file_name
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .unwrap_or_default();

    match extension.as_str() {
        "rs" => "rust",
        "py" => "python",
        "sh" | "bash" => "shell",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "json" => "json",
        "md" | "markdown" => "markdown",
        _ => "text",
    }
}

#[cfg(test)]
mod language_tests {
    use super::language_for;

    #[test]
    fn a_name_is_read_whatever_case_it_was_written_in() {
        assert_eq!(language_for("main.rs"), "rust");
        assert_eq!(language_for("MAIN.RS"), "rust");
        assert_eq!(language_for("README.Md"), "markdown");
        assert_eq!(language_for("setup.PY"), "python");
    }

    #[test]
    fn the_spellings_that_mean_the_same_thing_do() {
        assert_eq!(language_for("a.yml"), language_for("a.yaml"));
        assert_eq!(language_for("a.md"), language_for("a.markdown"));
        assert_eq!(language_for("a.sh"), language_for("a.bash"));
    }

    #[test]
    fn a_file_this_build_has_never_heard_of_is_text() {
        // Rather than a guess coloured with confidence.
        assert_eq!(language_for("notes"), "text");
        assert_eq!(language_for("archive.tar.zst"), "text");
        assert_eq!(language_for(""), "text");
        assert_eq!(language_for(".hidden"), "text");
    }

    #[test]
    fn only_the_last_dot_decides() {
        // `notes.rs.bak` is a backup, not Rust.
        assert_eq!(language_for("notes.rs.bak"), "text");
        assert_eq!(language_for("archive.tar.py"), "python");
    }
}
