//! In-memory Arrow RecordBatch generator — no network, no files.
//!
//! The mock dataset has **20 columns** spanning Int32, Int64, UInt32, Float64,
//! Boolean, and Utf8 Arrow types, with various nullability patterns, so the
//! playground exercises every cell-rendering and filter path.

use std::sync::Arc;

use arrow_array::{
    RecordBatch,
    builder::{
        BooleanBuilder, Float64Builder, Int32Builder, Int64Builder, StringBuilder, UInt32Builder,
    },
};
use arrow_schema::{DataType, Field, Schema, SchemaRef};

pub const DEPTS: &[&str] = &["Engineering", "Sales", "Finance", "HR", "Legal"];
pub const REGIONS: &[&str] = &["EMEA", "AMER", "APAC", "LATAM"];
pub const TEAMS: &[&str] = &["Alpha", "Bravo", "Charlie", "Delta", "Echo", "Foxtrot"];
pub const COUNTRIES: &[&str] = &["USA", "UK", "Germany", "France", "Japan", "Brazil", "Canada", "India"];
pub const ACCOUNT_TYPES: &[&str] = &["Free", "Pro", "Enterprise", "Trial"];

/// Canonical schema for the mock dataset (20 columns, mixed Arrow types).
///
/// | # | name         | type    | nullable | null rule       |
/// |---|--------------|---------|----------|-----------------|
/// | 0 | id           | Int64   | no       | —               |
/// | 1 | username     | Utf8    | no       | —               |
/// | 2 | email        | Utf8    | yes      | i % 23 == 0     |
/// | 3 | department   | Utf8    | yes      | i % 17 == 0     |
/// | 4 | salary       | Int64   | yes      | i % 11 == 0     |
/// | 5 | is_active    | Boolean | no       | —               |
/// | 6 | score        | Float64 | yes      | i % 13 == 0     |
/// | 7 | level        | Int32   | no       | —               |
/// | 8 | region       | Utf8    | no       | —               |
/// | 9 | team         | Utf8    | yes      | i % 7 == 0      |
/// |10 | start_year   | Int32   | yes      | i % 29 == 0     |
/// |11 | manager_id   | Int64   | yes      | i % 5 == 0      |
/// |12 | reports      | Int32   | no       | —               |
/// |13 | badge_id     | UInt32  | no       | —               |
/// |14 | phone        | Utf8    | yes      | i % 3 == 0      |
/// |15 | avg_rating   | Float64 | yes      | i % 19 == 0     |
/// |16 | login_count  | Int32   | no       | —               |
/// |17 | country      | Utf8    | no       | —               |
/// |18 | account_type | Utf8    | yes      | i % 8 == 0      |
/// |19 | is_verified  | Boolean | no       | —               |
pub fn mock_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("id",           DataType::Int64,   false),
        Field::new("username",     DataType::Utf8,    false),
        Field::new("email",        DataType::Utf8,    true),
        Field::new("department",   DataType::Utf8,    true),
        Field::new("salary",       DataType::Int64,   true),
        Field::new("is_active",    DataType::Boolean, false),
        Field::new("score",        DataType::Float64, true),
        Field::new("level",        DataType::Int32,   false),
        Field::new("region",       DataType::Utf8,    false),
        Field::new("team",         DataType::Utf8,    true),
        Field::new("start_year",   DataType::Int32,   true),
        Field::new("manager_id",   DataType::Int64,   true),
        Field::new("reports",      DataType::Int32,   false),
        Field::new("badge_id",     DataType::UInt32,  false),
        Field::new("phone",        DataType::Utf8,    true),
        Field::new("avg_rating",   DataType::Float64, true),
        Field::new("login_count",  DataType::Int32,   false),
        Field::new("country",      DataType::Utf8,    false),
        Field::new("account_type", DataType::Utf8,    true),
        Field::new("is_verified",  DataType::Boolean, false),
    ]))
}

