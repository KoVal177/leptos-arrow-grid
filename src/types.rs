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
#[derive(Clone, Debug, Default)]
pub struct SortState {
    /// Active sort: `(column_index, direction)`, or `None` for natural order.
    pub active: Option<(usize, SortDirection)>,
    /// Whether the sort index is currently building.
    pub building: bool,
    /// Build progress (0.0 → 1.0).
    pub progress: f32,
}

/// Cycle sort for a column-header click.
///
/// - Clicking an unsorted column → `Asc` on that column.
/// - Clicking the sorted column → cycle: `Asc → Desc → None`.
/// - Clicking a different column → `Asc` on the new column.
pub fn cycle_sort(current: &SortState, col_idx: usize) -> (usize, Option<SortDirection>) {
    match current.active {
        Some((active_idx, dir)) if active_idx == col_idx => match dir {
            SortDirection::Asc => (col_idx, Some(SortDirection::Desc)),
            SortDirection::Desc => (col_idx, None),
        },
        _ => (col_idx, Some(SortDirection::Asc)),
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
        assert_eq!(cycle_sort(&state, 2), (2, Some(SortDirection::Asc)));
    }

    #[test]
    fn cycle_sort_asc_goes_desc() {
        let state = SortState {
            active: Some((3, SortDirection::Asc)),
            building: false,
            progress: 0.0,
        };
        assert_eq!(cycle_sort(&state, 3), (3, Some(SortDirection::Desc)));
    }

    #[test]
    fn cycle_sort_desc_clears() {
        let state = SortState {
            active: Some((3, SortDirection::Desc)),
            building: false,
            progress: 0.0,
        };
        assert_eq!(cycle_sort(&state, 3), (3, None));
    }

    #[test]
    fn cycle_sort_different_column_starts_asc() {
        let state = SortState {
            active: Some((1, SortDirection::Desc)),
            building: false,
            progress: 0.0,
        };
        assert_eq!(cycle_sort(&state, 5), (5, Some(SortDirection::Asc)));
    }
}
