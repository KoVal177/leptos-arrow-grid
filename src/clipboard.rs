//! Copy selected rows to clipboard as tab-separated values.

use std::collections::HashSet;
use std::hash::BuildHasher;

use arrow_schema::SchemaRef;

use crate::cell::render_cell;
use crate::types::GridPage;

/// Build TSV text for the selected rows.
///
/// Reads directly from Arrow memory — no intermediate DOM access.
/// Rows not in the current page are skipped (they aren't loaded).
pub fn build_tsv<S: BuildHasher>(
    selected: &HashSet<u64, S>,
    schema: &SchemaRef,
    page: &Option<GridPage>,
) -> String {
    if selected.is_empty() {
        return String::new();
    }

    let mut sorted: Vec<u64> = selected.iter().copied().collect();
    sorted.sort_unstable();

    let col_count = schema.fields().len();

    // Header row
    let mut tsv = schema
        .fields()
        .iter()
        .map(|f| f.name().as_str())
        .collect::<Vec<_>>()
        .join("\t");
    tsv.push('\n');

    // Data rows
    if let Some(p) = page {
        for &abs_row in &sorted {
            #[allow(clippy::cast_possible_truncation)]
            let end = p.start + p.row_count as u64;
            if abs_row >= p.start && abs_row < end {
                #[allow(clippy::cast_possible_truncation)]
                let local_idx = (abs_row - p.start) as usize;
                for col in 0..col_count {
                    if col > 0 {
                        tsv.push('\t');
                    }
                    tsv.push_str(&render_cell(&p.batch, col, local_idx));
                }
                tsv.push('\n');
            }
        }
    }

    tsv
}

/// Copy TSV text to clipboard using the web Clipboard API.
///
/// Spawns an async task and returns immediately. `on_error` is called with a
/// human-readable message if the write fails. Common failure reasons:
///
/// - **Not a secure context**: The Clipboard API requires HTTPS or `localhost`.
///   Plain HTTP pages will receive a `NotAllowedError`.
/// - **Permission denied**: The browser may prompt the user; if they decline,
///   or if the page lacks the `clipboard-write` permission policy, the write
///   is rejected.
/// - **Focus lost**: Some browsers reject clipboard writes if the document
///   loses focus between the keydown event and the async write completing.
///
/// See [TROUBLESHOOTING.md](../TROUBLESHOOTING.md) for details and workarounds.
#[cfg(target_arch = "wasm32")]
pub fn copy_to_clipboard(text: &str, on_error: Option<leptos::prelude::Callback<String>>) {
    use leptos::prelude::Callable;
    let text = text.to_owned();
    wasm_bindgen_futures::spawn_local(async move {
        let Some(window) = web_sys::window() else {
            return;
        };
        let clipboard = window.navigator().clipboard();
        let promise = clipboard.write_text(&text);
        if let Err(err) = wasm_bindgen_futures::JsFuture::from(promise).await {
            let msg = err.as_string().unwrap_or_else(|| format!("{err:?}"));
            if let Some(cb) = on_error {
                cb.run(msg);
            }
        }
    });
}

/// No-op on non-WASM targets.
#[cfg(not(target_arch = "wasm32"))]
pub fn copy_to_clipboard(_text: &str, _on_error: Option<leptos::prelude::Callback<String>>) {}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow_array::{Int64Array, RecordBatch, StringArray};
    use arrow_schema::{DataType, Field, Schema};

    use super::*;

    fn test_page() -> GridPage {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::Utf8, false),
        ]));
        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(Int64Array::from(vec![1, 2, 3])),
                Arc::new(StringArray::from(vec!["alice", "bob", "carol"])),
            ],
        )
        .expect("test batch");
        GridPage {
            start: 0,
            row_count: 3,
            batch: Arc::new(batch),
        }
    }

    #[test]
    fn empty_selection() {
        let schema = Arc::new(Schema::new(Vec::<Field>::new()));
        assert_eq!(build_tsv(&HashSet::new(), &schema, &None), "");
    }

    #[test]
    fn single_row() {
        let page = test_page();
        let schema = page.batch.schema();
        let mut selected = HashSet::new();
        selected.insert(1);
        let tsv = build_tsv(&selected, &schema, &Some(page));
        assert_eq!(tsv, "id\tname\n2\tbob\n");
    }

    #[test]
    fn multiple_rows() {
        let page = test_page();
        let schema = page.batch.schema();
        let mut selected = HashSet::new();
        selected.insert(0);
        selected.insert(2);
        let tsv = build_tsv(&selected, &schema, &Some(page));
        assert_eq!(tsv, "id\tname\n1\talice\n3\tcarol\n");
    }
}