/// Generate `count` rows starting from row `offset`.
pub fn generate_mock_batch_range(offset: usize, count: usize) -> Arc<RecordBatch> {
    let schema = mock_schema();

    let mut id_b      = Int64Builder::with_capacity(count);
    let mut name_b    = StringBuilder::with_capacity(count, count * 10);
    let mut email_b   = StringBuilder::with_capacity(count, count * 20);
    let mut dept_b    = StringBuilder::with_capacity(count, count * 8);
    let mut sal_b     = Int64Builder::with_capacity(count);
    let mut active_b  = BooleanBuilder::with_capacity(count);
    let mut score_b   = Float64Builder::with_capacity(count);
    let mut level_b   = Int32Builder::with_capacity(count);
    let mut region_b  = StringBuilder::with_capacity(count, count * 5);
    let mut team_b    = StringBuilder::with_capacity(count, count * 7);
    let mut year_b    = Int32Builder::with_capacity(count);
    let mut mgr_b     = Int64Builder::with_capacity(count);
    let mut reports_b = Int32Builder::with_capacity(count);
    let mut badge_b   = UInt32Builder::with_capacity(count);
    let mut phone_b   = StringBuilder::with_capacity(count, count * 14);
    let mut rating_b  = Float64Builder::with_capacity(count);
    let mut logins_b  = Int32Builder::with_capacity(count);
    let mut country_b = StringBuilder::with_capacity(count, count * 8);
    let mut acct_b    = StringBuilder::with_capacity(count, count * 10);
    let mut verified_b = BooleanBuilder::with_capacity(count);

    for i in offset..offset + count {
        id_b.append_value(i as i64);
        name_b.append_value(format!("user_{i:07}"));

        if i % 23 == 0 { email_b.append_null(); }
        else { email_b.append_value(format!("user_{i:07}@example.com")); }

        if i % 17 == 0 { dept_b.append_null(); }
        else { dept_b.append_value(DEPTS[i % DEPTS.len()]); }

        if i % 11 == 0 { sal_b.append_null(); }
        else { sal_b.append_value(50_000 + (i as i64 % 100_000)); }

        active_b.append_value(i % 3 != 0);

        if i % 13 == 0 { score_b.append_null(); }
        else { score_b.append_value(50.0 + (i % 500) as f64 * 0.1); }

        level_b.append_value((i % 10 + 1) as i32);
        region_b.append_value(REGIONS[i % REGIONS.len()]);

        if i % 7 == 0 { team_b.append_null(); }
        else { team_b.append_value(TEAMS[i % TEAMS.len()]); }

        if i % 29 == 0 { year_b.append_null(); }
        else { year_b.append_value(2015 + (i % 10) as i32); }

        if i % 5 == 0 { mgr_b.append_null(); }
        else { mgr_b.append_value(((i / 10) * 10) as i64); }

        reports_b.append_value((i % 12) as i32);
        badge_b.append_value((100_000 + i) as u32);

        if i % 3 == 0 { phone_b.append_null(); }
        else {
            let area = 200 + i % 800;
            let num  = 1_000_000 + i % 9_000_000;
            phone_b.append_value(format!("+1-{area:03}-{num:07}"));
        }

        if i % 19 == 0 { rating_b.append_null(); }
        else { rating_b.append_value(1.0 + (i % 40) as f64 * 0.1); }

        logins_b.append_value((i % 1000) as i32);
        country_b.append_value(COUNTRIES[i % COUNTRIES.len()]);

        if i % 8 == 0 { acct_b.append_null(); }
        else { acct_b.append_value(ACCOUNT_TYPES[i % ACCOUNT_TYPES.len()]); }

        verified_b.append_value(i % 4 != 0);
    }

    Arc::new(
        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(id_b.finish()),
                Arc::new(name_b.finish()),
                Arc::new(email_b.finish()),
                Arc::new(dept_b.finish()),
                Arc::new(sal_b.finish()),
                Arc::new(active_b.finish()),
                Arc::new(score_b.finish()),
                Arc::new(level_b.finish()),
                Arc::new(region_b.finish()),
                Arc::new(team_b.finish()),
                Arc::new(year_b.finish()),
                Arc::new(mgr_b.finish()),
                Arc::new(reports_b.finish()),
                Arc::new(badge_b.finish()),
                Arc::new(phone_b.finish()),
                Arc::new(rating_b.finish()),
                Arc::new(logins_b.finish()),
                Arc::new(country_b.finish()),
                Arc::new(acct_b.finish()),
                Arc::new(verified_b.finish()),
            ],
        )
        .expect("mock batch construction is infallible"),
    )
}

