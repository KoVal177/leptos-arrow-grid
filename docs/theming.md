# Theming

leptos-arrow-grid ships with a [Catppuccin Mocha](https://github.com/catppuccin/catppuccin) dark theme as defaults. Every colour, font, and spacing value is exposed as a CSS custom property so you can swap the whole design by overriding a handful of variables.

## Variable Reference

| Variable | Default | Purpose |
|---|---|---|
| `--lag-font-mono` | `monospace` | Cell and header font family |
| `--lag-font-size-base` | `13px` | Body text size |
| `--lag-font-size-small` | `11px` | Header labels, row numbers |
| `--lag-bg-primary` | `#1e1e2e` | Grid body background |
| `--lag-bg-secondary` | `#181825` | Row-number gutter, context menu body |
| `--lag-bg-surface` | `#313244` | Header row, hover highlight |
| `--lag-text-primary` | `#cdd6f4` | Cell text |
| `--lag-text-secondary` | `#a6adc8` | Header labels |
| `--lag-text-muted` | `#6c7086` | Row numbers, kebab icon |
| `--lag-border` | `#45475a` | Column dividers, menu borders |
| `--lag-accent` | `#89b4fa` | Selected rows, sort arrow, focus ring |
| `--lag-warning` | `#f9e2af` | Sort-building indicator glow |
| `--lag-error` | `#f38ba8` | Error states |
| `--lag-transition-fast` | `100ms ease` | Hover/focus transitions |
| `--lag-grid-header-height` | `32px` | Height of the sticky header row |
| `--lag-grid-cell-padding` | `4px 8px` | Inner padding for each data cell |

## Light Mode Override

Add this to your global stylesheet (or inside a `.light-theme` class):

```css
:root {
  --lag-font-mono: "Inter", ui-sans-serif, sans-serif;
  --lag-font-size-base: 13px;
  --lag-font-size-small: 11px;

  --lag-bg-primary:    #ffffff;
  --lag-bg-secondary:  #f5f5f5;
  --lag-bg-surface:    #ebebeb;

  --lag-text-primary:   #1a1a1a;
  --lag-text-secondary: #555555;
  --lag-text-muted:     #999999;

  --lag-border:  #d4d4d8;
  --lag-accent:  #2563eb;
  --lag-warning: #d97706;
  --lag-error:   #dc2626;

  --lag-transition-fast: 80ms ease;
}
```

## Scoped Theme (single component)

If multiple grids on the same page need different themes, scope the variables to the container:

```css
.my-report-grid {
  --lag-bg-primary:  #0f172a;
  --lag-accent:      #38bdf8;
  --lag-border:      #1e3a5f;
}
```

```rust
view! {
    <div class="my-report-grid" style="height: 600px;">
        <DataGrid ... />
    </div>
}
```

## Row Height

Row height is a Leptos prop, not a CSS variable, because the grid uses it for virtual scroll offset arithmetic:

```rust
<DataGrid row_height=32.0 ... />
```

The default is **28 px**. Set the same value in CSS if you rely on `--lag-grid-cell-padding` giving a full-height cell:

```css
.dg-row { height: 32px; }  /* keep in sync with row_height prop */
```

## Custom Font

The grid inherits `--lag-font-mono` for all cell text. Pair it with a web font loaded in your HTML:

```html
<link rel="preconnect" href="https://fonts.googleapis.com">
<link href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500&display=swap" rel="stylesheet">
```

```css
:root {
  --lag-font-mono: "JetBrains Mono", monospace;
}
```
