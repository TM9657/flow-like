use std::sync::Arc;

use arrow::datatypes::FieldRef;
use arrow_array::{RecordBatch, RecordBatchIterator};
use arrow_schema::{DataType, Field};
use flow_like_types::{
    Result, Value, anyhow,
    json::{Deserialize, Serialize, to_value},
};
use serde_arrow::schema::{SchemaLike, TracingOptions};

pub fn value_to_record_batch(records: Vec<Value>) -> Result<RecordBatch> {
    // Determine Arrow schema
    let mut fields: Vec<std::sync::Arc<arrow_schema::Field>> =
        Vec::<FieldRef>::from_samples(&records, TracingOptions::new())?;

    // we need to make sure the vector column is actually a vector!!
    for field in &mut fields {
        if field.name() == "vector" {
            *field = Arc::new(Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    get_vector_dimension(&records)? as i32,
                ),
                true,
            ));
        }
    }
    //

    // Build a record batch
    let batch: RecordBatch = serde_arrow::to_record_batch(&fields, &records)?;
    Ok(batch)
}

fn get_vector_dimension<T>(records: &[T]) -> Result<i32>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    if records.is_empty() {
        return Err(anyhow!("No records to determine vector dimension"));
    }
    // Serialize the first record to JSON
    let first_record = &records[0];
    let serialized = to_value(first_record)?;

    // Check if the "vector" field exists and is a Vec<f32>
    if let Some(map) = serialized.as_object()
        && let Some(Value::Array(vec)) = map.get("vector")
        && !vec.is_empty()
    {
        // Determine the length of the vector
        return Ok(vec.len() as i32);
    }

    Err(anyhow!("Unable to determine vector dimension from records"))
}

pub fn value_to_batch_iterator(
    records: Vec<Value>,
) -> Result<
    RecordBatchIterator<
        std::iter::Map<
            std::array::IntoIter<RecordBatch, 1>,
            fn(RecordBatch) -> Result<RecordBatch, arrow_schema::ArrowError>,
        >,
    >,
> {
    // Determine Arrow schema
    let batch = value_to_record_batch(records)?;
    let schema = batch.schema();
    let iterator: RecordBatchIterator<
        std::iter::Map<
            std::array::IntoIter<RecordBatch, 1>,
            fn(RecordBatch) -> Result<RecordBatch, arrow_schema::ArrowError>,
        >,
    > = RecordBatchIterator::new([batch].into_iter().map(Ok), schema);

    Ok(iterator)
}

pub fn record_batch_to_value(record_batch: &RecordBatch) -> Result<Vec<Value>> {
    let items = serde_arrow::from_record_batch(record_batch)?;
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_like_types::json::{Deserialize, to_value};

    #[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
    struct TestStruct {
        id: i32,
        name: String,
    }

    #[test]
    fn test_value_to_batchreader_and_back() -> Result<()> {
        // Mock data as JSON Values
        let records = [
            TestStruct {
                id: 1,
                name: "Alice".to_string(),
            },
            TestStruct {
                id: 2,
                name: "Bob".to_string(),
            },
        ];

        let records = records
            .iter()
            .map(|r| to_value(r).unwrap())
            .collect::<Vec<Value>>();

        // Convert JSON to RecordBatch
        let record_batch = value_to_record_batch(records.clone())?;

        // Convert RecordBatch back to JSON
        let result = record_batch_to_value(&record_batch)?;

        // Check that the original data and the result match
        assert_eq!(records, result);

        Ok(())
    }
}
