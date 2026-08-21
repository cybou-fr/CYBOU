// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Undo and redo layout history management.

use crate::layout::engine::DesktopLayout;

/// Undo/redo history buffer for Desktop layout changes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutHistory {
    undo_stack: Vec<DesktopLayout>,
    redo_stack: Vec<DesktopLayout>,
    max_history: usize,
}

impl LayoutHistory {
    /// Create a new history manager with default max depth (30).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history: 30,
        }
    }

    /// Push current layout snapshot before making a change.
    pub fn push(&mut self, layout: DesktopLayout) {
        if self.undo_stack.last() == Some(&layout) {
            return;
        }
        self.undo_stack.push(layout);
        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    /// Check if undo is available.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available.
    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Undo last layout change, returning restored layout.
    pub fn undo(&mut self, current: DesktopLayout) -> Option<DesktopLayout> {
        let prev = self.undo_stack.pop()?;
        self.redo_stack.push(current);
        Some(prev)
    }

    /// Redo last reverted layout change, returning re-applied layout.
    pub fn redo(&mut self, current: DesktopLayout) -> Option<DesktopLayout> {
        let next = self.redo_stack.pop()?;
        self.undo_stack.push(current);
        Some(next)
    }
}
