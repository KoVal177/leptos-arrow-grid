//! Context menu component for the data grid.

use leptos::prelude::*;

/// Actions available from the context menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextAction {
    /// Copy selected rows as TSV.
    Copy,
    /// Select all rows.
    SelectAll,
}

/// Placement coordinates for the context menu.
#[derive(Clone, Copy, Debug)]
pub struct MenuPosition {
    /// Page X coordinate.
    pub x: f64,
    /// Page Y coordinate.
    pub y: f64,
}

/// Context menu component rendered inside the grid container.
///
/// Positioned at `(x, y)` in page coordinates. Backdrop covers
/// the full viewport to catch dismiss clicks.
#[component]
pub fn GridContextMenu(
    /// Position of the menu (None = hidden).
    position: Signal<Option<MenuPosition>>,
    /// Callback when an action is selected.
    on_action: Callback<ContextAction>,
    /// Callback to close the menu.
    on_close: Callback<()>,
    /// Number of currently selected rows.
    selected_count: Signal<usize>,
) -> impl IntoView {
    let copy_label = Signal::derive(move || {
        let count = selected_count.get();
        if count > 0 {
            format!("Copy ({count} rows)")
        } else {
            "Copy".to_string()
        }
    });

    view! {
        <Show when=move || position.get().is_some()>
            {move || {
                let pos = position.get().expect("checked in Show");
                view! {
                    <div
                        class="dg-context-backdrop"
                        on:pointerdown=move |_| on_close.run(())
                        on:contextmenu=move |ev| {
                            ev.prevent_default();
                            on_close.run(());
                        }
                    />
                    <div
                        class="dg-context-menu"
                        style:left=format!("{}px", pos.x)
                        style:top=format!("{}px", pos.y)
                    >
                        <button
                            class="dg-context-item"
                            on:click=move |_| {
                                on_action.run(ContextAction::Copy);
                                on_close.run(());
                            }
                        >
                            {copy_label}
                            <span class="dg-context-shortcut">{"Ctrl+C"}</span>
                        </button>
                        <button
                            class="dg-context-item"
                            on:click=move |_| {
                                on_action.run(ContextAction::SelectAll);
                                on_close.run(());
                            }
                        >
                            "Select All"
                            <span class="dg-context-shortcut">{"Ctrl+A"}</span>
                        </button>
                    </div>
                }
            }}
        </Show>
    }
}
