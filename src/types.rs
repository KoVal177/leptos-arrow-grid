//! Grid data types — the grid's contract with its data source.

use std::sync::Arc;

use arrow_array::RecordBatch;
use leptos::prelude::Callback;

/// Minimum column width in pixels.
pub const MIN_COL_WIDTH_PX: f64 = 40.0;

/// Default column width in pixels.
pub const DEFAULT_COL_WIDTH_PX: f64 = 120.0;

/// Row number gutter width in pixels.
pub const ROW_NUM_WIDTH_PX: f64 = 72.0;

/// The grid's view of the current data page.
#[derive(Clone, Debug)]
pub struct GridPage {
    /// Virtual start row of this page.
    pub start: u64,
    /// Row count in this page.
    pub row_count: usize,
    /// Decoded Arrow data.
    pub batch: Arc<RecordBatch>,
}

/// Direction of a column sort.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortDirection {
    /// Ascending (A → Z, 0 → 9).
    Asc,
    /// Descending (Z → A, 9 → 0).
    Desc,
}

impl SortDirection {
    /// Cycle to the next direction: `Asc → Desc`.
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Asc => Self::Desc,
            Self::Desc => Self::Asc,
        }
    }

    /// Unicode arrow for this direction.
    pub fn arrow(self) -> &'static str {
        match self {
            Self::Asc => "\u{2191}",  // ↑
            Self::Desc => "\u{2193}", // ↓
        }
    }
}

/// Sort state as seen by the data grid.
///
/// `active` is a priority-ordered list: index 0 is the primary sort key.
/// An empty list means natural (unsorted) order.
#[derive(Clone, Debug, Default)]
pub struct SortState {
    /// Active sorts in priority order: `(column_index, direction)`.
    /// Empty = natural order.
    pub active: Vec<(usize, SortDirection)>,
}

/// Compute the next sort list after clicking a column header.
///
/// - `additive = false` (plain click): replaces the entire sort.
///   - Unsorted column → `[Asc on col]`.
///   - Sorted-Asc column → `[Desc on col]`.
///   - Sorted-Desc column → `[]` (natural order).
/// - `additive = true` (Shift+click): adds/cycles/removes the column from the
///   existing priority list without disturbing other sorted columns.
///   - Not in list → append `Asc on col`.
///   - In list as Asc → change to Desc.
///   - In list as Desc → remove from list.
pub fn cycle_sort_multi(
    current: &SortState,
    col_idx: usize,
    additive: bool,
) -> Vec<(usize, SortDirection)> {
    if additive {
        let mut result = current.active.clone();
        if let Some(pos) = result.iter().position(|(i, _)| *i == col_idx) {
            match result[pos].1 {
                SortDirection::Asc => result[pos].1 = SortDirection::Desc,
                SortDirection::Desc => {
                    result.remove(pos);
                }
            }
        } else {
            result.push((col_idx, SortDirection::Asc));
        }
        result
    } else {
        match current.active.iter().find(|(i, _)| *i == col_idx) {
            Some((_, SortDirection::Asc)) => vec![(col_idx, SortDirection::Desc)],
            Some((_, SortDirection::Desc)) => vec![],
            None => vec![(col_idx, SortDirection::Asc)],
        }
    }
}

/// Filter mode for a column.
#[derive(Clone, Debug, PartialEq)]
pub enum FilterKind {
    /// Case-insensitive substring match.
    Contains(String),
    /// Case-insensitive prefix match.
    StartsWith(String),
    /// Regex match (case-insensitive flag prepended automatically).
    Regex(String),
}

/// Which filter mode is being edited.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterMode {
    /// Substring match.
    Contains,
    /// Prefix match.
    StartsWith,
    /// Regular expression.
    Regex,
}

impl FilterMode {
    /// Human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Contains => "Contains",
            Self::StartsWith => "Starts with",
            Self::Regex => "Regex",
        }
    }
}

/// Build a `FilterKind` from a mode and text value.
pub fn build_filter(mode: FilterMode, text: String) -> FilterKind {
    match mode {
        FilterMode::Contains => FilterKind::Contains(text),
        FilterMode::StartsWith => FilterKind::StartsWith(text),
        FilterMode::Regex => FilterKind::Regex(text),
    }
}

