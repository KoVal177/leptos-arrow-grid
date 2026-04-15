//! Grid header row — sort arrows, resize handles, kebab menus.

use leptos::prelude::*;

use crate::col_menu::ColMenu;
use crate::column_state::ColumnWidths;
use crate::types::{
    FilterKind, MIN_COL_WIDTH_PX, MenuItem, ROW_NUM_WIDTH_PX, SortDirection, SortState,
    cycle_sort_multi,
};

/// Data for a single header cell (schema-derived, no sort state).
#[derive(Clone)]
pub struct HeaderCellData {
    /// Column index.
    pub idx: usize,
    /// Column name.
    pub name: String,
}

/// The header row: sort arrows, kebab menus, resize handles.
#[allow(unreachable_pub, clippy::too_many_lines)]
#[component]
pub fn HeaderRow(
    /// Schema-derived column metadata.
    header_items: Signal<Vec<HeaderCellData>>,
    /// Visible column range `(first_col, count)` from column virtualization.
    visible_cols: Signal<(usize, usize)>,
    /// Per-column widths.
    col_widths: RwSignal<ColumnWidths>,
    /// Active drag state.
    drag: RwSignal<Option<(usize, f64, f64)>>,
    /// Show row number gutter.
    show_row_numbers: bool,
    /// Current sort state (read reactively).
    sort: Signal<SortState>,
    /// Active filters per column.
    filters: Signal<Vec<Option<FilterKind>>>,
    /// Sort-change callback — emits the full priority-ordered sort list.
    /// Empty vec = reset to natural order.
    on_sort_change: Callback<Vec<(usize, String, SortDirection)>>,
    /// Filter-change callback.
    on_filter_change: Callback<(usize, String, Option<FilterKind>)>,
    /// Builder for extra menu items per column.
    extra_menu_items: Option<Callback<usize, Vec<MenuItem>>>,
) -> impl IntoView {
    let open_menu: RwSignal<Option<(usize, f64, f64)>> = RwSignal::new(None);

    // Internal building state — set on sort click, cleared when sort signal changes.
    let building_cols: RwSignal<Vec<usize>> = RwSignal::new(Vec::new());
    Effect::new(move |_| {
        let _ = sort.get(); // subscribe
        building_cols.set(Vec::new());
    });

    let gutter_w = if show_row_numbers {
        ROW_NUM_WIDTH_PX
    } else {
        0.0
    };
    let total_width = move || {
        let data_w = col_widths.with(super::column_state::ColumnWidths::total_width);
        gutter_w + data_w
    };

    view! {
        <div class="dg-header-row" style:width=move || format!("{}px", total_width())>
            {show_row_numbers.then(|| view! {
                <div class="dg-row-num-header"
                     style:width=format!("{ROW_NUM_WIDTH_PX}px")
                >
                    "#"
                </div>
            })}
            {move || {
                let items = header_items.get();
                let (first_col, col_count) = visible_cols.get();
                let last_col = (first_col + col_count).min(items.len());
                (first_col..last_col).map(|col_idx| {
                    let hc = items[col_idx].clone();
                    let idx = hc.idx;
                    let name = hc.name;
                    let col_w = Signal::derive(move || col_widths.with(|cw| cw.width(idx)));

                    // ── Reactive sort indicators ────────────
                    let is_sorted = move || sort.with(|s| s.active.iter().any(|(i, _)| *i == idx));
                    let building = move || building_cols.get().contains(&idx);
                    let sort_arrow = move || -> String {
                        if building_cols.get().contains(&idx) {
                            return "\u{23f3}".to_owned();
                        }
                        sort.with(|s| {
                            let pos = s.active.iter().position(|(i, _)| *i == idx);
                            if let Some(p) = pos {
                                let dir = s.active[p].1;
                                if s.active.len() == 1 {
                                    dir.arrow().to_owned()
                                } else {
                                    format!("{}{}", p + 1, dir.arrow())
                                }
                            } else {
                                "\u{00a0}".to_owned()
                            }
                        })
                    };

                    let has_filter = move || {
                        filters.with(|f| f.get(idx).and_then(|o| o.as_ref()).is_some())
                    };

                    // ── Sort click on label ─────────────────
                    let on_label_click = move |ev: leptos::ev::MouseEvent| {
                        let additive = ev.shift_key();
                        let new_sorts = cycle_sort_multi(&sort.get_untracked(), idx, additive);
                        let is_sorted_now = new_sorts.iter().any(|(i, _)| *i == idx);
                        building_cols.update(|v| {
                            if is_sorted_now {
                                if !v.contains(&idx) { v.push(idx); }
                            } else {
                                v.retain(|i| *i != idx);
                            }
                        });
                        let items = header_items.get_untracked();
                        let named: Vec<(usize, String, SortDirection)> = new_sorts
                            .into_iter()
                            .map(|(ci, dir)| {
                                let name = items.get(ci).map(|h| h.name.clone()).unwrap_or_default();
                                (ci, name, dir)
                            })
                            .collect();
                        on_sort_change.run(named);
                    };

                    // ── Kebab click ─────────────────────────
                    let on_kebab = move |ev: leptos::ev::MouseEvent| {
                        ev.stop_propagation();
                        let (x, y) = menu_anchor(&ev);
                        open_menu.update(|m| {
                            *m = if m.map(|(ci, _, _)| ci) == Some(idx) {
                                None
                            } else {
                                Some((idx, x, y))
                            };
                        });
                    };

                    // ── Resize handle events ────────────────
                    let on_handle_down = move |ev: leptos::ev::PointerEvent| {
                        ev.prevent_default();
                        ev.stop_propagation();
                        let start_w = col_widths.with_untracked(|cw| cw.width(idx));
                        drag.set(Some((idx, f64::from(ev.client_x()), start_w)));
                        #[cfg(target_arch = "wasm32")]
                        {
                            use wasm_bindgen::JsCast;
                            if let Some(t) = ev.target() {
                                let _ = t
                                    .unchecked_into::<web_sys::Element>()
                                    .set_pointer_capture(ev.pointer_id());
                            }
                        }
                    };
                    let on_handle_move = move |ev: leptos::ev::PointerEvent| {
                        if let Some((ci, sx, sw)) = drag.get_untracked()
                            && ci == idx {
                                let new_w =
                                    (sw + f64::from(ev.client_x()) - sx).max(MIN_COL_WIDTH_PX);
                                col_widths.update(|cw| cw.set_width(ci, new_w));
                            }
                    };
                    let on_handle_up = move |_: leptos::ev::PointerEvent| drag.set(None);

                    // ── Menu state for this column ──────────
                    let menu_open = Signal::derive(move || {
                        open_menu.get().filter(|(ci, _, _)| *ci == idx)
                    });

                    let name_menu = name.clone();
                    let extra = extra_menu_items;

                    let left = col_widths.with_untracked(|cw| cw.left_offset(idx)) + gutter_w;

                    view! {
                        <div
                            class="dg-header-cell"
                            class:dg-header-cell--sorted=is_sorted
                            class:dg-header-cell--building=building
                            class:dg-header-cell--filtered=has_filter
                            style:position="absolute"
                            style:left=format!("{left}px")
                            style:width=move || format!("{}px", col_w.get())
                        >
                            <span class="dg-sort-arrow">{sort_arrow}</span>
                            <button class="dg-header-label" on:click=on_label_click>
                                {name}
                            </button>
                            <button class="dg-kebab" on:click=on_kebab title="Column menu">
                                "\u{22ee}"
                            </button>
                            <div
                                class="dg-resize-handle"
                                on:pointerdown=on_handle_down
                                on:pointermove=on_handle_move
                                on:pointerup=on_handle_up
                                on:pointercancel=on_handle_up
                            />
                        </div>
                        {move || menu_open.get().map(|(_, x, y)| {
                            let sort_snap = sort.get_untracked();
                            let name_m = name_menu.clone();
                            let items = extra.map(|cb| cb.run(idx)).unwrap_or_default();
                            view! {
                                <ColMenu
                                    col_idx=idx
                                    col_name=name_m
                                    x=x
                                    y=y
                                    sort_state=sort_snap
                                    current_filter=Signal::derive(move || {
                                        filters.with(|f| f.get(idx).cloned().flatten())
                                    })
                                    on_sort_change=on_sort_change
                                    on_filter_change=on_filter_change
                                    on_close=Callback::new(move |()| open_menu.set(None))
                                    extra_items=items
                                />
                            }
                        })}
                    }
                }).collect_view()
            }}
        </div>
    }
}

/// Get anchor position for menu placement (below the kebab button).
fn menu_anchor(ev: &leptos::ev::MouseEvent) -> (f64, f64) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        ev.target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .map_or((0.0, 0.0), |el| {
                let r = el.get_bounding_client_rect();
                (r.left(), r.bottom())
            })
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = ev;
        (0.0, 0.0)
    }
}
