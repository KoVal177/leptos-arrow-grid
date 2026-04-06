//! Download selected rows as a CSV file in the browser.

use std::collections::HashSet;
use std::hash::BuildHasher;

use arrow_schema::SchemaRef;

use crate::cell::render_cell;
use crate::types::GridPage;

/// Build RFC 4180 CSV text for the selected rows.
///
/// If `selected` is empty, all rows in the current page are exported.
/// Rows not present in the current page are skipped.
pub fn build_csv<S: BuildHasher>(
    selected: &HashSet<u64, S>,
    schema: &SchemaRef,
    page: &Option<GridPage>,
) -> String {
    let col_count = schema.fields().len();

    // Header row
    let header: Vec<String> = schema.fields().iter().map(|f| csv_quote(f.name())).collect();
    let mut csv = header.join(",");
    csv.push('\n');

    let Some(p) = page else {
        return csv;
    };

    // Determine which rows to include.
    let end = p.start + p.row_count as u64;
    let mut rows: Vec<u64> = if selected.is_empty() {
        (p.start..end).collect()
    } else {
        let mut v: Vec<u64> =
            selected.iter().copied().filter(|&r| r >= p.start && r < end).collect();
        v.sort_unstable();
        v
    };
    rows.dedup();

    for abs_row in rows {
        #[allow(clippy::cast_possible_truncation)]
        let local_idx = (abs_row - p.start) as usize;
        let row: Vec<String> =
            (0..col_count).map(|col| csv_quote(&render_cell(&p.batch, col, local_idx))).collect();
        csv.push_str(&row.join(","));
        csv.push('\n');
    }

    csv
}

/// Wrap `s` in double-quotes if it contains a comma, newline, or double-quote,
/// doubling any inner quotes per RFC 4180.
fn csv_quote(s: &str) -> String {
    if s.contains([',', '\n', '\r', '"']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_owned()
    }
}

/// Trigger a browser file download of `content` as a `.csv` file.
///
/// Uses the Blob → object-URL → hidden `<a>` pattern.
/// The filename is generated from the current local timestamp.
#[cfg(target_arch = "wasm32")]
pub fn download_csv_file(content: &str) {
    use wasm_bindgen::JsCast;

    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };

    // Build Blob
    let array = js_sys::Array::new();
    array.push(&wasm_bindgen::JsValue::from_str(content));
    let options = web_sys::BlobPropertyBag::new();
    options.set_type("text/csv;charset=utf-8;");
    let Ok(blob) = web_sys::Blob::new_with_str_sequence_and_options(&array, &options) else {        return;
    };

    // Create object URL
    let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else {
        return;
    };

    // Create and click a hidden anchor
    let Ok(el) = document.create_element("a") else {
        return;
    };
    let Ok(anchor) = el.dyn_into::<web_sys::HtmlAnchorElement>() else {
        return;
    };
    anchor.set_href(&url);
    anchor.set_download(&download_filename());

    if let Some(body) = document.body() {
        let _ = body.append_child(&anchor);
        anchor.click();
        let _ = body.remove_child(&anchor);
    }

    let _ = web_sys::Url::revoke_object_url(&url);
}

/// No-op on non-WASM targets (e.g. during unit tests).
#[cfg(not(target_arch = "wasm32"))]
pub fn download_csv_file(_content: &str) {}

/// Build a filename of the form `lab-download-YYYY-MM-DD-HH-MM-SS.csv` using
/// the current local time from `js_sys::Date`.
#[cfg(target_arch = "wasm32")]
fn download_filename() -> String {
    let d = js_sys::Date::new_0();
    format!(
        "lab-download-{:04}-{:02}-{:02}-{:02}-{:02}-{:02}.csv",
        d.get_full_year(),
        d.get_month() + 1, // JS months are 0-indexed
        d.get_date(),
        d.get_hours(),
        d.get_minutes(),
        d.get_seconds(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::sync::Arc;

    use arrow_array::{Int64Array, RecordBatch, StringArray};
    use arrow_schema::{DataType, Field, Schema};

    fn make_page() -> (SchemaRef, GridPage) {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::Utf8, true),
        ]));
        let batch = RecordBatch::try_new(
            Arc::clone(&schema),
            vec![
                Arc::new(Int64Array::from(vec![1, 2, 3])),
                Arc::new(StringArray::from(vec!["Alice", "Bob, Jr.", "Carol \"C\""])),
            ],
        )
        .unwrap();
        let page = GridPage { start: 0, row_count: 3, batch: Arc::new(batch) };
        (schema, page)
    }

    #[test]
    fn all_rows_when_empty_selection() {
        let (schema, page) = make_page();
        let csv = build_csv(&HashSet::<u64>::new(), &schema, &Some(page));
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 4); // header + 3 data rows
        assert_eq!(lines[0], "id,name");
        assert_eq!(lines[1], "1,Alice");
        assert_eq!(lines[2], r#"2,"Bob, Jr.""#);
        assert_eq!(lines[3], r#"3,"Carol ""C""""#);
    }

    #[test]
    fn selected_rows_only() {
        let (schema, page) = make_page();
        let selected: HashSet<u64> = [0u64, 2].into_iter().collect();
        let csv = build_csv(&selected, &schema, &Some(page));
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 3); // header + 2 data rows
        assert_eq!(lines[1], "1,Alice");
        assert_eq!(lines[2], r#"3,"Carol ""C""""#);
    }

    #[test]
    fn empty_page_returns_header_only() {
        let (schema, _) = make_page();
        let csv = build_csv(&HashSet::<u64>::new(), &schema, &None);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "id,name");
    }

    #[test]
    fn csv_quote_plain() {
        assert_eq!(csv_quote("hello"), "hello");
    }

    #[test]
    fn csv_quote_with_comma() {
        assert_eq!(csv_quote("a,b"), r#""a,b""#);
    }

    #[test]
    fn csv_quote_with_double_quote() {
        assert_eq!(csv_quote(r#"say "hi""#), r#""say ""hi""""#);
    }
}
