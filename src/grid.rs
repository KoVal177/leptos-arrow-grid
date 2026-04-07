//! `DataGrid` — virtual scrolling grid backed by page-based data.
//!
//! # Contract
//!
//! The grid does NOT fetch data. It receives:
//! - `schema`: Arrow schema for column names/types
//! - `total_rows`: Total row count in the dataset
//! - `page`: The current data page (reactive signal)
//! - `sort`: Current sort state (reactive signal)
//!
//! The grid emits:
//! - `on_viewport_change`: `Callback(start_row: u64)` when the visible range changes
//! - `on_sort_change`: `Callback((col_idx, col_name, Option<SortDirection>))` when sort cycles
//! - `on_filter_change`: `Callback((col_idx, col_name, Option<FilterKind>))` when filter changes

use std::sync::Arc;

use arrow_array::RecordBatch;
use arrow_schema::SchemaRef;
use leptos::prelude::*;

use crate::cell::render_cell;
use crate::clipboard;
use crate::column_state::ColumnWidths;
use crate::context_menu::{ContextAction, GridContextMenu, MenuPosition};
use crate::header::{HeaderCellData, HeaderRow};
use crate::keyboard;
use crate::selection::SelectionState;
use crate::types::{
    DEFAULT_COL_WIDTH_PX, FilterKind, GridPage, MenuItem, ROW_NUM_WIDTH_PX, SortDirection,
    SortState, format_row_number,
};
use crate::viewport::ViewportState;

