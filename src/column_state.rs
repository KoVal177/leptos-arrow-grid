//! Per-column width signals for the data grid.

/// Column width configuration.
#[derive(Debug, Clone)]
pub struct ColumnWidths {
    widths: Vec<f64>,
    default_width: f64,
}

impl ColumnWidths {
    /// Create column widths for `num_columns`, all set to `default_px`.
    pub fn new(num_columns: usize, default_px: f64) -> Self {
        Self {
            widths: vec![default_px; num_columns],
            default_width: default_px,
        }
    }

    /// Number of columns tracked.
    pub fn len(&self) -> usize {
        self.widths.len()
    }

    /// Whether there are no columns.
    pub fn is_empty(&self) -> bool {
        self.widths.is_empty()
    }

    /// Get the width for a column.
    pub fn width(&self, col: usize) -> f64 {
        self.widths.get(col).copied().unwrap_or(self.default_width)
    }

    /// Set the width for a column.
    pub fn set_width(&mut self, col: usize, px: f64) {
        if col < self.widths.len() {
            self.widths[col] = px;
        }
    }

    /// Total width of all columns.
    pub fn total_width(&self) -> f64 {
        self.widths.iter().sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < f64::EPSILON
    }

    #[test]
    fn new_widths() {
        let cw = ColumnWidths::new(3, 100.0);
        assert_eq!(cw.len(), 3);
        assert!(!cw.is_empty());
        assert!(approx_eq(cw.width(0), 100.0));
        assert!(approx_eq(cw.width(1), 100.0));
        assert!(approx_eq(cw.width(2), 100.0));
    }

    #[test]
    fn set_width() {
        let mut cw = ColumnWidths::new(3, 100.0);
        cw.set_width(1, 200.0);
        assert!(approx_eq(cw.width(1), 200.0));
        assert!(approx_eq(cw.total_width(), 400.0));
    }

    #[test]
    fn out_of_bounds_returns_default() {
        let cw = ColumnWidths::new(2, 80.0);
        assert!(approx_eq(cw.width(99), 80.0));
    }

    #[test]
    fn set_width_out_of_bounds_is_noop() {
        let mut cw = ColumnWidths::new(2, 100.0);
        cw.set_width(99, 500.0);
        assert!(approx_eq(cw.total_width(), 200.0));
    }

    #[test]
    fn empty() {
        let cw = ColumnWidths::new(0, 100.0);
        assert!(cw.is_empty());
        assert!(approx_eq(cw.total_width(), 0.0));
    }
}
