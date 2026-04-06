//! In-memory Arrow RecordBatch generator — no network, no files.

use std::sync::Arc;

use arrow_array::{
    builder::{BooleanBuilder, Int64Builder, StringBuilder},
    RecordBatch,
};
use arrow_schema::{DataType, Field, Schema};

/// Generate `row_count` rows of realistic mock data natively in WASM memory.
pub fn generate_mock_batch(row_count: usize) -> Arc<RecordBatch> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("username", DataType::Utf8, false),
        Field::new("department", DataType::Utf8, true),
        Field::new("salary", DataType::Int64, true),
        Field::new("is_active", DataType::Boolean, false),
    ]));

    let mut id_b = Int64Builder::with_capacity(row_count);
    let mut name_b = StringBuilder::with_capacity(row_count, row_count * 10);
    let mut dept_b = StringBuilder::with_capacity(row_count, row_count * 8);
    let mut sal_b = Int64Builder::with_capacity(row_count);
    let mut active_b = BooleanBuilder::with_capacity(row_count);

    let depts = ["Engineering", "Sales", "Finance", "HR", "Legal"];

    for i in 0..row_count {
        id_b.append_value(i as i64);
        name_b.append_value(format!("user_{i:07}"));
        if i % 17 == 0 {
            dept_b.append_null();
        } else {
            dept_b.append_value(depts[i % depts.len()]);
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