/// The data grid component — virtual scrolling, sort headers, column resize, row numbers.
#[allow(unreachable_pub)]
#[allow(clippy::too_many_lines)]
#[component]
pub fn DataGrid(
    /// Arrow schema (column names and types).
    schema: Signal<Option<SchemaRef>>,
    /// Total rows in the dataset.
    total_rows: Signal<u64>,
    /// Current data page.
    page: Signal<Option<GridPage>>,
    /// Current sort state.
    sort: Signal<SortState>,
    /// Row height in pixels.
    #[prop(default = 24.0)]
    row_height: f64,
    /// Show row numbers in a left gutter.
    #[prop(default = true)]
    show_row_numbers: bool,
    /// Active filters per column — indexed by column position.
    #[prop(optional)]
    filters: Option<Signal<Vec<Option<FilterKind>>>>,
    /// Callback: viewport start row changed.
    on_viewport_change: Callback<u64>,
    /// Callback: column header sort changed — `(col_idx, col_name, new_direction)`.
    on_sort_change: Callback<(usize, String, Option<SortDirection>)>,
    /// Callback: filter changed — `(col_idx, col_name, new_filter)`.
    #[prop(optional)]
    on_filter_change: Option<Callback<(usize, String, Option<FilterKind>)>>,
    /// Builder for extra menu items per column (headless slot).
    #[prop(optional)]
    extra_menu_items: Option<Callback<usize, Vec<MenuItem>>>,
    /// Selection state — host-owned. If not provided, grid creates internal default.
    #[prop(optional)]
    selection: Option<RwSignal<SelectionState>>,
    /// Column widths — host-owned. If not provided, grid creates internal default.
    #[prop(optional)]
    col_widths: Option<RwSignal<ColumnWidths>>,
    /// Callback when clipboard copy fails (e.g., permission denied).
    #[prop(optional)]
    on_copy_error: Option<Callback<String>>,
) -> impl IntoView {
    let container_ref = NodeRef::<leptos::html::Div>::new();
    let (_viewport, set_viewport) = signal(ViewportState::default());

    // Unwrap optional props with defaults.
    let filters = filters.unwrap_or_else(|| Signal::derive(Vec::new));
    let on_filter_change = on_filter_change.unwrap_or_else(|| Callback::new(|_| {}));

    // ── Selection state ─────────────────────────────────────
    let selection: RwSignal<SelectionState> =
        selection.unwrap_or_else(|| RwSignal::new(SelectionState::default()));

    // Clear selection when schema changes (new dataset).
    Effect::new(move || {
        let _ = schema.get(); // subscribe
        selection.update(SelectionState::clear);
    });

    // ── Context menu state ──────────────────────────────────────
    let menu_position: RwSignal<Option<MenuPosition>> = RwSignal::new(None);

    // ── Per-column widths ───────────────────────────────────────
    let col_widths: RwSignal<ColumnWidths> =
        col_widths.unwrap_or_else(|| RwSignal::new(ColumnWidths::new(0, DEFAULT_COL_WIDTH_PX)));

    // Active drag: (column_index, pointer_start_x, width_at_start).
    let drag: RwSignal<Option<(usize, f64, f64)>> = RwSignal::new(None);

    // Sync col_widths when schema changes.
    Effect::new(move || {
        if let Some(s) = schema.get() {
            let n = s.fields().len();
            col_widths.update(|cw| {
                if cw.len() != n {
                    *cw = ColumnWidths::new(n, DEFAULT_COL_WIDTH_PX);
                }
            });
        }
    });

    // ── Viewport calculation (shared by scroll, mount, resize) ─
    let update_viewport = move || {
        let Some(el) = container_ref.get_untracked() else {
            return;
        };
        let scroll_top    = f64::from(el.scroll_top());
        let client_height = f64::from(el.client_height());

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let start_row    = (scroll_top / row_height).floor() as u64;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let visible_rows = (client_height / row_height).ceil() as usize;

        // Build proposed new state WITHOUT touching last_emitted yet.
        let next = ViewportState { start_row, visible_rows, last_emitted: None };

        set_viewport.update(|vp| {
            // Carry forward the previous last_emitted so should_emit can compare.
            let candidate = ViewportState { last_emitted: vp.last_emitted, ..next };
            if candidate.should_emit() {
                *vp = candidate.with_emitted();
                on_viewport_change.run(start_row);
            } else {
                // Still update the current values so the signal reflects reality,
                // but do NOT overwrite last_emitted (keeps dedupe intact).
                vp.start_row    = start_row;
                vp.visible_rows = visible_rows;
            }
        });
    };

    let on_scroll = move |_| update_viewport();

    // ── Initial viewport on mount ───────────────────────────────
    Effect::new(move |_| {
        if container_ref.get().is_some() {
            update_viewport();
        }
    });

    // ── Recalculate viewport on container resize ────────────────
    // ResizeObserver fires on every layout frame during row insertion.
    // update_viewport is idempotent: it only calls on_viewport_change when the
    // computed (start_row, visible_rows) pair differs from last_emitted.
    #[cfg(target_arch = "wasm32")]
    {
        use send_wrapper::SendWrapper;
        use wasm_bindgen::prelude::*;

        Effect::new(move |_| {
            let Some(el) = container_ref.get() else {
                return;
            };
            let cb = Closure::<dyn Fn(js_sys::Array)>::new(move |_entries: js_sys::Array| {
                update_viewport();
            });
            let Ok(observer) = web_sys::ResizeObserver::new(cb.as_ref().unchecked_ref()) else {
                return;
            };
            observer.observe(&el);
            // SendWrapper lets !Send WASM types satisfy on_cleanup's Send+Sync bound.
            // WASM is single-threaded so take() will never panic.
            let cleanup_data = SendWrapper::new((observer, cb));
            on_cleanup(move || {
                let (obs, _cb) = cleanup_data.take();
                obs.disconnect();
            });
        });
    }

    // ── Scroll to top when sort changes ────────────────────────
    // Skip first run (prev == None): mount Effect already called update_viewport.
    Effect::new(move |prev: Option<()>| {
        let _ = sort.get(); // subscribe
        if prev.is_none() {
            return;
        }
        if let Some(el) = container_ref.get_untracked() {
            el.set_scroll_top(0);
        }
        update_viewport();
    });

    // ── Scroll to top when filters change ──────────────────────
    // Skip first run (prev == None): mount Effect already called update_viewport.
    Effect::new(move |prev: Option<()>| {
        let _ = filters.get(); // subscribe
        if prev.is_none() {
            return;
        }
        if let Some(el) = container_ref.get_untracked() {
            el.set_scroll_top(0);
        }
        update_viewport();
    });

    // ── Total width (for data rows) ─────────────────────────────
    let gutter_w = if show_row_numbers {
        ROW_NUM_WIDTH_PX
    } else {
        0.0
    };

    let total_width = move || {
        let data_w = col_widths.with(super::column_state::ColumnWidths::total_width);
        gutter_w + data_w
    };

    // ── Header items (schema-derived, no sort state) ────────────
    let header_items = Signal::derive(move || {
        schema
            .get()
            .map(|s| {
                s.fields()
                    .iter()
                    .enumerate()
                    .map(|(idx, f)| HeaderCellData {
                        idx,
                        name: f.name().to_owned(),
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    });

    // ── Row keys ────────────────────────────────────────────────
    let row_keys = Signal::derive(move || -> Vec<(u64, u64, Arc<RecordBatch>)> {
        match page.get() {
            Some(ref p) => (0..p.row_count)
                .map(|local_idx| {
                    let virtual_row = p.start + local_idx as u64;
                    (virtual_row, p.start, Arc::clone(&p.batch))
                })
                .collect(),
            None => Vec::new(),
        }
    });

    let total_height = move || {
        let total = total_rows.get();
        #[allow(clippy::cast_precision_loss)]
        let height = total as f64 * row_height;
        format!("{height}px")
    };

    view! {
        <div
            class="dg-container"
            node_ref=container_ref
            tabindex="0"
            on:scroll=on_scroll
            on:pointerup=move |_| selection.update(SelectionState::on_pointer_up)
            on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                // Escape closes context menu first, then clears selection.
                if ev.key() == "Escape" && menu_position.get_untracked().is_some() {
                    menu_position.set(None);
                    ev.prevent_default();
                    return;
                }
                let action = selection.try_update(|s| {
                    keyboard::handle_keydown(
                        &ev.key(),
                        ev.ctrl_key() || ev.meta_key(),
                        ev.shift_key(),
                        s,
                        total_rows.get(),
                    )
                });
                if let Some(action) = action {
                    match action {
                        keyboard::KeyAction::None => {}
                        keyboard::KeyAction::ScrollTo(row) => {
                            if let Some(el) = container_ref.get_untracked() {
                                #[allow(clippy::cast_precision_loss)]
                                let target_top = row as f64 * row_height;
                                let scroll_top = f64::from(el.scroll_top());
                                let client_height = f64::from(el.client_height());
                                if target_top < scroll_top
                                    || target_top > scroll_top + client_height - row_height
                                {
                                    #[allow(clippy::cast_possible_truncation)]
                                    let pos = (target_top - client_height / 2.0).max(0.0) as i32;
                                    el.set_scroll_top(pos);
                                }
                            }
                            ev.prevent_default();
                        }
                        keyboard::KeyAction::Copy => {
                            if let Some(s) = schema.get() {
                                let tsv = selection.with_untracked(|sel| {
                                    clipboard::build_tsv(&sel.selected, &s, &page.get())
                                });
                                clipboard::copy_to_clipboard(&tsv, on_copy_error);
                            }
                            ev.prevent_default();
                        }
                        keyboard::KeyAction::Download => {
                            if let Some(s) = schema.get() {
                                let csv = selection.with_untracked(|sel| {
                                    crate::download::build_csv(&sel.selected, &s, &page.get())
                                });
                                crate::download::download_csv_file(&csv);
                            }
                            ev.prevent_default();
                        }
                    }
                }
            }
        >
            <HeaderRow
                header_items=header_items
                col_widths=col_widths
                drag=drag
                show_row_numbers=show_row_numbers
                sort=sort
                filters=filters
                on_sort_change=on_sort_change
                on_filter_change=on_filter_change
                extra_menu_items=extra_menu_items
            />
            // ── Virtual scroll spacer + data rows ───────────
            <div class="dg-scroll-spacer" style:height=total_height style:width=move || format!("{}px", total_width())>
                <For
                    each=move || row_keys.get()
                    key=|(virtual_row, _, batch)| (*virtual_row, std::sync::Arc::as_ptr(batch) as usize)
                    children=move |(virtual_row, page_start, batch)| {
                        let local_idx =
                            usize::try_from(virtual_row - page_start).unwrap_or(usize::MAX);

                        #[allow(clippy::cast_precision_loss)]
                        let top = virtual_row as f64 * row_height;

                        let col_count = batch.num_columns();

                        // ── Pointer / selection events ──────────────
                        let on_row_down = move |ev: leptos::ev::PointerEvent| {
                            ev.prevent_default();
                            if ev.button() != 2 {
                                let total = total_rows.get();
                                selection.update(|s| {
                                    s.on_pointer_down(
                                        virtual_row,
                                        ev.ctrl_key() || ev.meta_key(),
                                        ev.shift_key(),
                                        total,
                                    );
                                });
                            }
                        };
                        let on_row_enter = move |_: leptos::ev::PointerEvent| {
                            let total = total_rows.get();
                            selection.update(|s| s.on_pointer_enter_drag(virtual_row, total));
                        };
                        let on_row_context = move |ev: leptos::ev::MouseEvent| {
                            ev.prevent_default();
                            selection.update(|s| s.on_context_menu(virtual_row));
                            #[allow(clippy::cast_precision_loss)]
                            menu_position.set(Some(MenuPosition {
                                x: f64::from(ev.page_x()),
                                y: f64::from(ev.page_y()),
                            }));
                        };

                        let is_selected = Signal::derive(move || {
                            selection.with(|s| s.is_selected(virtual_row))
                        });

                        view! {
                            <div
                                class="dg-row"
                                class:dg-row--selected=is_selected
                                style:position="absolute"
                                style:top=format!("{top}px")
                                style:height=format!("{row_height}px")
                                style:width=move || format!("{}px", total_width())
                                on:pointerdown=on_row_down
                                on:pointerenter=on_row_enter
                                on:contextmenu=on_row_context
                            >
                                {show_row_numbers.then(|| {
                                    let row_num = virtual_row + 1;
                                    let formatted = format_row_number(row_num);
                                    let title_str = formatted.clone();
                                    view! {
                                        <div
                                            class="dg-row-num"
                                            style:width=format!("{ROW_NUM_WIDTH_PX}px")
                                            title=title_str
                                        >
                                            {formatted}
                                        </div>
                                    }
                                })}
                                {(0..col_count).map(|col_idx| {
                                    let value = render_cell(&batch, col_idx, local_idx);
                                    view! {
                                        <div
                                            class="dg-cell"
                                            style:width=move || col_widths.with(|cw| format!("{}px", cw.width(col_idx)))
                                            style:flex-shrink="0"
                                        >
                                            {value}
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        }
                    }
                />
            </div>
            <GridContextMenu
                position=menu_position.into()
                on_action=Callback::new(move |action| {
                    match action {
                        ContextAction::Copy => {
                            if let Some(s) = schema.get() {
                                let tsv = selection.with_untracked(|sel| {
                                    clipboard::build_tsv(&sel.selected, &s, &page.get())
                                });
                                clipboard::copy_to_clipboard(&tsv, on_copy_error);
                            }
                        }
                        ContextAction::SelectAll => {
                            selection.update(|s| s.select_all(total_rows.get()));
                        }
                        ContextAction::Download => {
                            if let Some(s) = schema.get() {
                                let csv = selection.with_untracked(|sel| {
                                    crate::download::build_csv(&sel.selected, &s, &page.get())
                                });
                                crate::download::download_csv_file(&csv);
                            }
                        }
                    }
                })
                on_close=Callback::new(move |()| menu_position.set(None))
                selected_count=Signal::derive(move || selection.with(super::selection::SelectionState::count))
            />
        </div>
    }
}


#[cfg(test)]
mod viewport_effect_tests {
    use crate::viewport::ViewportState;

    /// Simulate multiple `update_viewport` calls with the same computed values.
    /// Verifies that `should_emit` suppresses duplicate emissions.
    #[test]
    fn dedupe_suppresses_same_viewport() {
        let mut vp = ViewportState::default();

        // First call — always emits (last_emitted is None)
        assert!(vp.should_emit(), "first call must emit");
        vp = vp.with_emitted();

        // Second call, same values — must NOT emit
        assert!(!vp.should_emit(), "second call with same values must be suppressed");
    }

    #[test]
    fn changed_start_row_breaks_suppression() {
        let vp = ViewportState { start_row: 0, visible_rows: 20, last_emitted: None }
            .with_emitted();

        let vp2 = ViewportState { start_row: 5, visible_rows: 20, last_emitted: vp.last_emitted };
        assert!(vp2.should_emit(), "changed start_row must re-emit");
    }

    #[test]
    fn changed_visible_rows_breaks_suppression() {
        let vp = ViewportState { start_row: 0, visible_rows: 20, last_emitted: None }
            .with_emitted();

        let vp2 = ViewportState { start_row: 0, visible_rows: 30, last_emitted: vp.last_emitted };
        assert!(vp2.should_emit(), "changed visible_rows must re-emit");
    }

    /// Simulate the startup storm: mount, sort, and filter Effects all fire
    /// `update_viewport` with `start_row=0`.  Only the first call should emit.
    #[test]
    fn startup_storm_emits_once() {
        let mut vp = ViewportState::default();
        let mut emit_count = 0u32;

        // Simulate 3 calls with identical computed values (the startup burst)
        for _ in 0..3 {
            if vp.should_emit() {
                emit_count += 1;
                vp = vp.with_emitted();
            }
        }

        assert_eq!(emit_count, 1, "startup burst of 3 identical viewport reads must emit exactly once");
    }
}
