//! Keyboard navigation for the data grid.

use crate::selection::SelectionState;

/// Key action result — what the grid should do after a key press.
pub enum KeyAction {
    /// No grid-level action needed.
    None,
    /// Scroll to make this row visible.
    ScrollTo(u64),
    /// Copy selected data to clipboard.
    Copy,
    /// Download visible/selected rows as a CSV file.
    Download,
}

/// Handle keydown on the grid container.
///
/// Returns a `KeyAction` indicating what the grid should do.
/// Mutates `state` in place for selection changes.
pub fn handle_keydown(
    key: &str,
    ctrl: bool,
    shift: bool,
    state: &mut SelectionState,
    total_rows: u64,
) -> KeyAction {
    match key {
        "Escape" => {
            state.clear();
            KeyAction::None
        }
        "ArrowDown" => {
            let cur = state.cursor.unwrap_or(0);
            let next = (cur + 1).min(total_rows.saturating_sub(1));
            handle_arrow_key(state, next, shift, ctrl, total_rows);
            KeyAction::ScrollTo(next)
        }
        "ArrowUp" => {
            let cur = state.cursor.unwrap_or(0);
            let next = cur.saturating_sub(1);
            handle_arrow_key(state, next, shift, ctrl, total_rows);
            KeyAction::ScrollTo(next)
        }
        "PageDown" => {
            let cur = state.cursor.unwrap_or(0);
            let next = (cur + 20).min(total_rows.saturating_sub(1));
            handle_arrow_key(state, next, shift, ctrl, total_rows);
            KeyAction::ScrollTo(next)
        }
        "PageUp" => {
            let cur = state.cursor.unwrap_or(0);
            let next = cur.saturating_sub(20);
            handle_arrow_key(state, next, shift, ctrl, total_rows);
            KeyAction::ScrollTo(next)
        }
        "Home" if ctrl => {
            handle_arrow_key(state, 0, shift, false, total_rows);
            KeyAction::ScrollTo(0)
        }
        "End" if ctrl => {
            let last = total_rows.saturating_sub(1);
            handle_arrow_key(state, last, shift, false, total_rows);
            KeyAction::ScrollTo(last)
        }
        "a" | "A" if ctrl => {
            state.select_all(total_rows);
            KeyAction::ScrollTo(0)
        }
        "c" | "C" if ctrl => KeyAction::Copy,
        "s" | "S" if ctrl => KeyAction::Download,
        _ => KeyAction::None,
    }
}

fn handle_arrow_key(
    state: &mut SelectionState,
    next: u64,
    shift: bool,
    ctrl: bool,
    total_rows: u64,
) {
    state.cursor = Some(next);
    if shift {
        // Extend selection from anchor
        let from = state.anchor.unwrap_or(next);
        let lo = from.min(next);
        let hi = from.max(next).min(total_rows.saturating_sub(1));
        state.selected.clear();
        for i in lo..=hi {
            state.selected.insert(i);
        }
    } else if !ctrl {
        // Single select (no ctrl = not just moving cursor)
        state.anchor = Some(next);
        state.selected.clear();
        state.selected.insert(next);
    }
    // If ctrl (no shift), just move cursor without changing selection
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrow_down_from_start() {
        let mut state = SelectionState::default();
        let action = handle_keydown("ArrowDown", false, false, &mut state, 100);
        assert_eq!(state.cursor, Some(1));
        assert!(state.is_selected(1));
        assert!(matches!(action, KeyAction::ScrollTo(1)));
    }

    #[test]
    fn arrow_up_from_start() {
        let mut state = SelectionState::default();
        let action = handle_keydown("ArrowUp", false, false, &mut state, 100);
        assert_eq!(state.cursor, Some(0));
        assert!(matches!(action, KeyAction::ScrollTo(0)));
    }

    #[test]
    fn shift_arrow_extends_range() {
        let mut state = SelectionState::default();
        state.on_pointer_down(5, false, false, 100);
        state.on_pointer_up();
        let _ = handle_keydown("ArrowDown", false, true, &mut state, 100);
        let _ = handle_keydown("ArrowDown", false, true, &mut state, 100);
        assert_eq!(state.count(), 3); // 5, 6, 7
    }

    #[test]
    fn ctrl_a_selects_all() {
        let mut state = SelectionState::default();
        let action = handle_keydown("a", true, false, &mut state, 50);
        assert_eq!(state.count(), 50);
        assert!(matches!(action, KeyAction::ScrollTo(0)));
    }

    #[test]
    fn ctrl_c_returns_copy() {
        let mut state = SelectionState::default();
        let action = handle_keydown("c", true, false, &mut state, 100);
        assert!(matches!(action, KeyAction::Copy));
    }

    #[test]
    fn escape_clears() {
        let mut state = SelectionState::default();
        state.select_all(100);
        let _ = handle_keydown("Escape", false, false, &mut state, 100);
        assert_eq!(state.count(), 0);
    }

    #[test]
    fn page_down() {
        let mut state = SelectionState::default();
        let action = handle_keydown("PageDown", false, false, &mut state, 100);
        assert_eq!(state.cursor, Some(20));
        assert!(matches!(action, KeyAction::ScrollTo(20)));
    }

    #[test]
    fn ctrl_end() {
        let mut state = SelectionState::default();
        let action = handle_keydown("End", true, false, &mut state, 100);
        assert_eq!(state.cursor, Some(99));
        assert!(matches!(action, KeyAction::ScrollTo(99)));
    }
}
