//! Playground entry point — visual testbed for leptos-arrow-grid.

use leptos::prelude::*;
use leptos_arrow_grid::{DataGrid, GridPage, SortDirection, SortState, cycle_sort};

mod mock_data;

#[component]
fn PlaygroundApp() -> impl IntoView {
    let (row_count, set_row_count) = signal(100_000usize);

    let batch = Memo::new(move |_| mock_data::generate_mock_batch(row_count.get()));

    let schema = Signal::derive(move || Some(batch.get().schema()));
    let total_rows = Signal::derive(move || row_count.get() as u64);

    let page = Signal::derive(move || {
        let b = batch.get();
        let rows = b.num_rows();
        Some(GridPage {
            start: 0,
            row_count: rows,
            batch: b,
        })
    });

    let sort = RwSignal::new(SortState::default());

    view! {
        <div class="toolbar">
            <h1>"leptos-arrow-grid playground"</h1>
            <button on:click=move |_| set_row_count.set(1_000)>"1 K rows"</button>
            <button on:click=move |_| set_row_count.set(100_000)>"100 K rows"</button>
            <button on:click=move |_| set_row_count.set(1_000_000)>"1 M rows (stress)"</button>
            <span style="color:#a6e3a1">
                {move || format!("{} rows loaded", row_count.get())}
            </span>
        </div>
        <div class="grid-host">
            <DataGrid
                schema=schema
                total_rows=total_rows
                page=page
                sort=sort.into()
                on_viewport_change=move |_range| { /* no-op: static data */ }
                on_sort_click=move |(col, _dir): (usize, Option<SortDirection>)| {
                    sort.update(|s| {
                        let (_idx, dir) = cycle_sort(s, col);
                        s.active = dir.map(|d| (col, d));
                    });
                }
            />
        </div>
    }
}

pub fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(PlaygroundApp);
}
