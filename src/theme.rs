//! Theme components for `leptos-arrow-grid`.
//!
//! # Usage
//!
//! ```rust,ignore
//! use leptos_arrow_grid::{ArrowGridStyles, ArrowGridThemeScope, ArrowGridTheme, DataGrid};
//!
//! view! {
//!     <ArrowGridStyles />
//!     <ArrowGridThemeScope theme=ArrowGridTheme::Dark>
//!         <div style="height: 400px;">
//!             <DataGrid ... />
//!         </div>
//!     </ArrowGridThemeScope>
//! }
//! ```

use leptos::prelude::*;

/// Theme variant for `leptos-arrow-grid`.
///
/// `Light` is the default when no explicit scope is applied.
/// `Dark` activates the Catppuccin Mocha–inspired dark palette.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ArrowGridTheme {
    /// Light theme — clean neutral palette, default via `:root`.
    #[default]
    Light,
    /// Dark theme — Catppuccin Mocha–inspired palette.
    Dark,
}

impl ArrowGridTheme {
    /// CSS class name for this theme variant.
    pub fn class(self) -> &'static str {
        match self {
            Self::Light => "lag-theme-light",
            Self::Dark => "lag-theme-dark",
        }
    }
}

/// Injects the base grid stylesheet and theme token stylesheet.
///
/// Place once near the top of your component tree.
/// `DataGrid` does **not** auto-inject styles — this component is required.
#[component]
pub fn ArrowGridStyles() -> impl IntoView {
    view! {
        <style id="lag-grid-base">
            {include_str!("../style/grid.css")}
        </style>
        <style id="lag-grid-themes">
            {include_str!("../style/lag-themes.css")}
        </style>
    }
}

/// Wraps children in a themed `<div>` that sets the correct `lag-theme-*` class.
///
/// Without this wrapper, the default light theme from `:root` is active.
/// Use this to scope a subtree to a specific theme.
#[component]
pub fn ArrowGridThemeScope(
    /// Theme variant — defaults to `Light`.
    #[prop(into, default = ArrowGridTheme::Light.into())]
    theme: Signal<ArrowGridTheme>,
    children: Children,
) -> impl IntoView {
    let class = move || theme.get().class();
    view! {
        <div class=class>
            {children()}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_is_default() {
        assert_eq!(ArrowGridTheme::default(), ArrowGridTheme::Light);
    }

    #[test]
    fn class_names() {
        assert_eq!(ArrowGridTheme::Light.class(), "lag-theme-light");
        assert_eq!(ArrowGridTheme::Dark.class(), "lag-theme-dark");
    }

    #[test]
    fn theme_is_copy() {
        let t = ArrowGridTheme::Dark;
        let t2 = t;
        assert_eq!(t, t2);
    }
}
