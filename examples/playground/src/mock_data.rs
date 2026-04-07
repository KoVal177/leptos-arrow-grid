//! In-memory Arrow RecordBatch generator — no network, no files.

use std::sync::Arc;

use arrow_array::{
    builder::{BooleanBuilder, Int64Builder, StringBuilder},
    RecordBatch,
};
use arrow_schema::{DataType, Field, Schema, SchemaRef};

pub const DEPTS: &[&str] = &["Engineering", "Sales", "Finance", "HR", "Legal"];

/// Canonical schema for mock data.
pub fn mock_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("username", DataType::Utf8, false),
        Field::new("department", DataType::Utf8, true),
        Field::new("salary", DataType::Int64, true),
        Field::new("is_active", DataType::Boolean, false),
    ]))
}

/// Generate `count` rows starting from row `offset`.
pub fn generate_mock_batch_range(offset: usize, count: usize) -> Arc<RecordBatch> {
    let schema = mock_schema();

    let mut id_b = Int64Builder::with_capacity(count);
    let mut name_b = StringBuilder::with_capacity(count, count * 10);
    let mut dept_b = StringBuilder::with_capacity(count, count * 8);
    let mut sal_b = Int64Builder::with_capacity(count);
    let mut active_b = BooleanBuilder::with_capacity(count);

    for i in offset..offset + count {
        id_b.append_value(i as i64);
        name_b.append_value(format!("user_{i:07}"));
        if i % 17 == 0 {
            dept_b.append_null();
        } else {
            dept_b.append_value(DEPTS[i % DEPTS.len()]);
        }
        if i % 11 == 0 {
            sal_b.append_null();
        } else {
            sal_b.append_value(50_000 + (i as i64 % 100_000));
        }
        active_b.append_value(i % 3 != 0);
    }

    Arc::new(
        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(id_b.finish()),
                Arc::new(name_b.finish()),
                Arc::new(dept_b.finish()),
                Arc::new(sal_b.finish()),
                Arc::new(active_b.finish()),
            ],
        )
        .expect("mock batch construction is infallible"),
    )
}

/// Generate a batch from an explicit list of (sorted/filtered) row indices.
pub fn generate_mock_batch_from_indices(indices: &[usize]) -> Arc<RecordBatch> {
    let count = indices.len();
    let schema = mock_schema();

    let mut id_b = Int64Builder::with_capacity(count);
    let mut name_b = StringBuilder::with_capacity(count, count * 10);
    let mut dept_b = StringBuilder::with_capacity(count, count * 8);
    let mut sal_b = Int64Builder::with_capacity(count);
    let mut active_b = BooleanBuilder::with_capacity(count);

    for &i in indices {
        id_b.append_value(i as i64);
        name_b.append_value(format!("user_{i:07}"));
        if i % 17 == 0 {
            dept_b.append_null();
        } else {
            dept_b.append_value(DEPTS[i % DEPTS.len()]);
        }
        if i % 11 == 0 {
            sal_b.append_null();
        } else {
            sal_b.append_value(50_000 + (i as i64 % 100_000));
        }
        active_b.append_value(i % 3 != 0);
    }

    Arc::new(
        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(id_b.finish()),
                Arc::new(name_b.finish()),
                Arc::new(dept_b.finish()),
                Arc::new(sal_b.finish()),
                Arc::new(active_b.finish()),
            ],
        )
        .expect("mock batch from indices is infallible"),
    )
}

// ── Per-row string rendering ──────────────────────────────────────────────────

/// Return the string representation of column `col` for dataset row `i`.
///
/// This mirrors the values written by `generate_mock_batch_range` so that
/// filter matching and sort comparison stay in sync with batch generation.
pub fn row_value_str(i: usize, col: usize) -> String {
    match col {
        0 => i.to_string(),
        1 => format!("user_{i:07}"),
        2 => {
            if i % 17 == 0 {
                String::new()
            } else {
                DEPTS[i % DEPTS.len()].to_string()
            }
        }
        3 => {
            if i % 11 == 0 {
                String::new()
            } else {
                (50_000 + i % 100_000).to_string()
            }
        }
        4 => (i % 3 != 0).to_string(),
        _ => String::new(),
    }
}

// ── Filter matching ───────────────────────────────────────────────────────────

/// Return `true` when dataset row `i`, column `col` satisfies `filter`.
///
/// Matching is case-insensitive. `FilterKind::Regex` falls back to substring
/// matching because the playground ships no regex engine.
pub fn row_matches_filter(i: usize, col: usize, filter: &leptos_arrow_grid::FilterKind) -> bool {
    use leptos_arrow_grid::FilterKind;
    let val = row_value_str(i, col).to_lowercase();
    match filter {
        FilterKind::Contains(s) => val.contains(&s.to_lowercase()),
        FilterKind::StartsWith(s) => val.starts_with(&s.to_lowercase()),
        FilterKind::Regex(s) => val.contains(&s.to_lowercase()), // substring fallback
    }
}

