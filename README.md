# leptos-arrow-grid

[![Crates.io version](https://img.shields.io/crates/v/leptos-arrow-grid.svg)](https://crates.io/crates/leptos-arrow-grid)
[![Docs.rs](https://img.shields.io/docsrs/leptos-arrow-grid)](https://docs.rs/leptos-arrow-grid)
[![CI](https://github.com/KoVal177/leptos-arrow-grid/actions/workflows/ci.yml/badge.svg)](https://github.com/KoVal177/leptos-arrow-grid/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)
[![MSRV](https://img.shields.io/badge/rustc-1.94+-blue.svg)](https://blog.rust-lang.org/)
[![Downloads](https://img.shields.io/crates/d/leptos-arrow-grid.svg)](https://crates.io/crates/leptos-arrow-grid)

High-performance, virtualised data grid for [Leptos](https://leptos.dev/), powered by [Apache Arrow](https://arrow.apache.org/).

## Overview

`leptos-arrow-grid` renders `Arc<RecordBatch>` pages directly in the browser — no
serialisation, no intermediate model, just Arrow buffers mapped to DOM rows.
It handles millions of rows through proportional virtual scroll and scales
rendering cost with visible columns rather than total columns.

## Features

- **Zero-copy rendering** — renders from `Arc<RecordBatch>` with no serialisation overhead
- **Virtualised rows and columns** — only visible DOM nodes are alive; handles millions of rows
- **Multi-column sort** — Shift+click adds sort keys; priority numbers shown on headers
- **Excel-like selection** — click, Ctrl+click, Shift+click, and drag-to-select
- **Per-column filtering** — Contains, StartsWith, and Regex modes via kebab menu
- **Column resize** — drag header borders; min-width enforced
- **Context menu** — right-click for Copy / Select All / Download CSV
- **Keyboard navigation** — ↑↓, Shift+↑↓, Ctrl+A, Ctrl+C, Ctrl+S, Esc
- **Dual theme** — built-in light and dark themes, CSS-variable customization
- **WASM-safe** — no panics in production code, graceful fallbacks

## Installation

```toml
[dependencies]
leptos-arrow-grid = "0.1"
```

Add the stylesheet to `index.html`:

```html
<link rel="stylesheet" href="path/to/arrow-grid.css" />
```

Requires the `wasm32-unknown-unknown` target and [`trunk`](https://trunkrs.dev/):

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
```

## Quick Start

> **The grid owns zero rows.** You manage `page_start`, `sort`, and the `GridPage` signal.

```rust
use std::sync::Arc;
use arrow_array::{Int64Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema};
use leptos::prelude::*;
use leptos_arrow_grid::{ArrowGridStyles, DataGrid, GridPage, SortState};

#[component]
pub fn MyGrid() -> impl IntoView {
    let schema = Arc::new(Schema::new(vec![
        Field::new("id",   DataType::Int64, false),
        Field::new("name", DataType::Utf8,  true),
    ]));
    let batch = Arc::new(RecordBatch::try_new(Arc::clone(&schema), vec![
        Arc::new(Int64Array::from(vec![1, 2, 3])),
        Arc::new(StringArray::from(vec!["Alice", "Bob", "Carol"])),
    ]).expect("valid batch"));

    let page_start = RwSignal::new(0u64);
    let sort       = RwSignal::new(SortState::default());
    let schema_sig = Signal::derive({ let s = Arc::clone(&schema); move || Some(Arc::clone(&s)) });
    let page_sig   = Signal::derive({ let b = Arc::clone(&batch); move || {
        Some(GridPage { start: page_start.get(), row_count: 3, batch: Arc::clone(&b) })
    }});
    view! {
        <ArrowGridStyles />
        <div style="height: 400px;">
            <DataGrid
                schema=schema_sig total_rows=Signal::derive(|| 3)
                page=page_sig sort=sort.into()
                on_viewport_change=Callback::new(move |s: u64| page_start.set(s))
                on_sort_change=Callback::new(move |_| {})
            />
        </div>
    }
}
```

## Examples

See [`EXAMPLES.md`](EXAMPLES.md) for a full walkthrough of the playground.

| Example | What it shows |
|---|---|
| [`playground`](examples/playground/) | 1 M-row in-memory grid — sort, filter, drag-select, CSV download |

## Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `schema` | `Signal<Option<SchemaRef>>` | required | Arrow schema |
| `total_rows` | `Signal<u64>` | required | Total rows (drives scrollbar height) |
| `page` | `Signal<Option<GridPage>>` | required | Current data page |
| `sort` | `Signal<SortState>` | required | Current sort state |
| `on_viewport_change` | `Callback<u64>` | required | New first visible row on scroll |
| `on_sort_change` | `Callback<Vec<(usize, String, SortDirection)>>` | required | Sort change; empty = natural order |
| `row_height` | `f64` | `24.0` | Pixel height per row |
| `show_row_numbers` | `bool` | `true` | Row-number gutter |
| `filters` | `Signal<Vec<Option<FilterKind>>>` | `[]` | Per-column filter state |
| `on_filter_change` | `Callback<(usize, String, Option<FilterKind>)>` | noop | Filter edit |
| `selection` | `RwSignal<SelectionState>` | internal | Host-owned selection signal |
| `col_widths` | `RwSignal<ColumnWidths>` | internal | Host-owned column widths |
| `on_copy_error` | `Callback<String>` | none | Clipboard copy failure |

## Known Limitations

- Fixed row height only (no dynamic heights)
- No column reorder or hide
- CSR only (`wasm32-unknown-unknown`)
- Clipboard requires HTTPS or `localhost`

## Further Reading

- [`docs/arrow-data-integration.md`](docs/arrow-data-integration.md) — building `RecordBatch` from real data
- [`docs/state-management.md`](docs/state-management.md) — why `SelectionState` is pure
- [`docs/theming.md`](docs/theming.md) — CSS variable reference, light/dark themes, scoped themes
- [`TROUBLESHOOTING.md`](TROUBLESHOOTING.md) — clipboard HTTPS, "?" cells, empty grid

## MSRV

Rust **1.94** (edition 2024). Requires `wasm32-unknown-unknown` for browser use.

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md).

## License

MIT OR Apache-2.0
