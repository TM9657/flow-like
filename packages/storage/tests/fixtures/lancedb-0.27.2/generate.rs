use std::sync::Arc;

use arrow_array::{
    Date32Array, FixedSizeListArray, Int64Array, RecordBatch, StringArray,
    TimestampMillisecondArray, types::Float32Type,
};
use arrow_schema::{DataType, Field, Schema, TimeUnit};
use lance::dataset::{WriteMode, WriteParams};
use lance_file::version::LanceFileVersion;
use lancedb::database::listing::{ListingDatabaseOptions, NewTableConfig};
use lancedb::index::{Index, scalar::BTreeIndexBuilder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args()
        .nth(1)
        .expect("usage: generator OUTPUT_DIRECTORY");
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("title", DataType::Utf8, false),
        Field::new("event_date", DataType::Date32, true),
        Field::new(
            "occurred_at",
            DataType::Timestamp(TimeUnit::Millisecond, Some("UTC".into())),
            true,
        ),
        Field::new(
            "vector",
            DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 4),
            false,
        ),
    ]));
    let vectors = FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
        [
            Some(vec![Some(1.0), Some(0.0), Some(0.0), Some(0.0)]),
            Some(vec![Some(0.0), Some(1.0), Some(0.0), Some(0.0)]),
            Some(vec![Some(0.0), Some(0.0), Some(1.0), Some(0.0)]),
            Some(vec![Some(0.0), Some(0.0), Some(0.0), Some(1.0)]),
        ],
        4,
    );
    let records = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(Int64Array::from(vec![1, 2, 3, 4])),
            Arc::new(StringArray::from(vec!["alpha", "beta", "gamma", "delta"])),
            Arc::new(Date32Array::from(vec![
                Some(20_089),
                Some(20_090),
                Some(20_091),
                None,
            ])),
            Arc::new(
                TimestampMillisecondArray::from(vec![
                    Some(1_735_689_600_000),
                    Some(1_735_776_000_000),
                    Some(1_735_862_400_000),
                    None,
                ])
                .with_timezone("UTC"),
            ),
            Arc::new(vectors),
        ],
    )?;
    let mut connection = lancedb::connect(&output);
    if std::env::args().nth(2).as_deref() == Some("v2.2") {
        connection = connection.database_options(&ListingDatabaseOptions {
            new_table_config: NewTableConfig {
                data_storage_version: Some(LanceFileVersion::V2_2),
                ..Default::default()
            },
            ..Default::default()
        });
    }
    let db = connection.execute().await?;
    let table = db
        .create_table("legacy", records)
        .write_options(lancedb::table::WriteOptions {
            lance_write_params: Some(WriteParams {
                data_storage_version: Some(LanceFileVersion::V2_2),
                mode: WriteMode::Append,
                ..Default::default()
            }),
        })
        .execute()
        .await?;
    for column in ["id", "event_date", "occurred_at"] {
        table
            .create_index(&[column], Index::BTree(BTreeIndexBuilder::default()))
            .execute()
            .await?;
    }
    println!(
        "Created legacy table with {} rows and {} indexes",
        table.count_rows(None).await?,
        table.list_indices().await?.len()
    );
    Ok(())
}
