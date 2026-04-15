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
//! - `on_sort_change`: `Callback(Vec<(col_idx, col_name, SortDirection)>)` when sort changes
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
use crate::viewport::{
    HorizontalViewport, ViewportState, scroll_top_to_virtual_offset_px, scrollable_height_px,
    virtual_offset_to_scroll_top_px,
};

/// Column buffer: render this many extra columns on each side of the viewport.
const COL_BUFFER: usize = 2;

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
    /// Callback: column header sort changed — full priority-ordered list of `(col_idx, col_name, direction)`.
    /// Empty vec = natural order (no sort).
    on_sort_change: Callback<Vec<(usize, String, SortDirection)>>,
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
    let (viewport, set_viewport) = signal(ViewportState::default());
    let h_viewport = RwSignal::new(HorizontalViewport::default());
    let scroll_top_px = RwSignal::new(0.0);
    let virtual_row_offset_px = RwSignal::new(0.0);

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
        let scroll_top = f64::from(el.scroll_top());
        let client_height = f64::from(el.client_height());
        let total = total_rows.get();
        let virtual_scroll_top =
            scroll_top_to_virtual_offset_px(scroll_top, client_height, row_height, total);
        scroll_top_px.set(scroll_top);

        // Horizontal scroll tracking for column virtualization.
        let scroll_left = f64::from(el.scroll_left());
        let container_width = f64::from(el.client_width());
        h_viewport.set(HorizontalViewport {
            scroll_left,
            container_width,
        });

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let start_row = (virtual_scroll_top / row_height).floor() as u64;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let visible_rows = (client_height / row_height).ceil() as usize;
        let visible_rows = visible_rows.max(20); // Never fewer than 20 rows.
        let row_offset =
            (virtual_scroll_top - start_row as f64 * row_height).clamp(0.0, row_height);
        virtual_row_offset_px.set(row_offset);

        // Build proposed new state WITHOUT touching last_emitted yet.
        let next = ViewportState {
            start_row,
            visible_rows,
            last_emitted: None,
        };

        set_viewport.update(|vp| {
            // Carry forward the previous last_emitted so should_emit can compare.
            let candidate = ViewportState {
                last_emitted: vp.last_emitted,
                ..next
            };
            if candidate.should_emit() {
                *vp = candidate.with_emitted();
                on_viewport_change.run(start_row);
            } else {
                // Still update the current values so the signal reflects reality,
                // but do NOT overwrite last_emitted (keeps dedupe intact).
                vp.start_row = start_row;
                vp.visible_rows = visible_rows;
            }
        });
    };

    let on_scroll = move |_| update_viewport();

    // ── Initial viewport on mount ───────────────────────────────
    Effect::new(move |_| {
        if container_ref.get().is_some() {
            #[cfg(target_arch = "wasm32")]
            leptos::prelude::request_animation_frame(move || {
                update_viewport();
            });
            #[cfg(not(target_arch = "wasm32"))]
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

    // ── Column virtualization: visible column range ─────────────
    let visible_cols = Signal::derive(move || {
        let hv = h_viewport.get();
        let base =
            col_widths.with(|cw| cw.visible_range(hv.scroll_left, hv.container_width, COL_BUFFER));
        // If dragging a column resize, ensure it stays in the visible set.
        if let Some((drag_col, _, _)) = drag.get() {
            let (first, count) = base;
            let first = first.min(drag_col);
            let last = (first + count).max(drag_col + 1);
            (first, last - first)
        } else {
            base
        }
    });

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

    // ── Row keys — only the viewport-visible slice of the current page ────
    //
    // Reading `viewport` here means row_keys re-derives on every scroll, but
    // <For> with keyed rendering only DOM-patches the edge rows that actually
    // change — typically 1-2 nodes per scroll tick instead of all 2000.
    let row_keys = Signal::derive(move || -> Vec<(u64, u64, Arc<RecordBatch>)> {
        let vp = viewport.get();
        let render_start = vp.start_row;
        // +2: one extra row for partial-row overlap at viewport bottom.
        let render_end_abs = render_start + vp.visible_rows as u64 + 2;
        match page.get() {
            Some(ref p) => {
                let page_end = p.start + p.row_count as u64;
                let first = render_start.max(p.start);
                let last = render_end_abs.min(page_end);
                if first >= last {
                    return Vec::new();
                }
                (first..last)
                    .map(|virtual_row| (virtual_row, p.start, Arc::clone(&p.batch)))
                    .collect()
            }
            None => Vec::new(),
        }
    });

    let total_height = move || {
        let total = total_rows.get();
        let height = scrollable_height_px(total, row_height);
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
                // Eagerly prevent browser defaults for all keys we handle — stops
                // arrow-key page scrolling, Ctrl+A text selection, etc.
                let ctrl = ev.ctrl_key() || ev.meta_key();
                let key = ev.key();
                if matches!(
                    key.as_str(),
                    "ArrowUp" | "ArrowDown" | "PageUp" | "PageDown"
                        | "Home" | "End" | "Escape"
                ) || (ctrl && matches!(key.as_str(), "a" | "A" | "c" | "C" | "s" | "S"))
                {
                    ev.prevent_default();
                }
                // Escape closes context menu first, then clears selection.
                if key == "Escape" && menu_position.get_untracked().is_some() {
                    menu_position.set(None);
                    return;
                }
                let action = selection.try_update(|s| {
                    keyboard::handle_keydown(
                        &key,
                        ctrl,
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
                                let scroll_top = f64::from(el.scroll_top());
                                let client_height = f64::from(el.client_height());
                                let total = total_rows.get();
                                let virtual_view_top = scroll_top_to_virtual_offset_px(
                                    scroll_top,
                                    client_height,
                                    row_height,
                                    total,
                                );
                                #[allow(clippy::cast_precision_loss)]
                                let target_top = row as f64 * row_height;
                                #[allow(clippy::cast_precision_loss)]
                                let target_bottom = (row as f64 + 1.0) * row_height;
                                if target_top < virtual_view_top {
                                    // Row above viewport: scroll up to reveal at top.
                                    let next_scroll_top = virtual_offset_to_scroll_top_px(
                                        target_top,
                                        client_height,
                                        row_height,
                                        total,
                                    );
                                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                                    el.set_scroll_top(next_scroll_top as i32);
                                } else if target_bottom > virtual_view_top + client_height {
                                    // Row below viewport: scroll down to reveal at bottom.
                                    let next_scroll_top = virtual_offset_to_scroll_top_px(
                                        (target_bottom - client_height).max(0.0),
                                        client_height,
                                        row_height,
                                        total,
                                    );
                                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                                    el.set_scroll_top(next_scroll_top as i32);
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
                visible_cols=visible_cols
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

                        let vp = viewport.get();
                        let scroll_top = scroll_top_px.get();
                        let row_offset = virtual_row_offset_px.get();
                        #[allow(clippy::cast_precision_loss)]
                        let top = scroll_top - row_offset
                            + (virtual_row.saturating_sub(vp.start_row)) as f64 * row_height;

                        // ── Pointer / selection events ──────────────
                        let on_row_down = move |ev: leptos::ev::PointerEvent| {
                            // Focus the container so keyboard events reach our handler.
                            // (prevent_default below suppresses the browser's automatic focus.)
                            #[cfg(target_arch = "wasm32")]
                            if let Some(el) = container_ref.get_untracked() {
                                let _ = el.focus();
                            }
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
                            // Only update when a drag is in progress — avoids marking the
                            // selection signal dirty (and re-evaluating all is_selected
                            // subscriptions) on every mouse-over during normal browsing.
                            if !selection.with_untracked(|s| s.dragging) {
                                return;
                            }
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
                                {move || {
                                    let (first_col, col_count) = visible_cols.get();
                                    let last_col = (first_col + col_count).min(batch.num_columns());
                                    (first_col..last_col).map(|col_idx| {
                                        let value = render_cell(&batch, col_idx, local_idx);
                                        let title_value = value.clone();
                                        let left = col_widths.with(|cw| cw.left_offset(col_idx)) + gutter_w;
                                        let width = col_widths.with(|cw| cw.width(col_idx));
                                        view! {
                                            <div
                                                class="dg-cell"
                                                style:position="absolute"
                                                style:left=format!("{left}px")
                                                style:width=format!("{width}px")
                                                style:height=format!("{row_height}px")
                                                title=title_value
                                            >
                                                {value}
                                            </div>
                                        }
                                    }).collect::<Vec<_>>()
                                }}
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
        assert!(
            !vp.should_emit(),
            "second call with same values must be suppressed"
        );
    }

    #[test]
    fn changed_start_row_breaks_suppression() {
        let vp = ViewportState {
            start_row: 0,
            visible_rows: 20,
            last_emitted: None,
        }
        .with_emitted();

        let vp2 = ViewportState {
            start_row: 5,
            visible_rows: 20,
            last_emitted: vp.last_emitted,
        };
        assert!(vp2.should_emit(), "changed start_row must re-emit");
    }

    #[test]
    fn changed_visible_rows_breaks_suppression() {
        let vp = ViewportState {
            start_row: 0,
            visible_rows: 20,
            last_emitted: None,
        }
        .with_emitted();

        let vp2 = ViewportState {
            start_row: 0,
            visible_rows: 30,
            last_emitted: vp.last_emitted,
        };
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

        assert_eq!(
            emit_count, 1,
            "startup burst of 3 identical viewport reads must emit exactly once"
        );
    }
}
