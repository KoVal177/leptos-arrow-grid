//! Playground entry point — visual testbed for leptos-arrow-grid.

use leptos::prelude::*;
use leptos_arrow_grid::{
    ArrowGridStyles, ArrowGridTheme, ArrowGridThemeScope, ColumnWidths, DEFAULT_COL_WIDTH_PX,
    DataGrid, FilterKind, MenuItem, SelectionState, SortDirection, SortState,
};
use wasm_bindgen::prelude::*;

mod data_pipeline;
mod mock_data;

// ── Component ────────────────────────────────────────────────────────────────

#[component]
fn PlaygroundApp() -> impl IntoView {
    // ── Reactive state ───────────────────────────────────────────────────────
    let dataset_size = RwSignal::new(1_000u64);
    let page_start = RwSignal::new(0u64);
    let sort = RwSignal::new(SortState::default());
    let filters: RwSignal<Vec<Option<FilterKind>>> =
        RwSignal::new(vec![None; data_pipeline::NUM_COLS]);
    let selection: RwSignal<SelectionState> = RwSignal::new(SelectionState::default());
    let dark_mode = RwSignal::new(false);

    // ── Props exposed by DataGrid that the playground exercises ──────────────
    let row_height_large = RwSignal::new(false);
    let show_row_nums = RwSignal::new(true);
    // Host-owned column widths — lets the "Reset widths" button work.
    let col_widths: RwSignal<ColumnWidths> = RwSignal::new(ColumnWidths::new(
        data_pipeline::NUM_COLS,
        DEFAULT_COL_WIDTH_PX,
    ));

    let theme = Signal::derive(move || {
        if dark_mode.get() {
            ArrowGridTheme::Dark
        } else {
            ArrowGridTheme::Light
        }
    });

    // ── Data pipeline ────────────────────────────────────────────────────────
    let pipeline = data_pipeline::build_pipeline(data_pipeline::PipelineInputs {
        dataset_size: dataset_size.read_only(),
        page_start: page_start.read_only(),
        sort: sort.read_only(),
        filters: filters.read_only(),
    });
    let total_rows = pipeline.total_rows;
    let page = pipeline.page;
    let schema = pipeline.schema;
    let filters_signal: Signal<Vec<Option<FilterKind>>> = filters.into();

    // ── extra_menu_items demo ────────────────────────────────────────────────
    // Each column gets a "★ Pin column" item that writes a message to #status.
    let extra_menu_items = Callback::new(move |col: usize| -> Vec<MenuItem> {
        vec![MenuItem {
            label: format!("★ Pin col {col}"),
            disabled: false,
            on_click: Callback::new(move |()| {
                #[cfg(target_arch = "wasm32")]
                {
                    web_sys::window()
                        .and_then(|w| w.document())
                        .and_then(|d| d.get_element_by_id("status"))
                        .inspect(|el| {
                            el.set_text_content(Some(&format!("Pinned column {col}")));
                        });
                }
                #[cfg(not(target_arch = "wasm32"))]
                let _ = col;
            }),
        }]
    });

    // ── View ─────────────────────────────────────────────────────────────────
    view! {
        <ArrowGridStyles />
        <ArrowGridThemeScope theme=theme>
        <div class="pg-shell">
        <div class="toolbar">
            <h1>"leptos-arrow-grid playground"</h1>
            // Dataset size
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
            <span class="toolbar-sep">"|"</span>
            // row_height prop toggle
            <button
                class:active=move || row_height_large.get()
                on:click=move |_| row_height_large.update(|v| *v = !*v)
            >
                {move || if row_height_large.get() { "Row 36px" } else { "Row 24px" }}
            </button>
            // show_row_numbers prop toggle
            <button
                class:active=move || show_row_nums.get()
                on:click=move |_| show_row_nums.update(|v| *v = !*v)
            >
                {move || if show_row_nums.get() { "# Rows ON" } else { "# Rows OFF" }}
            </button>
            // col_widths reset (demonstrates host-owned ColumnWidths)
            <button
                on:click=move |_| col_widths.update(|cw| {
                    *cw = ColumnWidths::new(data_pipeline::NUM_COLS, DEFAULT_COL_WIDTH_PX);
                })
            >
                "Reset widths"
            </button>
            <span class="toolbar-sep">"|"</span>
            <span class="status-text">
                {move || {
                    let t = total_rows.get();
                    if page.get().is_some() && total_rows.get() < dataset_size.get() {
                        format!("{t} visible (filtered/sorted)")
                    } else {
                        format!("{t} rows")
                    }
                }}
            </span>
            <span id="status"></span>
            // CSV download
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
            // Theme toggle
            <button on:click=move |_| dark_mode.update(|d| *d = !*d)>
                {move || if dark_mode.get() { "\u{2600} Light" } else { "\u{263E} Dark" }}
            </button>
        </div>
        <div class="grid-host">
            // Wrap DataGrid in a reactive closure keyed on static props (row_height,
            // show_row_numbers).  The Memo isolates those deps so that changes to
            // sort/filter do NOT cause an unnecessary remount; only the two toolbar
            // toggles do.
            {move || {
                let row_h  = if row_height_large.get() { 36.0_f64 } else { 24.0_f64 };
                let show_rn = show_row_nums.get();
                view! {
                    <DataGrid
                        schema=schema
                        total_rows=total_rows
                        page=page
                        sort=sort.into()
                        row_height=row_h
                        show_row_numbers=show_rn
                        filters=filters_signal
                        selection=selection
                        col_widths=col_widths
                        extra_menu_items=extra_menu_items
                        on_viewport_change=Callback::new(move |start: u64| {
                            page_start.set(start);
                        })
                        on_sort_change=Callback::new(move |(col, _name, new_dir): (usize, String, Option<SortDirection>)| {
                            sort.update(|s| { s.active = new_dir.map(|d| (col, d)); });
                            page_start.set(0);
                        })
                        on_filter_change=Callback::new(move |(col, _name, fk): (usize, String, Option<FilterKind>)| {
                            filters.update(|f| { if col < f.len() { f[col] = fk; } });
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
                }
            }}
        </div>
        <div class="selection-status">
            <span>
                {move || {
                    let count = selection.with(SelectionState::count);
                    if count > 0 { format!("{count} rows selected") }
                    else         { "No selection".to_string() }
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
        </div>
        </ArrowGridThemeScope>
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(PlaygroundApp);
}
