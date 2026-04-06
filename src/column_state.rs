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
