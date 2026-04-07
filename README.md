# leptos-arrow-grid

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
- **CSS-variable theming** — bring your own design tokens (Catppuccin Mocha defaults)
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

Link the stylesheet in your HTML (Trunk):

```html
<link data-trunk rel="css" href="path/to/leptos-arrow-grid/style/grid.css" />
```

### Minimal working example

```rust
use std::sync::Arc;

use arrow_array::{Int64Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema};
use leptos::prelude::*;
use leptos_arrow_grid::{DataGrid, GridPage, SortState};

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

See `examples/playground/` for a full 1 M-row demo with in-memory sort and filtering.

---

## Visual Playground

`examples/playground/` is a self-contained Leptos app that exercises every feature of the grid.
It generates a synthetic in-memory dataset — no server, no database, no network requests.

### What the playground demos

| Feature | How to try it |
|---|---|
| Dataset scale | **1 K / 100 K / 1 M rows** toolbar buttons — scrollbar thumb shrinks as total grows |
| Virtual scrolling | Scroll freely; only ~100 DOM rows are ever alive |
| Column sort | Click a column header → Asc → Desc → Natural (3rd click removes sort) |
| Per-column filter | Click the **⋮** kebab on any header → type in the filter box; try *Contains*, *StartsWith*, or *Regex* |
| Combined sort + filter | Active at the same time; visible row count updates in the toolbar status |
| Row selection | Click → single row; Ctrl+click → add/remove; Shift+click → extend range; drag → lasso |
| Keyboard navigation | ↑ / ↓ move focus; Shift+↑↓ extend selection; Ctrl+A select all; Esc clear |
| Copy to clipboard | Select rows → **Ctrl+C** — pastes as TSV into any spreadsheet (requires `localhost` or HTTPS) |
| CSV download | **Ctrl+S** or right-click → *Download CSV* — or use the **Save CSV** toolbar button |
| Column resize | Drag the border between two column headers |
| Context menu | Right-click anywhere in the grid body |
| Selection counter | Bottom status bar shows live *N rows selected* count |

### Prerequisites

```bash
# Rust WASM target (one-time)
rustup target add wasm32-unknown-unknown

# Trunk web bundler (one-time)
cargo install trunk
```

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
| Fixed row height only | Height is set via the `row_height` prop (default 28 px). Dynamic heights are not supported. |
| Single-column sort | Multi-column sort is planned for a future release. |
| No column reordering or hiding | Drag-to-reorder is not implemented. |
| CSR only | No SSR support; `#[cfg(target_arch = "wasm32")]` guards all DOM code. |
| Clipboard requires HTTPS | `Ctrl+C` copy uses the [browser Clipboard API](https://developer.mozilla.org/en-US/docs/Web/API/Clipboard_API) which only works in a secure context (HTTPS or `localhost`). The `on_copy_error` callback fires otherwise. |
| Regex filter is substring match | The playground demo does not ship a regex engine; Regex mode falls back to substring matching. |

---

## CSS Custom Properties

| Variable | Default | Description |
|----------|---------|-------------|
| `--lag-font-mono` | `monospace` | Font family |
| `--lag-font-size-base` | `13px` | Base font size |
| `--lag-font-size-small` | `11px` | Small text (headers, row nums) |
| `--lag-bg-primary` | `#1e1e2e` | Primary background |
| `--lag-bg-secondary` | `#181825` | Secondary background (row nums, menus) |
| `--lag-bg-surface` | `#313244` | Surface background (header, hover) |
| `--lag-text-primary` | `#cdd6f4` | Primary text colour |
| `--lag-text-secondary` | `#a6adc8` | Secondary text colour |
| `--lag-text-muted` | `#6c7086` | Muted text (row numbers, kebab) |
| `--lag-border` | `#45475a` | Border colour |
| `--lag-accent` | `#89b4fa` | Accent (selection, sort indicators) |
| `--lag-warning` | `#f9e2af` | Warning (sort building) |
| `--lag-error` | `#f38ba8` | Error colour |
| `--lag-transition-fast` | `100ms ease` | Hover/focus transition |
| `--lag-grid-header-height` | `32px` | Header row height |
| `--lag-grid-cell-padding` | `4px 8px` | Data cell padding |

See [docs/theming.md](docs/theming.md) for a complete light-mode override example.

---

## Further Reading

| Document | Topic |
|---|---|
| [docs/arrow-data-integration.md](docs/arrow-data-integration.md) | Building `RecordBatch` from real data; using `GridPage` |
| [docs/state-management.md](docs/state-management.md) | Why `SelectionState` is pure; wrapping in `RwSignal` |
| [docs/theming.md](docs/theming.md) | CSS variables in context; light-mode override snippet |
| [TROUBLESHOOTING.md](TROUBLESHOOTING.md) | Clipboard HTTPS, "?" cells, empty grid |

---

## MSRV

Rust **1.94** (edition 2024). Requires `wasm32-unknown-unknown` target for browser use.

## License

MIT OR Apache-2.0
