//! Viewport math for the virtualized grid.
//!
//! All functions are pure — no signals, no I/O, no side effects.

/// The current viewport state (used by `DataGrid` signal).
#[derive(Clone, Debug, Default)]
pub struct ViewportState {
    /// First visible virtual row.
    pub start_row: u64,
    /// Number of visible rows.
    pub visible_rows: usize,
}

/// The range of rows that should be visible given the current scroll state.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportRange {
    /// Absolute index of the first visible row (inclusive).
    pub first_row: u64,
    /// Number of rows to render.
    pub row_count: usize,
}

/// Compute the visible row range for a given scroll state.
///
/// # Guarantees (enforced by proptest)
///
/// - `first_row + row_count <= total_rows` always.
/// - `first_row` is always a valid row index when `total_rows > 0`.
/// - `row_count >= 1` when `total_rows > 0`.
pub fn compute_viewport(
    scroll_top_px: f64,
    container_height_px: f64,
    row_height_px: f64,
    total_rows: u64,
) -> ViewportRange {
    if total_rows == 0 || row_height_px <= 0.0 || container_height_px <= 0.0 {
        return ViewportRange {
            first_row: 0,
            row_count: 0,
        };
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let first_row = (scroll_top_px / row_height_px).floor() as u64;
    let first_row = first_row.min(total_rows.saturating_sub(1));

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let visible = (container_height_px / row_height_px).ceil() as usize;
    // Add one extra row to handle partial rows at viewport boundary.
    let visible = visible + 1;

    #[allow(clippy::cast_possible_truncation)]
    let remaining = (total_rows - first_row) as usize;
    let row_count = visible.min(remaining);

    ViewportRange {
        first_row,
        row_count,
    }
}

/// Total scrollable height for a dataset.
pub fn total_height_px(total_rows: u64, row_height_px: f64) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let result = total_rows as f64 * row_height_px;
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_rows_returns_zero_range() {
        let vp = compute_viewport(0.0, 600.0, 28.0, 0);
        assert_eq!(vp.first_row, 0);
        assert_eq!(vp.row_count, 0);
    }

    #[test]
    fn basic_viewport() {
        let vp = compute_viewport(0.0, 280.0, 28.0, 1000);
        assert_eq!(vp.first_row, 0);
        // ceil(280/28) + 1 = 11
        assert_eq!(vp.row_count, 11);
    }

    #[test]
    fn scrolled_halfway() {
        let vp = compute_viewport(2800.0, 280.0, 28.0, 1000);
        // floor(2800/28) = 100
        assert_eq!(vp.first_row, 100);
        assert_eq!(vp.row_count, 11);
    }

    #[test]
    fn scroll_past_end_clamps() {
        let vp = compute_viewport(1_000_000.0, 280.0, 28.0, 100);
        assert_eq!(vp.first_row, 99);
        assert_eq!(vp.row_count, 1);
    }

    #[test]
    fn total_height() {
        assert!((total_height_px(1000, 28.0) - 28000.0).abs() < f64::EPSILON);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn viewport_is_always_in_bounds(
            scroll_top in 0.0f64..1_000_000.0f64,
            container_height in 100.0f64..2000.0f64,
            row_height in 1.0f64..200.0f64,
            total_rows in 0u64..10_000_000u64,
        ) {
            let vp = compute_viewport(scroll_top, container_height, row_height, total_rows);
            prop_assert!(vp.first_row + vp.row_count as u64 <= total_rows);
        }

        #[test]
        fn viewport_row_count_is_nonzero_when_data_exists(
            scroll_top in 0.0f64..1_000_000.0f64,
            container_height in 100.0f64..2000.0f64,
            row_height in 1.0f64..200.0f64,
            total_rows in 1u64..10_000_000u64,
        ) {
            let vp = compute_viewport(scroll_top, container_height, row_height, total_rows);
            prop_assert!(vp.row_count >= 1);
        }

        #[test]
        fn viewport_first_row_never_exceeds_total(
            scroll_top in 0.0f64..1_000_000_000.0f64,
            container_height in 100.0f64..2000.0f64,
            row_height in 1.0f64..200.0f64,
            total_rows in 1u64..10_000_000u64,
        ) {
            let vp = compute_viewport(scroll_top, container_height, row_height, total_rows);
            prop_assert!(vp.first_row < total_rows);
        }
    }
}
