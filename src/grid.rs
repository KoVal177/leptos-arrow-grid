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
    FilterKind, GridPage, MenuItem, SortDirection, SortState, DEFAULT_COL_WIDTH_PX,
    ROW_NUM_WIDTH_PX, format_row_number,
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
) -> impl IntoView {
    let container_ref = NodeRef::<leptos::html::Div>::new();
    let (_viewport, set_viewport) = signal(ViewportState::default());

    // Unwrap optional props with defaults.
    let filters = filters.unwrap_or_else(|| Signal::derive(Vec::new));
    let on_filter_change = on_filter_change.unwrap_or_else(|| Callback::new(|_| {}));

    // ── Selection state ─────────────────────────────────────────
    let selection: RwSignal<SelectionState> = RwSignal::new(SelectionState::default());

    // Clear selection when schema changes (new dataset).
    Effect::new(move || {
        let _ = schema.get(); // subscribe
        selection.update(SelectionState::clear);
    });

    // ── Context menu state ──────────────────────────────────────
    let menu_position: RwSignal<Option<MenuPosition>> = RwSignal::new(None);

    // ── Per-column widths ───────────────────────────────────────
    let col_widths: RwSignal<ColumnWidths> =
        RwSignal::new(ColumnWidths::new(0, DEFAULT_COL_WIDTH_PX));

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

    // ── Scroll handler ──────────────────────────────────────────
    let on_scroll = move |_| {
        let Some(el) = container_ref.get() else {
            return;
        };
        let scroll_top = f64::from(el.scroll_top());
        let client_height = f64::from(el.client_height());

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let start_row = (scroll_top / row_height).floor() as u64;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let visible_rows = (client_height / row_height).ceil() as usize;

        set_viewport.set(ViewportState {
            start_row,
            visible_rows,
        });
        on_viewport_change.run(start_row);
    };

    // ── Total width (for data rows) ─────────────────────────────
    let gutter_w = if show_row_numbers { ROW_NUM_WIDTH_PX } else { 0.0 };

    let total_width = move || {
        let data_w = col_widths.with(super::column_state::ColumnWidths::total_width);
        #[allow(clippy::cast_precision_loss)]
        let handle_w = col_widths.with(|cw| cw.len() as f64 * 4.0);
        gutter_w + data_w + handle_w
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
                            if let Some(el) = container_ref.get() {
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
                                clipboard::copy_to_clipboard(&tsv);
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
            <div class="dg-scroll-spacer" style:height=total_height>
                <For
                    each=move || row_keys.get()
                    key=|(virtual_row, _, _)| *virtual_row
                    children=move |(virtual_row, page_start, batch)| {
                        let local_idx =
                            usize::try_from(virtual_row - page_start).unwrap_or(usize::MAX);

                        #[allow(clippy::cast_precision_loss)]
                        let top = virtual_row as f64 * row_height;

                        let col_count = batch.num_columns();

                        // ── Pointer / selection events ──────────────
                        let on_row_down = move |ev: leptos::ev::PointerEvent| {
                            ev.prevent_default();
                            let total = total_rows.get();
                            selection.update(|s| {
                                s.on_pointer_down(
                                    virtual_row,
                                    ev.ctrl_key() || ev.meta_key(),
                                    ev.shift_key(),
                                    total,
                                );
                            });
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
                                    view! {
                                        <div class="dg-row-num" style:width=format!("{ROW_NUM_WIDTH_PX}px")>
                                            {format_row_number(row_num)}
                                        </div>
                                    }
                                })}
                                {(0..col_count).map(|col_idx| {
                                    let w = col_widths.with_untracked(|cw| cw.width(col_idx));
                                    let value = render_cell(&batch, col_idx, local_idx);
                                    view! {
                                        <div
                                            class="dg-cell"
                                            style:width=format!("{w}px")
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
                                clipboard::copy_to_clipboard(&tsv);
                            }
                        }
                        ContextAction::SelectAll => {
                            selection.update(|s| s.select_all(total_rows.get()));
                        }
                    }
                })
                on_close=Callback::new(move |()| menu_position.set(None))
                selected_count=Signal::derive(move || selection.with(super::selection::SelectionState::count))
            />
        </div>
    }
}
