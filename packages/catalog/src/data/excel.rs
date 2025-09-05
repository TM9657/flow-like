use std::sync::Arc;

use flow_like_types::Value;
use serde::{Deserialize, Serialize};

use flow_like_storage::arrow::array::{ArrayRef, StringArray};
use flow_like_storage::arrow::datatypes::{DataType, Field, Schema as ArrowSchema};
use flow_like_storage::arrow::record_batch::RecordBatch;

use flow_like_storage::datafusion::datasource::memory::MemTable;
use flow_like_storage::datafusion::error::DataFusionError;
use flow_like_storage::datafusion::prelude::SessionContext;

pub mod copy_worksheet;
pub mod get_sheet_names;
pub mod insert_column;
pub mod insert_row;
pub mod new_worksheet;
pub mod read_cell;
pub mod remove_column;
pub mod remove_row;
pub mod try_extract_tables;
pub mod write_cell;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Table {
    headers: Vec<String>,
    rows: Vec<Vec<Value>>,
}

impl Table {
    pub fn new(headers: Vec<String>, rows: Vec<Vec<Value>>) -> Self {
        Self { headers, rows }
    }

    fn infer_column_type(&self, col_idx: usize) -> DataType {
        for row in &self.rows {
            if let Some(v) = row.get(col_idx) {
                match v {
                    Value::Bool(_) => return DataType::Boolean,
                    Value::Number(n) => {
                        if n.as_i64().is_some() || n.as_u64().is_some() {
                            return DataType::Int64;
                        }
                        if n.as_f64().is_some() {
                            return DataType::Float64;
                        }
                    }
                    Value::String(_) => return DataType::Utf8,
                    _ => {}
                }
            }
        }
        DataType::Utf8
    }

    pub fn arrow_schema(&self) -> Arc<ArrowSchema> {
        let fields = self
            .headers
            .iter()
            .enumerate()
            .map(|(col_idx, h)| {
                let inferred = self.infer_column_type(col_idx);
                // always nullable to be safe
                Field::new(h, inferred, true)
            })
            .collect::<Vec<_>>();
        Arc::new(ArrowSchema::new(fields))
    }

   pub fn to_record_batch(&self) -> flow_like_storage::datafusion::error::Result<RecordBatch> {
        use flow_like_storage::arrow::array::{
            ArrayRef, BooleanArray, Float64Array, Int64Array, StringArray,
        };

        let schema = self.arrow_schema();
        let ncols = self.headers.len();
        let mut arrays: Vec<ArrayRef> = Vec::with_capacity(ncols);

        for col_idx in 0..ncols {
            let data_type = schema.field(col_idx).data_type();
            match data_type {
                DataType::Boolean => {
                    let mut col: Vec<Option<bool>> = Vec::with_capacity(self.rows.len());
                    for row in &self.rows {
                        let opt = row.get(col_idx).and_then(|v| match v {
                            Value::Bool(b) => Some(*b),
                            Value::String(s) => match s.to_lowercase().as_str() {
                                "true" => Some(true),
                                "false" => Some(false),
                                _ => None,
                            },
                            Value::Number(n) => {
                                // accept 0/1 as booleans if present
                                if let Some(i) = n.as_i64() {
                                    Some(i != 0)
                                } else if let Some(f) = n.as_f64() {
                                    Some(f != 0.0)
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        });
                        col.push(opt);
                    }
                    let array = BooleanArray::from(col);
                    arrays.push(Arc::new(array) as ArrayRef);
                }
                DataType::Int64 => {
                    let mut col: Vec<Option<i64>> = Vec::with_capacity(self.rows.len());
                    for row in &self.rows {
                        let opt = row.get(col_idx).and_then(|v| match v {
                            Value::Number(n) => n.as_i64().or_else(|| n.as_u64().map(|u| u as i64)),
                            Value::String(s) => s.parse::<i64>().ok(),
                            Value::Bool(b) => Some(if *b { 1 } else { 0 }),
                            _ => None,
                        });
                        col.push(opt);
                    }
                    let array = Int64Array::from(col);
                    arrays.push(Arc::new(array) as ArrayRef);
                }
                DataType::Float64 => {
                    let mut col: Vec<Option<f64>> = Vec::with_capacity(self.rows.len());
                    for row in &self.rows {
                        let opt = row.get(col_idx).and_then(|v| match v {
                            Value::Number(n) => n.as_f64(),
                            Value::String(s) => s.parse::<f64>().ok(),
                            Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
                            _ => None,
                        });
                        col.push(opt);
                    }
                    let array = Float64Array::from(col);
                    arrays.push(Arc::new(array) as ArrayRef);
                }
                _ => {
                    // Utf8 fallback
                    let mut col: Vec<Option<String>> = Vec::with_capacity(self.rows.len());
                    for row in &self.rows {
                        let opt = row.get(col_idx).and_then(|v| match v {
                            Value::Null => None,
                            Value::String(s) => Some(s.clone()),
                            Value::Bool(b) => Some(b.to_string()),
                            Value::Number(n) => Some(n.to_string()),
                            other => flow_like_types::json::to_string(other).ok(),
                        });
                        col.push(opt);
                    }
                    let array = StringArray::from(col);
                    arrays.push(Arc::new(array) as ArrayRef);
                }
            }
        }

        RecordBatch::try_new(schema, arrays).map_err(|e| DataFusionError::ArrowError(e, None))
    }

    pub fn register_with_datafusion(
        &self,
        ctx: &SessionContext,
        table_name: &str,
    ) -> flow_like_storage::datafusion::error::Result<()> {
        let batch = self.to_record_batch()?;
        let schema = self.arrow_schema();
        let mem = MemTable::try_new(schema, vec![vec![batch]])?;
        ctx.register_table(table_name, Arc::new(mem))?;
        Ok(())
    }
}