/// A menu item injected by the consumer into the column menu.
#[derive(Clone)]
pub struct MenuItem {
    /// Display label.
    pub label: String,
    /// Whether the item is disabled.
    pub disabled: bool,
    /// Callback when clicked.
    pub on_click: Callback<()>,
}

/// Format a row number with thin-space separated thousands: `1 000 000`.
pub fn format_row_number(n: u64) -> String {
    if n == 0 {
        return "0".to_owned();
    }
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push('\u{2009}'); // thin space
        }
        result.push(ch);
    }
    result.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_row_number_basic() {
        assert_eq!(format_row_number(0), "0");
        assert_eq!(format_row_number(1), "1");
        assert_eq!(format_row_number(999), "999");
        assert_eq!(format_row_number(1000), "1\u{2009}000");
        assert_eq!(format_row_number(1_000_000), "1\u{2009}000\u{2009}000");
        assert_eq!(format_row_number(12_345_678), "12\u{2009}345\u{2009}678");
    }

    #[test]
    fn sort_direction_next() {
        assert_eq!(SortDirection::Asc.next(), SortDirection::Desc);
        assert_eq!(SortDirection::Desc.next(), SortDirection::Asc);
    }

    #[test]
    fn sort_direction_arrow() {
        assert_eq!(SortDirection::Asc.arrow(), "\u{2191}");
        assert_eq!(SortDirection::Desc.arrow(), "\u{2193}");
    }

    #[test]
    fn cycle_sort_unsorted_starts_asc() {
        let state = SortState::default();
        assert_eq!(
            cycle_sort_multi(&state, 2, false),
            vec![(2, SortDirection::Asc)]
        );
    }

    #[test]
    fn cycle_sort_asc_goes_desc() {
        let state = SortState {
            active: vec![(3, SortDirection::Asc)],
        };
        assert_eq!(
            cycle_sort_multi(&state, 3, false),
            vec![(3, SortDirection::Desc)]
        );
    }

    #[test]
    fn cycle_sort_desc_clears() {
        let state = SortState {
            active: vec![(3, SortDirection::Desc)],
        };
        assert_eq!(cycle_sort_multi(&state, 3, false), vec![]);
    }

    #[test]
    fn cycle_sort_different_column_starts_asc() {
        let state = SortState {
            active: vec![(1, SortDirection::Desc)],
        };
        assert_eq!(
            cycle_sort_multi(&state, 5, false),
            vec![(5, SortDirection::Asc)]
        );
    }

    #[test]
    fn cycle_sort_multi_additive_adds_new_column() {
        let state = SortState {
            active: vec![(0, SortDirection::Asc)],
        };
        let result = cycle_sort_multi(&state, 2, true);
        assert_eq!(
            result,
            vec![(0, SortDirection::Asc), (2, SortDirection::Asc)]
        );
    }

    #[test]
    fn cycle_sort_multi_additive_cycles_existing_asc_to_desc() {
        let state = SortState {
            active: vec![(0, SortDirection::Asc), (2, SortDirection::Desc)],
        };
        let result = cycle_sort_multi(&state, 0, true);
        assert_eq!(
            result,
            vec![(0, SortDirection::Desc), (2, SortDirection::Desc)]
        );
    }

    #[test]
    fn cycle_sort_multi_additive_removes_desc_column() {
        let state = SortState {
            active: vec![(0, SortDirection::Asc), (2, SortDirection::Desc)],
        };
        let result = cycle_sort_multi(&state, 2, true);
        assert_eq!(result, vec![(0, SortDirection::Asc)]);
    }

    #[test]
    fn cycle_sort_multi_non_additive_replaces_multi() {
        let state = SortState {
            active: vec![(0, SortDirection::Asc), (2, SortDirection::Desc)],
        };
        // Plain click on unsorted col — replaces everything
        assert_eq!(
            cycle_sort_multi(&state, 5, false),
            vec![(5, SortDirection::Asc)]
        );
    }
}
