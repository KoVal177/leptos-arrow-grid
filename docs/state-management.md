# State Management

## Why `SelectionState` Is Pure

`SelectionState` is a plain Rust struct — it has no internal Leptos signals, no `RwSignal`, no `Cell`. It is pure data:

```rust
pub struct SelectionState {
    pub selected: HashSet<u64>,   // absolute row indices
    pub anchor:   Option<u64>,    // shift-click anchor
    pub cursor:   Option<u64>,    // keyboard cursor
    pub dragging: bool,
}
```

**Why pure?** Because the grid receives `selection` as an `RwSignal<SelectionState>` from the outside and updates it via `.update()`. This means:

- You, the consumer, full own the reactivity wrapper.
- The grid can mutate selection inside a single `update()` call without triggering intermediate renders.
- You can read the selection outside the grid (`selection.with(|s| s.selected.clone())`) at any time.
- Pure structs are trivially testable — no reactor needed in unit tests.

## Wrapping in `RwSignal`

Declare the signal in your component (not inside `DataGrid`):

```rust
use leptos_arrow_grid::SelectionState;

let selection = RwSignal::new(SelectionState::default());
```

DataGrid takes it as an opaque `RwSignal<SelectionState>` prop. You can read it reactively to drive other parts of your UI:

```rust
// Show a status bar count
let count = Signal::derive(move || selection.with(|s| s.selected.len()));
view! {
    <p>{move || format!("{} rows selected", count.get())}</p>
    <DataGrid selection=selection ... />
}
```

## Reading Selection Outside the Grid

```rust
// One-shot read (non-reactive)
let rows: Vec<u64> = selection.with_untracked(|s| {
    let mut v: Vec<u64> = s.selected.iter().copied().collect();
    v.sort_unstable();
    v
});

// Reactive read (re-runs when selection changes)
let row_count = Signal::derive(move || selection.with(SelectionState::count));
```

## Programmatic Select All / Clear

```rust
// Select all rows
selection.update(|s| s.select_all(total_rows.get()));

// Clear selection
selection.update(|s| s.clear());
```

## SelectionState in Unit Tests

Because the struct is pure, tests don't need a Leptos reactor:

```rust
use leptos_arrow_grid::SelectionState;

#[test]
fn shift_click_selects_range() {
    let mut s = SelectionState::default();
    s.on_pointer_down(0, false, false, 10);   // click row 0
    s.on_pointer_down(4, false, true,  10);   // shift-click row 4
    assert_eq!(s.count(), 5);
}
```

## The `SortState` Signal

Sort state works the same way — owned by you, passed into the grid:

```rust
let sort = RwSignal::new(SortState::default());

// Read reactive sort info elsewhere
let sort_col = Signal::derive(move || sort.with(|s| s.active.map(|(col, _)| col)));
```

`SortState::building` is set to `true` while the grid header is animating a sort-in-progress indicator. You should clear it after your sort completes (the playground uses `spawn_local`):

```rust
on_sort_change=Callback::new(move |(col, _name, dir)| {
    sort.update(|s| { s.active = dir.map(|d| (col, d)); s.building = true; });
    page_start.set(0);
    leptos::task::spawn_local(async move {
        sort.update(|s| { s.building = false; });
    });
})
```
