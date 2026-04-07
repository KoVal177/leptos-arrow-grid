//! Playground entry point — visual testbed for leptos-arrow-grid.

use std::cmp::Ordering;

use leptos::prelude::*;
use leptos_arrow_grid::{
    ArrowGridStyles, ArrowGridTheme, ArrowGridThemeScope, DataGrid, FilterKind, GridPage,
    SelectionState, SortDirection, SortState,
};
use wasm_bindgen::prelude::*;

mod mock_data;

/// Rows sent to the grid per page (viewport window).
const PAGE_SIZE: usize = 100;

/// Maximum rows to index for in-browser sort/filter (keeps UI responsive).
const MAX_SORTABLE: usize = 1_000_000;

// ── Per-row value helpers ────────────────────────────────────────────────────

/// String representation of column `col` for dataset row `i`.
fn row_value_str(i: usize, col: usize) -> String {
    match col {
        0 => i.to_string(),
        1 => format!("user_{i:07}"),
        2 => {
            if i % 17 == 0 {
                String::new()
            } else {
                mock_data::DEPTS[i % mock_data::DEPTS.len()].to_string()
            }
        }
        3 => {
            if i % 11 == 0 {
                String::new()
            } else {
                (50_000 + i % 100_000).to_string()
            }
        }
        4 => (i % 3 != 0).to_string(),
        _ => String::new(),
    }
}

fn row_matches_filter(i: usize, col: usize, filter: &FilterKind) -> bool {
    let val = row_value_str(i, col).to_lowercase();
    match filter {
        FilterKind::Contains(s) => val.contains(&s.to_lowercase()),
        FilterKind::StartsWith(s) => val.starts_with(&s.to_lowercase()),
        // Regex rendered as substring match for the demo (no regex dep in WASM).
        FilterKind::Regex(s) => val.contains(&s.to_lowercase()),
    }
}

fn compare_rows(a: usize, b: usize, col: usize) -> Ordering {
    match col {
        // id and username order mirrors the row index.
        0 | 1 => a.cmp(&b),
        2 => row_value_str(a, 2).cmp(&row_value_str(b, 2)),
        3 => {
            // Null rows (i % 11 == 0) sort last.
            let va = (a % 11 != 0).then(|| 50_000 + a % 100_000);
            let vb = (b % 11 != 0).then(|| 50_000 + b % 100_000);
            match (va, vb) {
                (None, None) => a.cmp(&b),
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (Some(x), Some(y)) => x.cmp(&y),
            }
        }
        4 => {
            // true (active) sorts before false (inactive) in Asc.
            i32::from(b % 3 != 0).cmp(&i32::from(a % 3 != 0))
        }
        _ => Ordering::Equal,
    }
}

// ── Component ────────────────────────────────────────────────────────────────