/// Generate a batch from an explicit list of (sorted/filtered) row indices.
pub fn generate_mock_batch_from_indices(indices: &[usize]) -> Arc<RecordBatch> {
    let count = indices.len();
    let schema = mock_schema();

    let mut id_b      = Int64Builder::with_capacity(count);
    let mut name_b    = StringBuilder::with_capacity(count, count * 10);
    let mut email_b   = StringBuilder::with_capacity(count, count * 20);
    let mut dept_b    = StringBuilder::with_capacity(count, count * 8);
    let mut sal_b     = Int64Builder::with_capacity(count);
    let mut active_b  = BooleanBuilder::with_capacity(count);
    let mut score_b   = Float64Builder::with_capacity(count);
    let mut level_b   = Int32Builder::with_capacity(count);
    let mut region_b  = StringBuilder::with_capacity(count, count * 5);
    let mut team_b    = StringBuilder::with_capacity(count, count * 7);
    let mut year_b    = Int32Builder::with_capacity(count);
    let mut mgr_b     = Int64Builder::with_capacity(count);
    let mut reports_b = Int32Builder::with_capacity(count);
    let mut badge_b   = UInt32Builder::with_capacity(count);
    let mut phone_b   = StringBuilder::with_capacity(count, count * 14);
    let mut rating_b  = Float64Builder::with_capacity(count);
    let mut logins_b  = Int32Builder::with_capacity(count);
    let mut country_b = StringBuilder::with_capacity(count, count * 8);
    let mut acct_b    = StringBuilder::with_capacity(count, count * 10);
    let mut verified_b = BooleanBuilder::with_capacity(count);

    for &i in indices {
        id_b.append_value(i as i64);
        name_b.append_value(format!("user_{i:07}"));

        if i % 23 == 0 { email_b.append_null(); }
        else { email_b.append_value(format!("user_{i:07}@example.com")); }

        if i % 17 == 0 { dept_b.append_null(); }
        else { dept_b.append_value(DEPTS[i % DEPTS.len()]); }

        if i % 11 == 0 { sal_b.append_null(); }
        else { sal_b.append_value(50_000 + (i as i64 % 100_000)); }

        active_b.append_value(i % 3 != 0);

        if i % 13 == 0 { score_b.append_null(); }
        else { score_b.append_value(50.0 + (i % 500) as f64 * 0.1); }

        level_b.append_value((i % 10 + 1) as i32);
        region_b.append_value(REGIONS[i % REGIONS.len()]);

        if i % 7 == 0 { team_b.append_null(); }
        else { team_b.append_value(TEAMS[i % TEAMS.len()]); }

        if i % 29 == 0 { year_b.append_null(); }
        else { year_b.append_value(2015 + (i % 10) as i32); }

        if i % 5 == 0 { mgr_b.append_null(); }
        else { mgr_b.append_value(((i / 10) * 10) as i64); }

        reports_b.append_value((i % 12) as i32);
        badge_b.append_value((100_000 + i) as u32);

        if i % 3 == 0 { phone_b.append_null(); }
        else {
            let area = 200 + i % 800;
            let num  = 1_000_000 + i % 9_000_000;
            phone_b.append_value(format!("+1-{area:03}-{num:07}"));
        }

        if i % 19 == 0 { rating_b.append_null(); }
        else { rating_b.append_value(1.0 + (i % 40) as f64 * 0.1); }

        logins_b.append_value((i % 1000) as i32);
        country_b.append_value(COUNTRIES[i % COUNTRIES.len()]);

        if i % 8 == 0 { acct_b.append_null(); }
        else { acct_b.append_value(ACCOUNT_TYPES[i % ACCOUNT_TYPES.len()]); }

        verified_b.append_value(i % 4 != 0);
    }

    Arc::new(
        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(id_b.finish()),
                Arc::new(name_b.finish()),
                Arc::new(email_b.finish()),
                Arc::new(dept_b.finish()),
                Arc::new(sal_b.finish()),
                Arc::new(active_b.finish()),
                Arc::new(score_b.finish()),
                Arc::new(level_b.finish()),
                Arc::new(region_b.finish()),
                Arc::new(team_b.finish()),
                Arc::new(year_b.finish()),
                Arc::new(mgr_b.finish()),
                Arc::new(reports_b.finish()),
                Arc::new(badge_b.finish()),
                Arc::new(phone_b.finish()),
                Arc::new(rating_b.finish()),
                Arc::new(logins_b.finish()),
                Arc::new(country_b.finish()),
                Arc::new(acct_b.finish()),
                Arc::new(verified_b.finish()),
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
        0  => i.to_string(),
        1  => format!("user_{i:07}"),
        2  => {
            if i % 23 == 0 { String::new() }
            else { format!("user_{i:07}@example.com") }
        }
        3  => {
            if i % 17 == 0 { String::new() }
            else { DEPTS[i % DEPTS.len()].to_string() }
        }
        4  => {
            if i % 11 == 0 { String::new() }
            else { (50_000 + i % 100_000).to_string() }
        }
        5  => (i % 3 != 0).to_string(),
        6  => {
            if i % 13 == 0 { String::new() }
            else { format!("{:.6}", 50.0 + (i % 500) as f64 * 0.1) }
        }
        7  => (i % 10 + 1).to_string(),
        8  => REGIONS[i % REGIONS.len()].to_string(),
        9  => {
            if i % 7 == 0 { String::new() }
            else { TEAMS[i % TEAMS.len()].to_string() }
        }
        10 => {
            if i % 29 == 0 { String::new() }
            else { (2015 + i % 10).to_string() }
        }
        11 => {
            if i % 5 == 0 { String::new() }
            else { ((i / 10) * 10).to_string() }
        }
        12 => (i % 12).to_string(),
        13 => (100_000 + i).to_string(),
        14 => {
            if i % 3 == 0 { String::new() }
            else {
                let area = 200 + i % 800;
                let num  = 1_000_000 + i % 9_000_000;
                format!("+1-{area:03}-{num:07}")
            }
        }
        15 => {
            if i % 19 == 0 { String::new() }
            else { format!("{:.1}", 1.0 + (i % 40) as f64 * 0.1) }
        }
        16 => (i % 1000).to_string(),
        17 => COUNTRIES[i % COUNTRIES.len()].to_string(),
        18 => {
            if i % 8 == 0 { String::new() }
            else { ACCOUNT_TYPES[i % ACCOUNT_TYPES.len()].to_string() }
        }
        19 => (i % 4 != 0).to_string(),
        _  => String::new(),
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

    /// Compare two nullable integer values — `None` sorts last.
    fn cmp_opt<T: Ord>(va: Option<T>, vb: Option<T>, tie: Ordering) -> Ordering {
        match (va, vb) {
            (None, None)       => tie,
            (None, Some(_))    => Ordering::Greater,
            (Some(_), None)    => Ordering::Less,
            (Some(x), Some(y)) => x.cmp(&y),
        }
    }

    match col {
        // id and username order mirrors the row index directly.
        0 | 1 => a.cmp(&b),
        // email (nullable) — sort order equals row-index order; no String allocation
        2  => {
            let va = (a % 23 != 0).then_some(a);
            let vb = (b % 23 != 0).then_some(b);
            cmp_opt(va, vb, a.cmp(&b))
        }
        // department (nullable, string)
        3  => {
            let va = (a % 17 != 0).then(|| DEPTS[a % DEPTS.len()]);
            let vb = (b % 17 != 0).then(|| DEPTS[b % DEPTS.len()]);
            cmp_opt(va, vb, a.cmp(&b))
        }
        // salary (nullable, integer)
        4  => {
            let va = (a % 11 != 0).then(|| 50_000 + a % 100_000);
            let vb = (b % 11 != 0).then(|| 50_000 + b % 100_000);
            cmp_opt(va, vb, a.cmp(&b))
        }
        // is_active (bool): true sorts before false in ascending.
        5  => {
            // inactive (a%3==0) sorts after active (a%3!=0) — treat "not active" as larger.
            (a % 3 == 0).cmp(&(b % 3 == 0))
        }
        // score (nullable, float64) — compare by integer-scaled value to avoid f64::Ord issues.
        6  => {
            let va = (a % 13 != 0).then(|| (a % 500) as i64);
            let vb = (b % 13 != 0).then(|| (b % 500) as i64);
            cmp_opt(va, vb, a.cmp(&b))
        }
        // level (not null, Int32)
        7  => (a % 10 + 1).cmp(&(b % 10 + 1)),
        // region (not null, Utf8)
        8  => REGIONS[a % REGIONS.len()].cmp(REGIONS[b % REGIONS.len()]),
        // team (nullable, Utf8)
        9  => {
            let va = (a % 7 != 0).then(|| TEAMS[a % TEAMS.len()]);
            let vb = (b % 7 != 0).then(|| TEAMS[b % TEAMS.len()]);
            cmp_opt(va, vb, a.cmp(&b))
        }
        // start_year (nullable, Int32)
        10 => {
            let va = (a % 29 != 0).then(|| 2015 + a % 10);
            let vb = (b % 29 != 0).then(|| 2015 + b % 10);
            cmp_opt(va, vb, a.cmp(&b))
        }
        // manager_id (nullable, Int64)
        11 => {
            let va = (a % 5 != 0).then(|| (a / 10) * 10);
            let vb = (b % 5 != 0).then(|| (b / 10) * 10);
            cmp_opt(va, vb, a.cmp(&b))
        }
        // reports (not null, Int32)
        12 => (a % 12).cmp(&(b % 12)),
        // badge_id (not null, UInt32) — mirrors row index
        13 => a.cmp(&b),
        // phone (nullable) — compare integer components directly; no String allocation
        14 => {
            let key = |i: usize| (200 + i % 800, 1_000_000 + i % 9_000_000);
            let va = (a % 3 != 0).then(|| key(a));
            let vb = (b % 3 != 0).then(|| key(b));
            cmp_opt(va, vb, a.cmp(&b))
        }
        // avg_rating (nullable, Float64) — compare scaled integer to avoid f64::Ord
        15 => {
            let va = (a % 19 != 0).then(|| a % 40);
            let vb = (b % 19 != 0).then(|| b % 40);
            cmp_opt(va, vb, a.cmp(&b))
        }
        // login_count (not null, Int32)
        16 => (a % 1000).cmp(&(b % 1000)),
        // country (not null, Utf8)
        17 => COUNTRIES[a % COUNTRIES.len()].cmp(COUNTRIES[b % COUNTRIES.len()]),
        // account_type (nullable, Utf8)
        18 => {
            let va = (a % 8 != 0).then(|| ACCOUNT_TYPES[a % ACCOUNT_TYPES.len()]);
            let vb = (b % 8 != 0).then(|| ACCOUNT_TYPES[b % ACCOUNT_TYPES.len()]);
            cmp_opt(va, vb, a.cmp(&b))
        }
        // is_verified (not null, Boolean): verified (i%4!=0 → true) sorts before unverified.
        19 => (a % 4 == 0).cmp(&(b % 4 == 0)),
        _ => Ordering::Equal,
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
    fn row_value_str_email_null_every_23() {
        assert_eq!(row_value_str(0, 2), "");
        assert_eq!(row_value_str(23, 2), "");
        assert!(!row_value_str(1, 2).is_empty());
    }

    #[test]
    fn row_value_str_dept_null_every_17() {
        assert_eq!(row_value_str(0, 3), "");
        assert_eq!(row_value_str(17, 3), "");
        assert!(!row_value_str(1, 3).is_empty());
    }

    #[test]
    fn row_value_str_salary_null_every_11() {
        assert_eq!(row_value_str(0, 4), "");
        assert_eq!(row_value_str(11, 4), "");
        assert!(!row_value_str(1, 4).is_empty());
    }

    #[test]
    fn row_value_str_active() {
        assert_eq!(row_value_str(3, 5), "false");
        assert_eq!(row_value_str(1, 5), "true");
    }

    #[test]
    fn row_value_str_score_null_every_13() {
        assert_eq!(row_value_str(0, 6), "");
        assert_eq!(row_value_str(13, 6), "");
        assert!(!row_value_str(1, 6).is_empty());
    }

    #[test]
    fn row_value_str_level_range_1_to_10() {
        for i in 0..20usize {
            let v: usize = row_value_str(i, 7).parse().unwrap();
            assert!((1..=10).contains(&v), "level={v} for row {i}");
        }
    }

    #[test]
    fn row_value_str_region_known_value() {
        assert!(REGIONS.contains(&row_value_str(0, 8).as_str()));
    }

    #[test]
    fn row_value_str_team_null_every_7() {
        assert_eq!(row_value_str(0, 9), "");
        assert_eq!(row_value_str(7, 9), "");
        assert!(!row_value_str(1, 9).is_empty());
    }

    #[test]
    fn row_value_str_phone_null_every_3() {
        assert_eq!(row_value_str(0, 14), "");
        assert_eq!(row_value_str(3, 14), "");
        assert!(!row_value_str(1, 14).is_empty());
    }

    #[test]
    fn row_value_str_unknown_col() {
        assert_eq!(row_value_str(5, 99), "");
    }

    // row_matches_filter ───────────────────────────────────────────────────────

    #[test]
    fn filter_contains_case_insensitive() {
        // Row 5: dept = DEPTS[5 % 5] = DEPTS[0] = "Engineering", 5 % 17 != 0 → not null.
        assert!(row_matches_filter(5, 3, &FilterKind::Contains("ENG".to_string())));
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
        assert_eq!(compare_rows(11, 1, 4), Ordering::Greater);
        assert_eq!(compare_rows(1, 11, 4), Ordering::Less);
    }

    #[test]
    fn compare_rows_active_true_before_false() {
        use std::cmp::Ordering;
        // Row 1: active=true, Row 3: active=false. True sorts first.
        assert_eq!(compare_rows(1, 3, 5), Ordering::Less);
    }

    #[test]
    fn compare_rows_phone_null_sorts_last() {
        use std::cmp::Ordering;
        // Row 3: null phone (3 % 3 == 0); Row 1: has phone.
        assert_eq!(compare_rows(3, 1, 14), Ordering::Greater);
        assert_eq!(compare_rows(1, 3, 14), Ordering::Less);
    }
}
