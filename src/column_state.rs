//! Per-column width state with cumulative offset tracking for column virtualization.

use crate::types::MIN_COL_WIDTH_PX;

/// Per-column width tracking with precomputed cumulative offsets.
///
/// Cumulative offsets enable O(log N) visible-column-range queries via binary search.
/// Format: `cum_offsets[i]` = left pixel edge of column `i`.
/// Length = `widths.len() + 1` (last entry = total width).
#[derive(Clone, Debug)]
pub struct ColumnWidths {
    widths: Vec<f64>,
    /// `cum_offsets[i]` = sum of `widths[0..i]`. Length = `widths.len() + 1`.
    cum_offsets: Vec<f64>,
    default_width: f64,
}

impl ColumnWidths {
    /// Create with uniform column widths.
    pub fn new(num_columns: usize, default_px: f64) -> Self {
        let widths = vec![default_px; num_columns];
        let cum_offsets = Self::build_cumulative(&widths);
        Self {
            widths,
            cum_offsets,
            default_width: default_px,
        }
    }

    /// Number of columns.
    pub fn len(&self) -> usize {
        self.widths.len()
    }

    /// Whether there are no columns.
    pub fn is_empty(&self) -> bool {
        self.widths.is_empty()
    }

    /// Width of a single column.
    pub fn width(&self, col: usize) -> f64 {
        self.widths.get(col).copied().unwrap_or(self.default_width)
    }

    /// Set width of a single column; rebuilds cumulative offsets.
    pub fn set_width(&mut self, col: usize, px: f64) {
        if col < self.widths.len() {
            self.widths[col] = px.max(MIN_COL_WIDTH_PX);
            self.cum_offsets = Self::build_cumulative(&self.widths);
        }
    }

    /// Total width of all columns.
    pub fn total_width(&self) -> f64 {
        self.cum_offsets.last().copied().unwrap_or(0.0)
    }

    /// Left pixel offset of column `col`.
    pub fn left_offset(&self, col: usize) -> f64 {
        self.cum_offsets.get(col).copied().unwrap_or(0.0)
    }

    /// Compute the visible column range for a horizontal scroll position.
    ///
    /// Returns `(first_col, count)` — the first visible column index and
    /// number of columns to render (including buffer on each side).
    pub fn visible_range(
        &self,
        scroll_left: f64,
        viewport_width: f64,
        buffer: usize,
    ) -> (usize, usize) {
        if self.widths.is_empty() {
            return (0, 0);
        }
        let n = self.widths.len();

        // Binary search for first column whose right edge > scroll_left.
        let first_raw = self
            .cum_offsets
            .partition_point(|&offset| offset <= scroll_left)
            .saturating_sub(1);

        // Binary search for last column whose left edge < scroll_left + viewport_width.
        let right_edge = scroll_left + viewport_width;
        let last_raw = self
            .cum_offsets
            .partition_point(|&offset| offset < right_edge)
            .min(n);

        // Apply buffer.
        let first = first_raw.saturating_sub(buffer);
        let last = (last_raw + buffer).min(n);
        (first, last - first)
    }

    fn build_cumulative(widths: &[f64]) -> Vec<f64> {
        let mut cum = Vec::with_capacity(widths.len() + 1);
        cum.push(0.0);
        let mut acc = 0.0;
        for &w in widths {
            acc += w;
            cum.push(acc);
        }
        cum
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cumulative_offsets_correct() {
        let cw = ColumnWidths::new(4, 100.0);
        assert_eq!(cw.cum_offsets, vec![0.0, 100.0, 200.0, 300.0, 400.0]);
        assert!((cw.total_width() - 400.0).abs() < f64::EPSILON);
        assert!((cw.left_offset(0) - 0.0).abs() < f64::EPSILON);
        assert!((cw.left_offset(2) - 200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn set_width_rebuilds_offsets() {
        let mut cw = ColumnWidths::new(3, 100.0);
        cw.set_width(1, 200.0);
        assert_eq!(cw.cum_offsets, vec![0.0, 100.0, 300.0, 400.0]);
        assert!((cw.total_width() - 400.0).abs() < f64::EPSILON);
    }

    #[test]
    fn visible_range_no_scroll() {
        let cw = ColumnWidths::new(20, 100.0); // 2000px total
        // viewport 500px wide, no scroll, buffer=2
        let (first, count) = cw.visible_range(0.0, 500.0, 2);
        // visible: cols 0-4 (5 cols), buffer: -2 left (clamped to 0), +2 right = 0..7
        assert_eq!(first, 0);
        assert_eq!(count, 7);
    }

    #[test]
    fn visible_range_scrolled() {
        let cw = ColumnWidths::new(20, 100.0);
        // scroll_left=500 (col 5 starts), viewport=300px (cols 5,6,7 visible), buffer=2
        let (first, count) = cw.visible_range(500.0, 300.0, 2);
        // raw first=5, raw last=8, buffered: 3..10
        assert_eq!(first, 3);
        assert_eq!(count, 7);
    }

    #[test]
    fn visible_range_end_clamped() {
        let cw = ColumnWidths::new(10, 100.0); // 1000px total
        // scroll near end: scroll_left=800, viewport=400 => cols 8,9 visible, buffer=2
        let (first, count) = cw.visible_range(800.0, 400.0, 2);
        assert_eq!(first, 6); // 8-2
        assert!(first + count <= 10);
    }

    #[test]
    fn visible_range_empty() {
        let cw = ColumnWidths::new(0, 100.0);
        assert_eq!(cw.visible_range(0.0, 500.0, 2), (0, 0));
    }

    #[test]
    fn out_of_bounds_returns_default() {
        let cw = ColumnWidths::new(2, 80.0);
        assert!((cw.width(99) - 80.0).abs() < f64::EPSILON);
    }

    #[test]
    fn set_width_out_of_bounds_is_noop() {
        let mut cw = ColumnWidths::new(2, 100.0);
        cw.set_width(99, 500.0);
        assert!((cw.total_width() - 200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn empty() {
        let cw = ColumnWidths::new(0, 100.0);
        assert!(cw.is_empty());
        assert!((cw.total_width() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn set_width_enforces_minimum() {
        let mut cw = ColumnWidths::new(3, 100.0);
        cw.set_width(1, 10.0); // below MIN_COL_WIDTH_PX
        assert!((cw.width(1) - MIN_COL_WIDTH_PX).abs() < f64::EPSILON);
    }
}
