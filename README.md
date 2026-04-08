# leptos-arrow-grid

[![Crates.io](https://img.shields.io/crates/v/leptos-arrow-grid)](https://crates.io/crates/leptos-arrow-grid)
[![Docs.rs](https://docs.rs/leptos-arrow-grid/badge.svg)](https://docs.rs/leptos-arrow-grid)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](LICENSE-MIT)

High-performance, virtualised data grid for [Leptos](https://leptos.dev/), powered by [Apache Arrow](https://arrow.apache.org/).

## Features

- **Zero-copy rendering** from `Arc<RecordBatch>` — no serialisation overhead
- **Virtualised rows** — renders only visible DOM nodes, handles millions of rows
- **Excel-like selection** — single click, Ctrl+click, Shift+click, drag
- **3-state column sort** — Asc → Desc → Natural (per-column indicators)
- **Column resize** — drag header borders, min-width enforced
- **Per-column filtering** — Contains, StartsWith, Regex modes via kebab menu
- **Context menu** — right-click for Copy / Select All / Download CSV
- **Clipboard TSV copy** — Ctrl+C copies selected rows as tab-separated values
- **CSV download** — Ctrl+S or context menu downloads selection as a timestamped `.csv`
- **Keyboard navigation** — ↑↓ navigate, Shift+↑↓ extend, Ctrl+A select all, Ctrl+C copy, Ctrl+S download, Esc clear
- **Dual theme system** — built-in light (default) and dark themes, scoped switching, CSS-variable customization
- **WASM-safe** — no panics in production code, graceful fallbacks

---

## The Data Contract

> **The grid does NOT fetch or cache data.** It owns zero rows.

`leptos-arrow-grid` is a pure display component. You own all the data logic:

1. You keep `page_start: RwSignal<u64>` and update it from `on_viewport_change`.
2. You keep `sort: RwSignal<SortState>` and update it from `on_sort_change`.
3. You derive a `Signal<Option<GridPage>>` containing an `Arc<RecordBatch>` for the current window.
4. You pass `total_rows` so the grid can size its scrollbar thumb correctly.

The grid tells you what the user wants; you decide how to satisfy it (in-memory sort, SQL query, async fetch, etc.).

---

## Quick Start

### Prerequisites

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
```

### Cargo.toml

```toml
[dependencies]
leptos-arrow-grid = "0.1"
```

### Minimal working example

```rust
use std::sync::Arc;

use arrow_array::{Int64Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema};
use leptos::prelude::*;
use leptos_arrow_grid::{ArrowGridStyles, DataGrid, GridPage, SortState};

#[component]
pub fn MyGrid() -> impl IntoView {
    // 1. Build a static Arrow schema.
    let schema = Arc::new(Schema::new(vec![
        Field::new("id",   DataType::Int64, false),
        Field::new("name", DataType::Utf8,  true),
    ]));

    // 2. Build a RecordBatch with 3 rows.
    let batch = Arc::new(
        RecordBatch::try_new(
            Arc::clone(&schema),
            vec![
                Arc::new(Int64Array::from(vec![1, 2, 3])),
                Arc::new(StringArray::from(vec!["Alice", "Bob", "Carol"])),
            ],
        )
        .expect("valid batch"),
    );

    // 3. Wrap everything in reactive signals.
    let page_start = RwSignal::new(0u64);
    let sort       = RwSignal::new(SortState::default());

    let schema_sig = {
        let s = Arc::clone(&schema);
        Signal::derive(move || Some(Arc::clone(&s)))
    };
    let page_sig = {
        let b = Arc::clone(&batch);
        Signal::derive(move || {
            Some(GridPage { start: page_start.get(), row_count: 3, batch: Arc::clone(&b) })
        })
    };

    view! {
        <ArrowGridStyles />
        // The grid MUST live inside a container with a fixed pixel height.
        <div style="height: 400px;">
            <DataGrid
                schema=schema_sig
                total_rows=Signal::derive(|| 3)
                page=page_sig
                sort=sort.into()
                on_viewport_change=Callback::new(move |start: u64| page_start.set(start))
                on_sort_change=Callback::new(move |_| {})
            />
        </div>
    }
}
```

See `examples/playground/` for a full 1 M-row demo with in-memory sort, filtering, and wide columns.

---

## DataGrid Props Reference

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `schema` | `Signal<Option<SchemaRef>>` | required | Arrow schema — column names and types |
| `total_rows` | `Signal<u64>` | required | Total rows in the dataset (drives scrollbar height) |
| `page` | `Signal<Option<GridPage>>` | required | Current data page (see `GridPage` below) |
| `sort` | `Signal<SortState>` | required | Current sort state |
| `on_viewport_change` | `Callback<u64>` | required | Fires when the scroll position changes; argument is the new first visible row |
| `on_sort_change` | `Callback<(usize, String, Option<SortDirection>)>` | required | Fires on column header click; args are `(col_idx, col_name, new_direction)` |
| `row_height` | `f64` | `24.0` | Pixel height of every row (fixed; grid remounts if you change this) |
| `show_row_numbers` | `bool` | `true` | Show the row-number gutter on the left |
| `filters` | `Signal<Vec<Option<FilterKind>>>` | `vec![]` | Active per-column filters; index matches column position |
| `on_filter_change` | `Callback<(usize, String, Option<FilterKind>)>` | `noop` | Fires when the user edits a column filter; args are `(col_idx, col_name, new_filter)` |
| `extra_menu_items` | `Callback<usize, Vec<MenuItem>>` | none | Headless slot — return extra items to inject into each column's ⋮ kebab menu; arg is `col_idx` |
| `selection` | `RwSignal<SelectionState>` | internal | Host-owned selection state; share the same signal to read the selection outside the grid |
| `col_widths` | `RwSignal<ColumnWidths>` | internal | Host-owned column widths; share to implement "Reset widths" or programmatic resize |
| `on_copy_error` | `Callback<String>` | none | Fires if Ctrl+C clipboard copy fails (e.g. non-HTTPS context) |

---

## Visual Playground

`examples/playground/` is a self-contained Leptos app that exercises every feature of the grid.
It generates a **20-column** synthetic dataset covering six Arrow data types — no server, no
database, no network requests.

### Dataset schema

| # | Column | Arrow type | Nullable |
|---|--------|-----------|---------|
| 0 | `id` | `Int64` | no |
| 1 | `username` | `Utf8` | no |
| 2 | `email` | `Utf8` | ~4 % null |
| 3 | `department` | `Utf8` | ~6 % null |
| 4 | `salary` | `Int64` | ~9 % null |
| 5 | `is_active` | `Boolean` | no |
| 6 | `score` | `Float64` | ~8 % null |
| 7 | `level` | `Int32` | no |
| 8 | `region` | `Utf8` | no |
| 9 | `team` | `Utf8` | ~14 % null |
| 10 | `start_year` | `Int32` | ~3 % null |
| 11 | `manager_id` | `Int64` | ~20 % null |
| 12 | `reports` | `Int32` | no |
| 13 | `badge_id` | `UInt32` | no |
| 14 | `phone` | `Utf8` | ~33 % null |
| 15 | `avg_rating` | `Float64` | ~5 % null |
| 16 | `login_count` | `Int32` | no |
| 17 | `country` | `Utf8` | no |
| 18 | `account_type` | `Utf8` | ~13 % null |
| 19 | `is_verified` | `Boolean` | no |

The wide schema forces horizontal scrolling and column virtualisation, demonstrating that
rendering cost scales with **visible columns**, not total columns.

### What the playground demos

| Feature | How to try it |
|---|---|
| Dataset scale | **1 K / 100 K / 1 M rows** toolbar buttons — scrollbar thumb shrinks as total grows |
| Virtual scrolling | Scroll freely; only ~100 DOM rows are ever alive |

| Column virtualisation | Scroll horizontally across 20 columns; off-screen columns are unmounted via binary search |
| Column sort | Click a column header → Asc → Desc → Natural (3rd click removes sort) |
| Per-column filter | Click the **⋮** kebab on any header → type in the filter box; try *Contains*, *StartsWith*, or *Regex* |
| Combined sort + filter | Active at the same time; visible row count updates in the toolbar status |
| Row selection | Click → single row; Ctrl+click → add/remove; Shift+click → extend range; drag → lasso |
| Keyboard navigation | ↑ / ↓ move focus; Shift+↑↓ extend selection; Ctrl+A select all; Esc clear |
| Copy to clipboard | Select rows → **Ctrl+C** — pastes as TSV into any spreadsheet (requires `localhost` or HTTPS) |
| CSV download | **Ctrl+S** or right-click → *Download CSV* — or use the **Save CSV** toolbar button |
| Column resize | Drag the border between two column headers |
| **`row_height` prop** | Toggle **Row 24px / Row 36px** in the toolbar — remounts grid at new row height |
| **`show_row_numbers` prop** | Toggle **# Rows ON / OFF** — enables or hides the row-number gutter |
| **`col_widths` prop** | Drag column edges, then click **Reset widths** to restore defaults via host-owned `ColumnWidths` |
| **`extra_menu_items` prop** | Every column's **⋮** kebab has a custom *★ Pin col N* item — clicking it posts a message to the status bar |
| Context menu | Right-click anywhere in the grid body |
| Selection counter | Bottom status bar shows live *N rows selected* count |

### Prerequisites

```bash
# Rust WASM target (one-time)
rustup target add wasm32-unknown-unknown

# Trunk web bundler (one-time)
cargo install trunk
```

No extra build step is needed — Trunk compiles the playground to WASM automatically.

> **wasm-bindgen pin**: The playground pins `wasm-bindgen = "=0.2.117"` in its `Cargo.toml`.
> This is intentional — it avoids an upstream `externref` bug that causes a blank screen in some
> browser/toolchain combinations. Do not remove the `=` prefix when bumping.

### Run

```bash
cd examples/playground
trunk serve
```

Open **http://localhost:8080** in your browser.

Trunk watches `src/`, the library source, and the stylesheet for changes and rebuilds
automatically. A first cold build takes ~10 s; incremental rebuilds after a source edit are
typically under 1 s.

To produce an optimised release bundle (useful for profiling render performance):

```bash
trunk build --release
# artefacts land in examples/playground/dist/
```

---

## Known Limitations (v0.1)

| Limitation | Notes |
|---|---|
| Fixed row height only | Height is set via the `row_height` prop (default 24 px). Dynamic heights are not supported. |
| Single-column sort | Multi-column sort is planned for a future release. |
| No column reordering or hiding | Drag-to-reorder is not implemented. |
| CSR only | No SSR support; `#[cfg(target_arch = "wasm32")]` guards all DOM code. |
| Clipboard requires HTTPS | `Ctrl+C` copy uses the [browser Clipboard API](https://developer.mozilla.org/en-US/docs/Web/API/Clipboard_API) which only works in a secure context (HTTPS or `localhost`). The `on_copy_error` callback fires otherwise. |
| Regex filter is substring match | The playground demo does not ship a regex engine; Regex mode falls back to substring matching. |

---

## Theming

leptos-arrow-grid ships with **light** (default) and **dark** themes.

```rust
use leptos_arrow_grid::{ArrowGridStyles, ArrowGridTheme, ArrowGridThemeScope, DataGrid};

// Light theme (default — no wrapper needed):
view! {
    <ArrowGridStyles />
    <div style="height: 400px;">
        <DataGrid ... />
    </div>
}

// Dark theme:
view! {
    <ArrowGridStyles />
    <ArrowGridThemeScope theme=ArrowGridTheme::Dark>
        <div style="height: 400px;">
            <DataGrid ... />
        </div>
    </ArrowGridThemeScope>
}
```

See [docs/theming.md](docs/theming.md) for CSS variable reference, scoped themes,
and manual CSS inclusion.

---

## Further Reading

| Document | Topic |
|---|---|
| [docs/arrow-data-integration.md](docs/arrow-data-integration.md) | Building `RecordBatch` from real data; using `GridPage` |
| [docs/state-management.md](docs/state-management.md) | Why `SelectionState` is pure; wrapping in `RwSignal` |
| [docs/theming.md](docs/theming.md) | Light/dark themes, CSS variable reference, scoped themes, manual CSS inclusion |
| [TROUBLESHOOTING.md](TROUBLESHOOTING.md) | Clipboard HTTPS, "?" cells, empty grid |

---

## MSRV

Rust **1.94** (edition 2024). Requires `wasm32-unknown-unknown` target for browser use.

## License

MIT OR Apache-2.0