#[component]
fn PlaygroundApp() -> impl IntoView {
    // Total simulated dataset size — scrollbar thumb scales to this.
    let dataset_size = RwSignal::new(1_000u64);
    // First row of the current page (updated by on_viewport_change).
    let page_start = RwSignal::new(0u64);
    // Active sort state (read by DataGrid header for arrow rendering).
    let sort = RwSignal::new(SortState::default());
    // Active filters — one slot per column.
    let filters: RwSignal<Vec<Option<FilterKind>>> = RwSignal::new(vec![None; 5]);
    // Selection state — owned by playground so we can display count.
    let selection: RwSignal<SelectionState> = RwSignal::new(SelectionState::default());

    // Theme toggle.
    let dark_mode = RwSignal::new(false);
    let theme = Signal::derive(move || {
        if dark_mode.get() {
            ArrowGridTheme::Dark
        } else {
            ArrowGridTheme::Light
        }
    });

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
                indices.retain(|&i| row_matches_filter(i, col_idx, fk));
            }
        }

        if let Some((col, dir)) = sort_s.active {
            indices.sort_by(|&a, &b| compare_rows(a, b, col));
            if dir == SortDirection::Desc {
                indices.reverse();
            }
        }

        Some(indices)
    });

    // Total visible rows — either the full dataset size or the filtered count.
    let total_rows = Signal::derive(move || match sorted_filtered.get() {
        None => dataset_size.get(),
        Some(vis) => vis.len() as u64,
    });

    // Schema is constant for this mock dataset.
    let schema = Signal::derive(move || Some(mock_data::mock_schema()));

    // Current page — generated lazily (offset mode) or from the index cache.
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
                Some(GridPage { start: start as u64, row_count: count, batch })
            }
            Some(vis) => {
                if vis.is_empty() || start >= vis.len() {
                    return None;
                }
                let count = PAGE_SIZE.min(vis.len() - start);
                let batch =
                    mock_data::generate_mock_batch_from_indices(&vis[start..start + count]);
                Some(GridPage { start: start as u64, row_count: count, batch })
            }
        }
    });

    let filters_signal: Signal<Vec<Option<FilterKind>>> = filters.into();

    view! {
        <ArrowGridStyles />
        <ArrowGridThemeScope theme=theme>
        <div class="pg-shell">
        <div class="toolbar">
            <h1>"leptos-arrow-grid playground"</h1>
            <button
                class:active=move || dataset_size.get() == 1_000
                on:click=move |_| { dataset_size.set(1_000); page_start.set(0); }
            >
                "1 K rows"
            </button>
            <button
                class:active=move || dataset_size.get() == 100_000
                on:click=move |_| { dataset_size.set(100_000); page_start.set(0); }
            >
                "100 K rows"
            </button>
            <button
                class:active=move || dataset_size.get() == 1_000_000
                on:click=move |_| { dataset_size.set(1_000_000); page_start.set(0); }
            >
                "1 M rows"
            </button>
            <span class="status-text">
                {move || {
                    let t = total_rows.get();
                    if sorted_filtered.get().is_some() {
                        format!("{t} visible (filtered/sorted)")
                    } else {
                        format!("{t} rows")
                    }
                }}
            </span>
            <span id="status"></span>
            <button on:click=move |_| {
                if let Some(s) = schema.get() {
                    let csv = selection.with_untracked(|sel| {
                        leptos_arrow_grid::download::build_csv(&sel.selected, &s, &page.get())
                    });
                    leptos_arrow_grid::download::download_csv_file(&csv);
                }
            }>
                "Save CSV"
            </button>
            <button on:click=move |_| dark_mode.update(|d| *d = !*d)>
                {move || if dark_mode.get() { "\u{2600} Light" } else { "\u{263E} Dark" }}
            </button>
        </div>
        <div class="grid-host">
            <DataGrid
                schema=schema
                total_rows=total_rows
                page=page
                sort=sort.into()
                filters=filters_signal
                selection=selection
                on_viewport_change=Callback::new(move |start: u64| {
                    page_start.set(start);
                })
                on_sort_change=Callback::new(move |(col, _name, new_dir): (usize, String, Option<SortDirection>)| {
                    sort.update(|s| {
                        s.active = new_dir.map(|d| (col, d));
                    });
                    page_start.set(0);
                })
                on_filter_change=Callback::new(move |(col, _name, fk): (usize, String, Option<FilterKind>)| {
                    filters.update(|f| {
                        if col < f.len() {
                            f[col] = fk;
                        }
                    });
                    page_start.set(0);
                })
                on_copy_error=Callback::new(move |err: String| {
                    #[cfg(target_arch = "wasm32")]
                    {
                        web_sys::window()
                            .and_then(|w| w.document())
                            .and_then(|d| d.get_element_by_id("status"))
                            .inspect(|el| el.set_text_content(Some(&format!("Copy failed: {err}"))));
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    let _ = err;
                })
            />
        </div>
        <div class="selection-status">
            <span>
                {move || {
                    let count = selection.with(SelectionState::count);
                    if count > 0 {
                        format!("{count} rows selected")
                    } else {
                        "No selection".to_string()
                    }
                }}
            </span>
        </div>
        <div class="keyboard-hints">
            <kbd>"↑↓"</kbd>" Navigate  "
            <kbd>"Shift+↑↓"</kbd>" Extend selection  "
            <kbd>"Ctrl+A"</kbd>" Select all  "
            <kbd>"Ctrl+C"</kbd>" Copy  "
            <kbd>"Ctrl+S"</kbd>" Download CSV  "
            <kbd>"Esc"</kbd>" Clear"
        </div>
        </div> // pg-shell
        </ArrowGridThemeScope>
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(PlaygroundApp);
}
