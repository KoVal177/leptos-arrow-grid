# Troubleshooting

## "My clipboard copy isn't working"

**Symptom:** `Ctrl+C` does nothing; the `on_copy_error` callback fires with a message like `"NotAllowedError"`.

**Cause:** The browser Clipboard API (`navigator.clipboard.writeText`) only works in a **secure context** — that is, a page served over HTTPS or from `localhost`. HTTP pages on any other origin are silently denied.

**Fixes:**
- Develop on `trunk serve` at `http://localhost:8080` (this counts as a secure context).
- Serve your production build over HTTPS.
- As a fallback, you can display the `on_copy_error` message in your UI so the user knows to manually copy the selection.

```rust
on_copy_error=Callback::new(move |err: String| {
    set_status.set(format!("Copy failed — {err}. Please use HTTPS."));
})
```

---

## "My columns are all showing '?'"

**Symptom:** Every cell in the grid displays `"?"` instead of data.

**Cause:** The `RecordBatch` you passed in has zero rows, or the column indices are out of bounds relative to the schema.

**Diagnosis:**
1. Check `total_rows` — if it's `0`, the grid renders no rows and you'd see a blank grid, not `"?"`.
2. Check `GridPage::row_count` — if you pass `row_count: 5` but the `RecordBatch` has only 3 rows, the grid will call `render_cell` with out-of-bounds indices, returning `"?"`.
3. Check that `page.start` + `page.row_count` does not exceed `total_rows`.

**Fix:** Ensure `row_count == batch.num_rows()` and `start + row_count <= total_rows`.

```rust
// Correct:
let row_count = batch.num_rows();
Some(GridPage { start, row_count, batch })
```

---

## "The grid renders but shows no rows"

**Symptom:** The grid header is visible, the scrollbar appears at full height, but no data rows appear.

**Possible causes:**

### A) Container height is 0

The virtual scroll engine calculates how many rows fit by dividing the container's `clientHeight` by `row_height`. If the container has `height: 0` (or inherits one from a flex parent without an explicit height), `clientHeight == 0` → 0 visible rows → no `on_viewport_change` fires → no data is requested.

```rust
// WRONG — no height set
view! { <DataGrid ... /> }

// CORRECT
view! {
    <div style="height: 600px;">
        <DataGrid ... />
    </div>
}
```

### B) `total_rows` is 0

If `total_rows` returns `0`, the scroll spacer is 0 px tall and no rows are rendered. Make sure your data source is loaded before passing `total_rows`.

### C) `page` returns `None`

The grid renders nothing when `page.get()` is `None`. This is intentional (loading state), but verify your `Signal::derive` returns `Some(GridPage { ... })` when data is available.

---

## "The grid is laggy when sorting large datasets"

**Symptom:** Clicking a column header freezes the browser for several seconds.

**Cause:** The sort runs synchronously on the browser's WASM thread, blocking rendering until it completes. For 1 M rows with 5 string columns this can take 500 ms+.

**Fix options:**
- **Cap the sortable set**: The playground limits sorting to `MAX_SORTABLE` rows. Reduce it for your use case.
- **Sort server-side**: Push sort+filter parameters to your backend and return a pre-sorted page; the grid only receives 100–200 rows at a time.
- **Show a loading indicator**: Set `sort.building = true` and use `spawn_local` to clear it after the reactive system flushes. The header column will show a pulsing highlight during the sort (the `--lag-warning` colour).

---

## "Ctrl+S opens the browser's Save dialog"

**Symptom:** Pressing `Ctrl+S` on the grid triggers the browser's "Save page as…" dialog instead of the CSV download.

**Cause:** The grid's `keydown` handler calls `ev.prevent_default()` for `Ctrl+S`, but the event must bubble to the grid's container `<div>` for that to work. If your layout puts a `<textarea>` or `<input>` on top, focus may be on that element instead.

**Fix:** Ensure the grid container div has `tabindex="0"` and receives focus when the user interacts with a row. The grid already sets this; wrapping `<div>`s in your layout must not swallow the event before it reaches the grid.

---

## "Cells with `Decimal128` / `Timestamp` show weird output"

**Symptom:** Numeric timestamps or decimal values render as large integers or in an unexpected format.

**Cause:** `render_cell` has native fast paths for the most common types. All others fall back to Arrow's `ArrayFormatter`, which uses ISO 8601 for timestamps and scientific notation for `Decimal128` without a scale applied.

**Fix:** Pre-process these columns before building your `RecordBatch` — cast them to `Utf8` with your desired format — or post-process the string returned by `render_cell` in a custom cell renderer (a `DataGrid` prop planned for a future release).
