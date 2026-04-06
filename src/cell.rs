//! Typed Arrow cell value rendering.
//!
//! Values are read directly from Arrow arrays at the moment of rendering.
//! No intermediate `Vec<String>` or `HashMap` is allocated.

use arrow_array::types::{
    ArrowPrimitiveType, Float32Type, Float64Type, Int8Type, Int16Type, Int32Type, Int64Type,
    UInt8Type, UInt16Type, UInt32Type, UInt64Type,
};
use arrow_array::{
    Array, BooleanArray, LargeStringArray, PrimitiveArray, RecordBatch, StringArray,
    StringViewArray,
};
use arrow_cast::display::{ArrayFormatter, FormatOptions};
use arrow_schema::DataType;

/// Render the value at `(col_idx, row_idx)` in `batch` as a display string.
///
/// # Fallback behaviour
///
/// - **Out-of-bounds indices** (`col_idx >= num_columns` or `row_idx >= num_rows`): returns `"?"`.
///   This will not panic the WASM runtime.
/// - **Null slots**: returns `"NULL"`.
/// - **Unrecognised Arrow types** (e.g. `Date32`, `Timestamp`, `Decimal128`): delegates to
///   Arrow's [`ArrayFormatter`], which uses ISO 8601 for temporal types and full precision for
///   decimals. Pre-cast the column to `Utf8` if you need a custom format.
pub fn render_cell(batch: &RecordBatch, col_idx: usize, row_idx: usize) -> String {
    if col_idx >= batch.num_columns() || row_idx >= batch.num_rows() {
        return "?".to_owned();
    }
    let array = batch.column(col_idx);
    if array.is_null(row_idx) {
        return "NULL".to_owned();
    }
    arrow_value_to_string(array.as_ref(), row_idx)
}

fn arrow_value_to_string(array: &dyn Array, row: usize) -> String {
    match array.data_type() {
        DataType::Int8 => fmt_primitive::<Int8Type>(array, row),
        DataType::Int16 => fmt_primitive::<Int16Type>(array, row),
        DataType::Int32 => fmt_primitive::<Int32Type>(array, row),
        DataType::Int64 => fmt_primitive::<Int64Type>(array, row),
        DataType::UInt8 => fmt_primitive::<UInt8Type>(array, row),
        DataType::UInt16 => fmt_primitive::<UInt16Type>(array, row),
        DataType::UInt32 => fmt_primitive::<UInt32Type>(array, row),
        DataType::UInt64 => fmt_primitive::<UInt64Type>(array, row),
        DataType::Float32 => fmt_float::<Float32Type>(array, row),
        DataType::Float64 => fmt_float::<Float64Type>(array, row),
        DataType::Boolean => match array.as_any().downcast_ref::<BooleanArray>() {
            Some(arr) => arr.value(row).to_string(),
            None => fmt_via_array_formatter(array, row),
        },
        DataType::Utf8 => match array.as_any().downcast_ref::<StringArray>() {
            Some(arr) => arr.value(row).to_owned(),
            None => fmt_via_array_formatter(array, row),
        },
        DataType::LargeUtf8 => match array.as_any().downcast_ref::<LargeStringArray>() {
            Some(arr) => arr.value(row).to_owned(),
            None => fmt_via_array_formatter(array, row),
        },
        DataType::Utf8View => match array.as_any().downcast_ref::<StringViewArray>() {
            Some(arr) => arr.value(row).to_owned(),
            None => fmt_via_array_formatter(array, row),
        }
        // All other types: use Arrow's built-in display formatting.
        _ => fmt_via_array_formatter(array, row),
    }
}

/// Use `arrow_cast::display::ArrayFormatter` for production-quality rendering
/// of dates, timestamps, decimals, binary, etc.
fn fmt_via_array_formatter(array: &dyn Array, row: usize) -> String {
    match ArrayFormatter::try_new(array, &FormatOptions::default()) {
        Ok(fmt) => fmt.value(row).to_string(),
        Err(_) => "?".to_owned(),
    }
}

fn fmt_primitive<T: ArrowPrimitiveType>(array: &dyn Array, row: usize) -> String
where
    T::Native: std::fmt::Display,
{
    match array.as_any().downcast_ref::<PrimitiveArray<T>>() {
        Some(arr) => arr.value(row).to_string(),
        None => fmt_via_array_formatter(array, row),
    }
}

fn fmt_float<T: ArrowPrimitiveType>(array: &dyn Array, row: usize) -> String
where
    T::Native: std::fmt::Display,
{
    match array.as_any().downcast_ref::<PrimitiveArray<T>>() {
        Some(arr) => format!("{:.6}", arr.value(row)),
        None => fmt_via_array_formatter(array, row),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_array::{BooleanArray, Float64Array, Int64Array, RecordBatch, StringArray};
    use arrow_schema::{Field, Schema};
    use std::sync::Arc;

    fn test_batch() -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, true),
            Field::new("name", DataType::Utf8, true),
            Field::new("value", DataType::Float64, false),
            Field::new("flag", DataType::Boolean, false),
        ]));

        let ids = Int64Array::from(vec![Some(42), None, Some(99)]);
        let names = StringArray::from(vec![Some("alice"), None, Some("bob")]);
        #[allow(clippy::approx_constant)]
        let values = Float64Array::from(vec![3.14, 2.718, 1.0]);
        let flags = BooleanArray::from(vec![true, false, true]);

        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(ids),
                Arc::new(names),
                Arc::new(values),
                Arc::new(flags),
            ],
        )
        .expect("batch")
    }

    #[test]
    fn render_int64() {
        let batch = test_batch();
        assert_eq!(render_cell(&batch, 0, 0), "42");
    }

    #[test]
    fn render_null_value() {
        let batch = test_batch();
        assert_eq!(render_cell(&batch, 0, 1), "NULL");
        assert_eq!(render_cell(&batch, 1, 1), "NULL");
    }

    #[test]
    fn render_string() {
        let batch = test_batch();
        assert_eq!(render_cell(&batch, 1, 0), "alice");
    }

    #[test]
    fn render_float64() {
        let batch = test_batch();
        assert_eq!(render_cell(&batch, 2, 0), "3.140000");
    }

    #[test]
    fn render_boolean() {
        let batch = test_batch();
        assert_eq!(render_cell(&batch, 3, 0), "true");
        assert_eq!(render_cell(&batch, 3, 1), "false");
    }

    #[test]
    fn render_fallback_for_date_type() {
        use arrow_array::Date32Array;
        let schema = Arc::new(Schema::new(vec![Field::new("d", DataType::Date32, false)]));
        let dates = Date32Array::from(vec![18628]); // 2021-01-01
        let batch =
            RecordBatch::try_new(schema, vec![Arc::new(dates)]).expect("test batch");
        let val = render_cell(&batch, 0, 0);
        assert!(!val.is_empty());
        assert_ne!(val, "?");
    }

    #[test]
    fn render_out_of_bounds_col() {
        let batch = test_batch();
        assert_eq!(render_cell(&batch, 99, 0), "?");
    }

    #[test]
    fn render_out_of_bounds_row() {
        let batch = test_batch();
        assert_eq!(render_cell(&batch, 0, 99), "?");
    }
}
