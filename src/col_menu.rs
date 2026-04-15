//! Column context menu — sort, filter, and custom actions.

use leptos::prelude::*;

use crate::types::{FilterKind, FilterMode, MenuItem, SortDirection, SortState, build_filter};

/// Positioned column menu with sort, filter, and extra items.
#[allow(
    unreachable_pub,
    clippy::too_many_lines,
    clippy::needless_pass_by_value
)]
#[component]
pub fn ColMenu(
    /// Column index.
    col_idx: usize,
    /// Column name.
    col_name: String,
    /// Menu X coordinate (fixed position).
    x: f64,
    /// Menu Y coordinate (fixed position).
    y: f64,
    /// Snapshot of current sort state.
    sort_state: SortState,
    /// Current filter for this column.
    current_filter: Signal<Option<FilterKind>>,
    /// Sort-change callback — emits the full priority-ordered sort list.
    /// Empty vec = reset to natural order.
    on_sort_change: Callback<Vec<(usize, String, SortDirection)>>,
    /// Filter-change callback.
    on_filter_change: Callback<(usize, String, Option<FilterKind>)>,
    /// Close the menu.
    on_close: Callback<()>,
    /// Extra menu items from the consumer (headless slot).
    extra_items: Vec<MenuItem>,
) -> impl IntoView {
    let editing: RwSignal<Option<FilterMode>> = RwSignal::new(None);
    let filter_input: RwSignal<String> = RwSignal::new(String::new());

    let current_sort_dir = sort_state
        .active
        .iter()
        .find(|(ci, _)| *ci == col_idx)
        .map(|(_, d)| *d);

    // ── Sort handlers ───────────────────────────────────────
    let name_asc = col_name.clone();
    let set_sort_asc = move |_| {
        on_sort_change.run(vec![(col_idx, name_asc.clone(), SortDirection::Asc)]);
        on_close.run(());
    };
    let name_desc = col_name.clone();
    let set_sort_desc = move |_| {
        on_sort_change.run(vec![(col_idx, name_desc.clone(), SortDirection::Desc)]);
        on_close.run(());
    };
    let clear_sort = move |_| {
        on_sort_change.run(vec![]);
        on_close.run(());
    };

    let has_filter = Signal::derive(move || current_filter.get().is_some());

    // Names pre-cloned for use inside reactive blocks.
    let col_name_for_enter = col_name.clone();
    let col_name_for_btn = col_name.clone();
    let col_name_for_clear = col_name;

    view! {
        // ── Backdrop ────────────────────────────────────
        <div
            class="dg-menu-backdrop"
            on:click=move |_| on_close.run(())
            on:contextmenu=move |ev: leptos::ev::MouseEvent| {
                ev.prevent_default();
                on_close.run(());
            }
        />
        // ── Menu popup ──────────────────────────────────
        <div
            class="dg-col-menu"
            style=format!("left:{x}px;top:{y}px;")
            on:click=|ev: leptos::ev::MouseEvent| ev.stop_propagation()
        >
            // ── Custom items (headless slot) ────────────
            {(!extra_items.is_empty()).then(|| {
                let items = extra_items.clone();
                view! {
                    {items.into_iter().map(|item| {
                        let cb = item.on_click;
                        view! {
                            <button
                                class="dg-menu-item"
                                disabled=item.disabled
                                on:click=move |_| {
                                    cb.run(());
                                    on_close.run(());
                                }
                            >
                                {item.label.clone()}
                            </button>
                        }
                    }).collect::<Vec<_>>()}
                    <hr class="dg-menu-sep" />
                }
            })}

            // ── Sort section ────────────────────────────
            <button
                class="dg-menu-item"
                class:dg-menu-item--active=current_sort_dir == Some(SortDirection::Asc)
                on:click=set_sort_asc
            >
                "\u{2191}  Sort ascending"
            </button>
            <button
                class="dg-menu-item"
                class:dg-menu-item--active=current_sort_dir == Some(SortDirection::Desc)
                on:click=set_sort_desc
            >
                "\u{2193}  Sort descending"
            </button>
            <button class="dg-menu-item" on:click=clear_sort>
                "\u{2715}  Clear sort"
            </button>
            <hr class="dg-menu-sep" />

            // ── Filter section ──────────────────────────
            {move || {
                if let Some(mode) = editing.get() {
                    let name_e = col_name_for_enter.clone();
                    let name_b = col_name_for_btn.clone();
                    view! {
                        <div class="dg-filter-form">
                            <div class="dg-filter-label">
                                {format!("Filter \u{2014} {}", mode.label())}
                            </div>
                            <input
                                class="dg-filter-input"
                                type="text"
                                placeholder="value\u{2026}"
                                prop:value=move || filter_input.get()
                                on:input=move |ev| filter_input.set(event_target_value(&ev))
                                on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                                    if ev.key() == "Enter"
                                        && let Some(m) = editing.get_untracked() {
                                            let filter = build_filter(m, filter_input.get_untracked());
                                            on_filter_change.run((col_idx, name_e.clone(), Some(filter)));
                                            on_close.run(());
                                        }
                                    if ev.key() == "Escape" {
                                        editing.set(None);
                                    }
                                }
                            />
                            <div class="dg-filter-btns">
                                <button
                                    class="dg-filter-apply"
                                    on:click=move |_| {
                                        if let Some(m) = editing.get_untracked() {
                                            let filter = build_filter(m, filter_input.get_untracked());
                                            on_filter_change.run((col_idx, name_b.clone(), Some(filter)));
                                            on_close.run(());
                                        }
                                    }
                                >
                                    "Apply"
                                </button>
                                <button class="dg-filter-cancel" on:click=move |_| editing.set(None)>
                                    "Cancel"
                                </button>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    let name_c = col_name_for_clear.clone();
                    view! {
                        <div>
                            <button class="dg-menu-item"
                                on:click=move |_| editing.set(Some(FilterMode::Contains))
                            >
                                "\u{25b7}  Filter: contains\u{2026}"
                            </button>
                            <button class="dg-menu-item"
                                on:click=move |_| editing.set(Some(FilterMode::StartsWith))
                            >
                                "\u{25b7}  Filter: starts with\u{2026}"
                            </button>
                            <button class="dg-menu-item"
                                on:click=move |_| editing.set(Some(FilterMode::Regex))
                            >
                                "\u{25b7}  Filter: regex\u{2026}"
                            </button>
                            {has_filter.get().then(|| {
                                view! {
                                    <hr class="dg-menu-sep" />
                                    <button
                                        class="dg-menu-item dg-menu-item--danger"
                                        on:click=move |_| {
                                            on_filter_change.run((col_idx, name_c.clone(), None));
                                            on_close.run(());
                                        }
                                    >
                                        "\u{2715}  Clear filter"
                                    </button>
                                }
                            })}
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}
