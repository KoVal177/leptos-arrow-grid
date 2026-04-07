# Theming

leptos-arrow-grid ships with both **light** and **dark** themes built in.
Light is the default. No CSS overrides are needed for a clean out-of-the-box experience.

## Quick Start

```rust
use leptos_arrow_grid::{ArrowGridStyles, DataGrid};

view! {
    <ArrowGridStyles />
    <div style="height: 400px;">
        <DataGrid ... />
    </div>
}
```

`ArrowGridStyles` injects both the structural stylesheet and the theme tokens.
The light theme is active by default through `:root`.

## Dark Theme

Wrap your grid (or any subtree) in `ArrowGridThemeScope`:

```rust
use leptos_arrow_grid::{ArrowGridStyles, ArrowGridTheme, ArrowGridThemeScope, DataGrid};

view! {
    <ArrowGridStyles />
    <ArrowGridThemeScope theme=ArrowGridTheme::Dark>
        <div style="height: 400px;">
            <DataGrid ... />
        </div>
    </ArrowGridThemeScope>
}
```

## Reactive Theme Toggle

Bind the theme to a signal for user-controlled switching:

```rust
let dark_mode = RwSignal::new(false);
let theme = Signal::derive(move || {
    if dark_mode.get() { ArrowGridTheme::Dark } else { ArrowGridTheme::Light }
});

view! {
    <ArrowGridStyles />
    <ArrowGridThemeScope theme=theme>
        <div style="height: 400px;">
            <DataGrid ... />
        </div>
    </ArrowGridThemeScope>
    <button on:click=move |_| dark_mode.update(|d| *d = !*d)>
        "Toggle theme"
    </button>
}
```

## Mixed Themes on One Page

Each `ArrowGridThemeScope` creates an independent theme context:

```rust
view! {
    <ArrowGridStyles />

    <ArrowGridThemeScope theme=ArrowGridTheme::Light>
        <div style="height: 300px;">
            <DataGrid ... />  // light
        </div>
    </ArrowGridThemeScope>

    <ArrowGridThemeScope theme=ArrowGridTheme::Dark>
        <div style="height: 300px;">
            <DataGrid ... />  // dark
        </div>
    </ArrowGridThemeScope>
}
```

## Custom Token Overrides

Both built-in themes define `--lag-*` CSS custom properties.
Override any token on a wrapping element:

```css
.my-report-grid {
    --lag-accent: #10b981;
    --lag-border: #d1fae5;
}
```

```rust
view! {
    <ArrowGridStyles />
    <div class="my-report-grid" style="height: 400px;">
        <DataGrid ... />
    </div>
}
```

Overrides work with both light and dark themes.

## CSS Variable Reference

| Variable | Light | Dark | Purpose |
|---|---|---|---|
| `--lag-font-mono` | System mono stack | System mono stack | Font family |
| `--lag-font-size-base` | 14px | 13px | Body text size |
| `--lag-font-size-small` | 12px | 11px | Header, row numbers |
| `--lag-bg-primary` | `#ffffff` | `#1e1e2e` | Grid body background |
| `--lag-bg-secondary` | `#f9fafb` | `#181825` | Gutter, menus |
| `--lag-bg-surface` | `#f3f4f6` | `#313244` | Header, hover highlight |
| `--lag-text-primary` | `#111827` | `#cdd6f4` | Cell text |
| `--lag-text-secondary` | `#4b5563` | `#a6adc8` | Header labels |
| `--lag-text-muted` | `#9ca3af` | `#6c7086` | Row numbers |
| `--lag-border` | `#e5e7eb` | `#45475a` | Dividers, borders |
| `--lag-accent` | `#2563eb` | `#89b4fa` | Selection, sort, focus |
| `--lag-warning` | `#d97706` | `#f9e2af` | Warning states |
| `--lag-error` | `#dc2626` | `#f38ba8` | Error states |
| `--lag-grid-header-height` | 32px | 32px | Header row height |
| `--lag-grid-cell-padding` | 4px 8px | 4px 8px | Cell padding |
| `--lag-transition-fast` | 100ms ease | 100ms ease | Hover transition |

## Manual CSS Inclusion (Fallback)

For CSP-restricted environments or bundler-level CSS handling,
load the stylesheets manually instead of using `ArrowGridStyles`:

```html
<link rel="stylesheet" href="path/to/leptos-arrow-grid/style/grid.css" />
<link rel="stylesheet" href="path/to/leptos-arrow-grid/style/lag-themes.css" />
```

Then apply themes with CSS classes directly:

```html
<div class="lag-theme-dark">
    <!-- grid renders in dark theme -->
</div>
```

When using manual inclusion, do NOT also use `ArrowGridStyles` — it would
duplicate the stylesheets.

## Row Height

Row height is a Leptos prop, not a CSS variable:

```rust
<DataGrid row_height=32.0 ... />
```

The default is 24 px.
