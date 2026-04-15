# Changelog

All notable changes to `leptos-arrow-grid` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.3] — 2026-04-15

### Added
- **Multi-column sort** — `SortState.sorts: Vec<SortSpec>` replaces the single-column sort
  state. Shift+click adds a column to the sort; plain click replaces it. Priority numbers
  (1↑, 2↓) are shown in column headers for multi-key sorts.
- `cycle_sort_multi(current, col_idx, additive)` — functional sort-state reducer that drives
  both additive and replace-mode sort transitions.
- `on_sort_change: Callback<Vec<(usize, String, SortDirection)>>` — empty vec signals natural
  order (sort cleared). Replaces the old single-column callback.
- `read_sort_columns()` helper for multi-column Parquet pushdown.
- `read_csv_sorted_page(sorts: &[(&str, bool)])` for multi-column CSV `ORDER BY` generation.

### Changed
- `SortState.active: Vec<(usize, SortDirection)>` — ordered list of active sort columns
  (index + direction), replacing the previous `Option<(usize, SortDirection)>` single-column
  state.

### Removed
- `sort_disabled` field removed entirely (was always `false`; dead code).

---

## [0.1.2] — 2026-04-08

### Added
- **Column virtualisation** — off-screen columns are unmounted via horizontal binary search;
  rendering cost now scales with visible columns, not total schema width.
- `top: 0` on `.dg-cell` — critical correctness fix: cells were displaced below their row's
  bottom edge by the `position: sticky` row-number gutter, making data invisible under opaque
  row backgrounds.
- Explicit `background: var(--lag-bg-primary)` on `.dg-row` — enables correct compositing for
  hover and selection highlights without bleed-through from underlying rows.
- Playground expanded to **20 columns** (+5 new: `avg_rating`, `login_count`, `country`,
  `account_type`, `is_verified`) to stress-test column virtualisation.
- Playground toolbar separator styling.

### Changed
- **Keyboard navigation** — eager `prevent_default()` at handler entry for all grid-handled
  keys (↑↓ PgUp PgDn Home End Esc Ctrl+A/C/S); eliminates native page scroll and text-select
  side-effects.
- **Scroll-to-row** — switched from center-jump to minimal "reveal at edge" behaviour: scrolls
  only enough to bring the target row into view, matching standard spreadsheet UX.
- Grid container now calls `el.focus()` on pointer-down, restoring keyboard focus after click
  when the browser's automatic focus is suppressed by `prevent_default`.
- `.dg-row:hover` accent mix tuned: 12 % → 10 % (lighter hover highlight).
- `.dg-row--selected` compositing corrected: was `color-mix(… transparent)` (invisible in many
  browsers), now `color-mix(… var(--lag-bg-primary))` at 22 %.
- **Sort performance** — `Memo<SortBuf>` replaces `Memo<Option<Vec<usize>>>`: sorted indices
  are `Arc`-wrapped so every `.get()` is a pointer clone (O(1)) instead of an 8 MB copy, and
  Leptos change-detection uses pointer equality instead of O(n) `Vec::eq`.
- **Comparison performance** — `compare_rows` for the email (col 2) and phone (col 14) columns
  now uses zero-allocation integer arithmetic instead of `format!()` string construction;
  eliminates ~20–40 M heap allocations per 1 M-row sort that previously fragmented the WASM
  allocator and caused progressive slowdown after repeated sorts.

### Fixed
- Column virtualisation: `rAF`-gated mount prevents flash-of-unrendered-columns on initial
  load.
- Header title tooltip now shows full column name on overflow.
- Various `clippy` warnings resolved (`is_multiple_of`, `cast_sign_loss`).

---

## [0.1.1] — 2026-04-05

### Added
- Viewport storm fix — debounced scroll handler with `requestAnimationFrame` prevents redundant
  re-renders during rapid wheel events.
- Light and dark built-in themes (`ArrowGridTheme`, `ArrowGridThemeScope`).
- CSS variable theming reference (`docs/theming.md`).

---

## [0.1.0] — 2026-04-01

### Added
- Initial public release.
- `DataGrid` component: virtualised rows, Arrow `RecordBatch` data contract.
- Excel-like selection (click, Ctrl+click, Shift+click, drag lasso).
- 3-state column sort with per-column indicators.
- Column resize via drag handle.
- Per-column filtering (Contains / StartsWith / Regex).
- Context menu (Copy / Select All / Download CSV).
- Clipboard TSV copy (`Ctrl+C`) and CSV download (`Ctrl+S`).
- Keyboard navigation (↑↓, Shift+↑↓, Ctrl+A, Esc).
- `extra_menu_items` headless slot for custom kebab-menu entries.
- Host-owned `SelectionState` and `ColumnWidths` props.
- `on_copy_error` callback for non-HTTPS clipboard failures.
- `ArrowGridStyles` / `ArrowGridTheme` / `ArrowGridThemeScope` theming API.
- Playground example (`examples/playground/`) with 1 M-row in-memory demo.
