# Examples

## `playground` — 1 M-row Arrow Grid

The playground is a self-contained Leptos app that exercises every feature of
`leptos-arrow-grid` against a fully in-memory synthetic dataset. No server,
no database, no network requests.

### Running

```bash
cd examples/playground
trunk serve
# open http://localhost:8080
```

For an optimised release build (useful for profiling render performance):

```bash
trunk build --release
# artefacts land in examples/playground/dist/
```

> **wasm-bindgen pin**: `Cargo.toml` pins `wasm-bindgen = "=0.2.117"` to avoid
> an upstream `externref` bug. Do not drop the `=` prefix when bumping.

### Dataset

The playground generates a **20-column** schema (six Arrow types, up to 1 M rows)
toggled by toolbar buttons. The wide schema forces horizontal scrolling and
column virtualisation, showing that rendering cost scales with **visible columns**,
not total columns.

### Key Decisions in the Source

**Owning all data state** — the host component keeps all signals:

```rust
let page_start  = RwSignal::new(0u64);
let sort        = RwSignal::new(SortState::default());
let filters     = RwSignal::new(vec![None::<FilterKind>; SCHEMA.fields().len()]);
let selection   = RwSignal::new(SelectionState::default());
let col_widths  = RwSignal::new(ColumnWidths::default());
```

The `DataGrid` callback props (`on_viewport_change`, `on_sort_change`,
`on_filter_change`) write back to these signals; the host then derives what
data to serve.

**Multi-column sort** — `on_sort_change` receives `Vec<(usize, String, SortDirection)>`.
Shift+click adds a sort key; plain click replaces:

```rust
on_sort_change=Callback::new(move |cols: Vec<(usize, String, SortDirection)>| {
    sort.set(SortState::from_vec(cols));
})
```

**In-memory sort + filter loop** — the playground derives the visible page
using a pure function so the derivation is automatically re-run on any dependency change:

```rust
let page = Signal::derive(move || {
    let batch = apply_filters(&full_batch, &filters.get());
    let batch = apply_sort(&batch, &sort.get());
    let start = page_start.get() as usize;
    let end   = (start + PAGE_SIZE).min(batch.num_rows());
    Some(GridPage { start: start as u64, row_count: batch.num_rows() as u64,
                    batch: batch.slice(start, end - start).into() })
});
```

**Host-owned selection and column widths** — sharing the signals enables the
toolbar to read live selection count and the "Reset widths" button:

```rust
<DataGrid
    selection=selection
    col_widths=col_widths
    ...
/>
// elsewhere:
let selected_count = Signal::derive(move || selection.get().len());
```

### Key Concepts

- **Data Contract** — the grid owns zero rows; you own `page_start`, `sort`, and the
  `GridPage` signal.
- `on_sort_change` — receives `Vec<(usize, String, SortDirection)>` where an empty vec
  means natural order.
- `SortState::from_vec` — constructs the multi-column sort state from the callback arg.
- `GridPage` — wraps an `Arc<RecordBatch>` plus the `start` row offset and `row_count`.
- Proportional scroll — `total_rows` drives the scrollbar height; the grid maps
  pixel scroll position to row index proportionally.

### What to Try

- Click **1 K / 100 K / 1 M** to see scrollbar thumb shrink as total rows grow.
- Shift+click two column headers to activate multi-column sort; priority numbers
  appear on each sorted column.
- Click the **⋮** kebab on a header, type a filter string, and switch between
  *Contains*, *StartsWith*, and *Regex* modes.
- Drag-select a range of rows then press **Ctrl+C** to copy as TSV,
  or **Ctrl+S** / the toolbar button to download a CSV.
- Resize a column, then click **Reset widths** to restore defaults.
