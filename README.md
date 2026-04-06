# leptos-arrow-grid

The high-performance, virtualized data grid for Leptos, powered by Apache Arrow.

## Features

- Zero-copy rendering from `Arc<RecordBatch>` — no serialisation
- Virtualised rows: renders only visible DOM nodes regardless of dataset size
- Excel-like selection: single, Ctrl+click, Shift+click, drag
- 3-state column sort (Asc → Desc → Natural)
- Column resize, context menu, clipboard TSV copy
- Keyboard navigation (arrow keys, Page Up/Down, Home/End)
- CSS-variable theming — bring your own design tokens

## Quick Start

```toml
[dependencies]
leptos-arrow-grid = "0.1"
```

```rust
use leptos_arrow_grid::{DataGrid, GridPage, SortState};
```

## Visual Playground

```bash
cd examples/playground
trunk serve
```

## License

MIT OR Apache-2.0