// ── Sort comparison ───────────────────────────────────────────────────────────

/// Compare dataset rows `a` and `b` for sort column `col` (ascending order).
///
/// Null rows (encoded as empty strings) sort *after* non-null rows.
/// The caller reverses the result for descending order.
pub fn compare_rows(a: usize, b: usize, col: usize) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match col {
        // id (col 0) and username (col 1) order mirrors the row index.
        0 | 1 => a.cmp(&b),
        2 => row_value_str(a, 2).cmp(&row_value_str(b, 2)),
        3 => {
            // Null rows (i % 11 == 0) sort last in ascending order.
            let va = (a % 11 != 0).then(|| 50_000 + a % 100_000);
            let vb = (b % 11 != 0).then(|| 50_000 + b % 100_000);
            match (va, vb) {
                (None, None) => a.cmp(&b),
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (Some(x), Some(y)) => x.cmp(&y),
            }
        }
        4 => {
            // true (active) sorts before false (inactive) in ascending.
            i32::from(b % 3 != 0).cmp(&i32::from(a % 3 != 0))
        }
        _ => std::cmp::Ordering::Equal,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod data_logic_tests {
    use super::*;
    use leptos_arrow_grid::FilterKind;

    // row_value_str ────────────────────────────────────────────────────────────

    #[test]
    fn row_value_str_id() {
        assert_eq!(row_value_str(42, 0), "42");
    }

    #[test]
    fn row_value_str_username() {
        assert_eq!(row_value_str(7, 1), "user_0000007");
    }

    #[test]
    fn row_value_str_dept_null_every_17() {
        assert_eq!(row_value_str(0, 2), "");
        assert_eq!(row_value_str(17, 2), "");
        assert!(!row_value_str(1, 2).is_empty());
    }

    #[test]
    fn row_value_str_salary_null_every_11() {
        assert_eq!(row_value_str(0, 3), "");
        assert_eq!(row_value_str(11, 3), "");
        assert!(!row_value_str(1, 3).is_empty());
    }

    #[test]
    fn row_value_str_active() {
        // row 3 → i%3 == 0 → false
        assert_eq!(row_value_str(3, 4), "false");
        // row 1 → i%3 != 0 → true
        assert_eq!(row_value_str(1, 4), "true");
    }

    #[test]
    fn row_value_str_unknown_col() {
        assert_eq!(row_value_str(5, 99), "");
    }

    // row_matches_filter ───────────────────────────────────────────────────────

    #[test]
    fn filter_contains_case_insensitive() {
        // Row 5: 5 % 5 == 0 → DEPTS[0] = "Engineering", 5 % 17 != 0 so not null.
        assert!(row_matches_filter(5, 2, &FilterKind::Contains("ENG".to_string())));
    }

    #[test]
    fn filter_starts_with() {
        assert!(row_matches_filter(1, 1, &FilterKind::StartsWith("user".to_string())));
        assert!(!row_matches_filter(1, 1, &FilterKind::StartsWith("admin".to_string())));
    }

    #[test]
    fn filter_regex_falls_back_to_contains() {
        assert!(row_matches_filter(1, 1, &FilterKind::Regex("0000001".to_string())));
    }

    // compare_rows ─────────────────────────────────────────────────────────────

    #[test]
    fn compare_rows_id_ascending() {
        use std::cmp::Ordering;
        assert_eq!(compare_rows(1, 5, 0), Ordering::Less);
        assert_eq!(compare_rows(5, 1, 0), Ordering::Greater);
        assert_eq!(compare_rows(3, 3, 0), Ordering::Equal);
    }

    #[test]
    fn compare_rows_salary_null_sorts_last() {
        use std::cmp::Ordering;
        // Row 11 has null salary (11 % 11 == 0); row 1 has a value.
        // Nulls sort after non-nulls in ascending: 11 > 1.
        assert_eq!(compare_rows(11, 1, 3), Ordering::Greater);
        assert_eq!(compare_rows(1, 11, 3), Ordering::Less);
    }

    #[test]
    fn compare_rows_active_true_before_false() {
        use std::cmp::Ordering;
        // Row 1: active=true, Row 3: active=false.
        // In ascending, true < false (true sorts first).
        assert_eq!(compare_rows(1, 3, 4), Ordering::Less);
    }
}
