//! Sort + filter pipeline for the playground.
//!
//! Entirely reactive — no DOM access, no wasm-bindgen.  Suitable for native
//! unit tests (`cargo test -p playground --lib`).

use leptos::prelude::*;
use leptos_arrow_grid::{FilterKind, GridPage, SortDirection, SortState};

use crate::mock_data;

/// Maximum rows processed for in-browser sort/filter.
/// Keeps the WASM thread responsive at the cost of truncating very large datasets.
pub const MAX_SORTABLE: usize = 1_000_000;

/// How many rows are sent to the grid per viewport window.
pub const PAGE_SIZE: usize = 100;

/// Reactive input signals consumed by the pipeline.
pub struct PipelineInputs {
    pub dataset_size: ReadSignal<u64>,
    pub page_start: ReadSignal<u64>,
    pub sort: ReadSignal<SortState>,
    pub filters: ReadSignal<Vec<Option<FilterKind>>>,
}

/// Reactive outputs produced by the pipeline.
pub struct PipelineOutputs {
    /// Total visible row count (filtered or full dataset size).
    pub total_rows: Signal<u64>,
    /// Current page for the grid.
    pub page: Signal<Option<GridPage>>,
    /// Schema (constant for mock data).
    pub schema: Signal<Option<arrow_schema::SchemaRef>>,
}

/// Build the sort+filter pipeline from `inputs`.
///
/// Returns derived signals that can be passed directly to `DataGrid`.
/// All computation is lazy and memoised — only recomputes when inputs change.
pub fn build_pipeline(inputs: PipelineInputs) -> PipelineOutputs {
    let PipelineInputs {
        dataset_size,
        page_start,
        sort,
        filters,
    } = inputs;

    // Pre-compute sorted+filtered indices.
    // None  → lazy offset mode (no active sort or filter).
    // Some  → sorted/filtered index slice, capped at MAX_SORTABLE.
    let sorted_filtered: Memo<Option<Vec<usize>>> = Memo::new(move |_| {
        let sort_s = sort.get();
        let filter_s = filters.get();
        let has_sort = sort_s.active.is_some();
        let has_filter = filter_s.iter().any(|f| f.is_some());

        if !has_sort && !has_filter {
            return None;
        }

        let cap = (dataset_size.get() as usize).min(MAX_SORTABLE);
        let mut indices: Vec<usize> = (0..cap).collect();

        for (col_idx, maybe_fk) in filter_s.iter().enumerate() {
            if let Some(fk) = maybe_fk {
                indices.retain(|&i| mock_data::row_matches_filter(i, col_idx, fk));
            }
        }

        if let Some((col, dir)) = sort_s.active {
            indices.sort_by(|&a, &b| mock_data::compare_rows(a, b, col));
            if dir == SortDirection::Desc {
                indices.reverse();
            }
        }

        Some(indices)
    });

    let total_rows = Signal::derive(move || match sorted_filtered.get() {
        None => dataset_size.get(),
        Some(vis) => vis.len() as u64,
    });

    let page = Signal::derive(move || {
        let start = page_start.get() as usize;
        match sorted_filtered.get() {
            None => {
                let total = dataset_size.get() as usize;
                if start >= total {
                    return None;
                }
                let count = PAGE_SIZE.min(total - start);
                let batch = mock_data::generate_mock_batch_range(start, count);
                Some(GridPage {
                    start: start as u64,
                    row_count: count,
                    batch,
                })
            }
            Some(vis) => {
                if vis.is_empty() || start >= vis.len() {
                    return None;
                }
                let count = PAGE_SIZE.min(vis.len() - start);
                let batch = mock_data::generate_mock_batch_from_indices(&vis[start..start + count]);
                Some(GridPage {
                    start: start as u64,
                    row_count: count,
                    batch,
                })
            }
        }
    });

    let schema = Signal::derive(move || Some(mock_data::mock_schema()));

    PipelineOutputs {
        total_rows,
        page,
        schema,
    }
}
