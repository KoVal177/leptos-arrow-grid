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
