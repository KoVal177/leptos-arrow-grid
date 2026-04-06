//! Logical selection model for the virtualised grid.
//!
//! Tracks selected row indices as `HashSet<u64>`. All operations are pure
//! (no signals) — the caller wraps in `RwSignal<SelectionState>`.

use std::collections::HashSet;

/// Logical grid selection state.
#[derive(Clone, Debug, Default)]
pub struct SelectionState {
    /// Set of selected absolute row indices.
    pub selected: HashSet<u64>,
    /// Anchor for Shift+click range selection.
    pub anchor: Option<u64>,
    /// Keyboard cursor position (active row).
    pub cursor: Option<u64>,
    /// Whether the user is currently drag-selecting.
    pub dragging: bool,
}

impl SelectionState {
    /// Clear all selection state.
    pub fn clear(&mut self) {
        self.selected.clear();
        self.anchor = None;
        self.cursor = None;
        self.dragging = false;
    }

    /// Select all rows `0..total_rows`.
    pub fn select_all(&mut self, total_rows: u64) {
        self.selected = (0..total_rows).collect();
        self.anchor = Some(0);
        self.cursor = total_rows.checked_sub(1);
    }

    /// Check if a row is selected.
    pub fn is_selected(&self, row: u64) -> bool {
        self.selected.contains(&row)
    }

    /// Number of selected rows.
    pub fn count(&self) -> usize {
        self.selected.len()
    }

    /// Handle pointer down on a row.
    pub fn on_pointer_down(&mut self, row: u64, ctrl: bool, shift: bool, total_rows: u64) {
        if ctrl {
            // Toggle single row
            if !self.selected.remove(&row) {
                self.selected.insert(row);
            }
            self.cursor = Some(row);
        } else if shift {
            // Range from anchor to row
            let from = self.anchor.or(self.cursor).unwrap_or(row);
            let lo = from.min(row);
            let hi = from.max(row).min(total_rows.saturating_sub(1));
            self.selected.clear();
            for i in lo..=hi {
                self.selected.insert(i);
            }
            self.cursor = Some(row);
        } else {
            // Single click — clear and select
            self.selected.clear();
            self.selected.insert(row);
            self.anchor = Some(row);
            self.cursor = Some(row);
            self.dragging = true;
        }
    }

    /// Handle pointer enter during drag.
    pub fn on_pointer_enter_drag(&mut self, row: u64, total_rows: u64) {
        if !self.dragging {
            return;
        }
        let anchor = self.anchor.or(self.cursor).unwrap_or(row);
        let lo = anchor.min(row);
        let hi = anchor.max(row).min(total_rows.saturating_sub(1));
        self.selected.clear();
        for i in lo..=hi {
            self.selected.insert(i);
        }
        self.cursor = Some(row);
    }

    /// Handle pointer up — stop dragging.
    pub fn on_pointer_up(&mut self) {
        self.dragging = false;
    }

    /// Handle right-click: if the row is not selected, select only it.
    /// If already selected, keep selection.
    pub fn on_context_menu(&mut self, row: u64) {
        if !self.selected.contains(&row) {
            self.selected.clear();
            self.selected.insert(row);
            self.anchor = Some(row);
            self.cursor = Some(row);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_click() {
        let mut s = SelectionState::default();
        s.on_pointer_down(5, false, false, 100);
        assert!(s.is_selected(5));
        assert_eq!(s.count(), 1);
        assert_eq!(s.anchor, Some(5));
    }

    #[test]
    fn ctrl_click_toggle() {
        let mut s = SelectionState::default();
        s.on_pointer_down(5, false, false, 100);
        s.on_pointer_up();
        s.on_pointer_down(10, true, false, 100);
        assert!(s.is_selected(5));
        assert!(s.is_selected(10));
        assert_eq!(s.count(), 2);
        // Toggle off
        s.on_pointer_down(5, true, false, 100);
        assert!(!s.is_selected(5));
        assert_eq!(s.count(), 1);
    }

    #[test]
    fn shift_click_range() {
        let mut s = SelectionState::default();
        s.on_pointer_down(5, false, false, 100);
        s.on_pointer_up();
        s.on_pointer_down(10, false, true, 100);
        assert_eq!(s.count(), 6); // 5,6,7,8,9,10
        for i in 5..=10 {
            assert!(s.is_selected(i));
        }
    }

    #[test]
    fn drag_select() {
        let mut s = SelectionState::default();
        s.on_pointer_down(5, false, false, 100);
        s.on_pointer_enter_drag(8, 100);
        assert_eq!(s.count(), 4); // 5,6,7,8
        s.on_pointer_up();
        assert!(!s.dragging);
    }

    #[test]
    fn select_all() {
        let mut s = SelectionState::default();
        s.select_all(5);
        assert_eq!(s.count(), 5);
        assert_eq!(s.anchor, Some(0));
        assert_eq!(s.cursor, Some(4));
    }

    #[test]
    fn context_menu_unselected_drops() {
        let mut s = SelectionState::default();
        s.on_pointer_down(5, false, false, 100);
        s.on_pointer_up();
        s.on_pointer_down(10, true, false, 100);
        assert_eq!(s.count(), 2);
        // Right-click on unselected row 20
        s.on_context_menu(20);
        assert_eq!(s.count(), 1);
        assert!(s.is_selected(20));
    }

    #[test]
    fn context_menu_selected_keeps() {
        let mut s = SelectionState::default();
        s.on_pointer_down(5, false, false, 100);
        s.on_pointer_up();
        s.on_pointer_down(10, true, false, 100);
        assert_eq!(s.count(), 2);
        // Right-click on already-selected row 5
        s.on_context_menu(5);
        assert_eq!(s.count(), 2);
    }

    #[test]
    fn clear() {
        let mut s = SelectionState::default();
        s.select_all(100);
        s.clear();
        assert_eq!(s.count(), 0);
        assert!(s.anchor.is_none());
        assert!(s.cursor.is_none());
    }
}
