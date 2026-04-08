//! leptos-arrow-grid — virtual scrolling data grid for Arrow `RecordBatch`es.

pub mod cell;
pub mod clipboard;
pub mod col_menu;
pub mod column_state;
pub mod context_menu;
pub mod download;
pub mod grid;
pub mod header;
pub mod keyboard;
pub mod selection;
pub mod theme;
pub mod types;
pub mod viewport;

pub use cell::render_cell;
pub use column_state::ColumnWidths;
pub use context_menu::{ContextAction, MenuPosition};
pub use grid::DataGrid;
pub use keyboard::KeyAction;
pub use selection::SelectionState;
pub use theme::{ArrowGridStyles, ArrowGridTheme, ArrowGridThemeScope};
pub use types::{
    DEFAULT_COL_WIDTH_PX, FilterKind, FilterMode, GridPage, MIN_COL_WIDTH_PX, MenuItem,
    ROW_NUM_WIDTH_PX, SortDirection, SortState, build_filter, cycle_sort, format_row_number,
};
pub use viewport::{
    HorizontalViewport, ViewportRange, ViewportState, compute_viewport, total_height_px,
};
