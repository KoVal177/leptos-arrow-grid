# Arrow Data Integration

## What is a `RecordBatch`?

An `Arc<RecordBatch>` is the core data primitive in leptos-arrow-grid. A `RecordBatch` is an Apache Arrow columnar in-memory table: it bundles a `SchemaRef` (column names + types) with one Arrow array per column, all sharing the same row count.

Because `RecordBatch` is wrapped in `Arc`, cloning it is `O(1)` — the grid never copies row data, it just increments a reference count.

## The `GridPage` Struct

```rust
pub struct GridPage {
    /// Absolute row index of the first row in this page.
    pub start: u64,
    /// Number of rows in `batch`.
    pub row_count: usize,
    /// The Arrow data for this page window.
    pub batch: Arc<RecordBatch>,
}
```

`start` is the grid-wide row offset. If the user has scrolled to row 500 and you are showing rows 500–599, `start = 500` and `row_count = 100`.

## Building a `RecordBatch` from Rust Vecs

```rust
use std::sync::Arc;
use arrow_array::{Float64Array, Int64Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema};

fn build_batch(ids: Vec<i64>, names: Vec<&str>, scores: Vec<f64>) -> Arc<RecordBatch> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("id",    DataType::Int64,   false),
        Field::new("name",  DataType::Utf8,    true),
        Field::new("score", DataType::Float64, false),
    ]));

    Arc::new(
        RecordBatch::try_new(
            Arc::clone(&schema),
            vec![
                Arc::new(Int64Array::from(ids)),
                Arc::new(StringArray::from(names)),
                Arc::new(Float64Array::from(scores)),
            ],
        )
        .expect("schema and arrays must have matching lengths"),
    )
}
```

> **Tip:** All arrays passed to `RecordBatch::try_new` must have the same length; otherwise it returns `Err`.

## Nullable Columns

Wrap column values in `Option<T>` to mark nulls:

```rust
// A column with some null values
Arc::new(StringArray::from(vec![Some("Alice"), None, Some("Carol")]))
```

The grid renders `None`/null cells as `"NULL"`.

## Reacting to `on_viewport_change`

The grid calls `on_viewport_change(start_row)` whenever the user scrolls. Your job is to update a `page_start` signal and re-derive the `GridPage`:

```rust
let page_start = RwSignal::new(0u64);

// Inside the component:
on_viewport_change=Callback::new(move |start: u64| {
    page_start.set(start);
})
```

Then derive your page reactively:

```rust
let page = Signal::derive(move || {
    let start = page_start.get() as usize;
    // `rows` is your full Vec<MyRow> or similar in-memory store
    let window = &rows[start..(start + PAGE_SIZE).min(rows.len())];
    let batch = build_batch_from_window(window);
    Some(GridPage { start: start as u64, row_count: window.len(), batch })
});
```

## Async Data Fetching

For remote data, trigger your fetch inside a `spawn_local` (the grid itself is synchronous):

```rust
let page_start = RwSignal::new(0u64);
let page: RwSignal<Option<GridPage>> = RwSignal::new(None);

on_viewport_change=Callback::new(move |start: u64| {
    page_start.set(start);
    leptos::task::spawn_local(async move {
        let batch = fetch_page(start).await;
        page.set(Some(GridPage { start, row_count: batch.num_rows(), batch }));
    });
})
```

## Handling Sort

When `on_sort_change` fires, you receive `(col_idx, col_name, Option<SortDirection>)`. Reset `page_start` to `0` after applying the sort so the grid scrolls back to the top:

```rust
on_sort_change=Callback::new(move |(col, _name, dir): (usize, String, Option<SortDirection>)| {
    sort.update(|s| { s.active = dir.map(|d| (col, d)); });
    page_start.set(0);
})
```

## Supported Arrow Types

`render_cell` has fast paths for:

| Arrow type | Displayed as |
|---|---|
| `Int8/16/32/64`, `UInt8/16/32/64` | Decimal integer string |
| `Float32/64` | Decimal float (trailing zeros stripped) |
| `Boolean` | `"true"` / `"false"` |
| `Utf8`, `LargeUtf8`, `Utf8View` | As-is string value |
| `Null` / any null slot | `"NULL"` |
| Out-of-bounds index | `"?"` |
| All other types | Arrow's `ArrayFormatter` fallback (Date, Timestamp, Decimal, …) |
