use arrow_array::RecordBatch;
use flow_like_types::Cacheable;
use flow_like_types::async_trait;
use flow_like_types::{Result, Value, anyhow};
use futures::TryStreamExt;
use lancedb::{
    Connection, Table, connect,
    index::{
        Index,
        scalar::{FtsIndexBuilder, FullTextSearchQuery},
    },
    query::{ExecutableQuery, QueryBase},
    table::{CompactionOptions, Duration, OptimizeOptions},
};
use std::{any::Any, path::PathBuf, sync::Arc};

use crate::arrow_utils::record_batch_to_value;
use crate::arrow_utils::value_to_batch_iterator;

use super::VectorStore;

#[derive(Clone)]
pub struct LanceDBVectorStore {
    connection: Connection,
    table: Option<Table>,
    table_name: String,
}

impl Cacheable for LanceDBVectorStore {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
impl LanceDBVectorStore {
    pub async fn new(path: PathBuf, table_name: String) -> Option<Self> {
        let connection = connect(path.to_str().unwrap()).execute().await.ok();
        connection.as_ref()?;
        let connection: Connection = connection.unwrap();

        let table = connection.open_table(&table_name).execute().await.ok();

        Some(LanceDBVectorStore {
            connection,
            table,
            table_name,
        })
    }

    pub async fn from_connection(connection: Connection, table_name: String) -> Self {
        let table = connection.open_table(&table_name).execute().await.ok();

        LanceDBVectorStore {
            connection,
            table,
            table_name,
        }
    }
}

fn record_batches_to_vec(batches: Option<Vec<RecordBatch>>) -> Result<Vec<Value>> {
    batches
        .as_ref()
        .ok_or(anyhow!("Error converting record batches to vec"))?;

    let batches = batches.unwrap();
    let mut items = vec![];

    for batch in batches {
        let values = record_batch_to_value(&batch);
        match values {
            Ok(mut values) => {
                items.append(&mut values);
            }
            Err(err) => {
                println!("Error converting batch to value: {:?}", err);
            }
        }
    }

    Ok(items)
}

#[async_trait]
impl VectorStore for LanceDBVectorStore {
    async fn vector_search(
        &self,
        vector: Vec<f64>,
        filter: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Value>> {
        let table = self
            .table
            .clone()
            .ok_or_else(|| anyhow!("Table not initialized"))?;

        let mut query = table
            .query()
            .nearest_to(vector)?
            .distance_type(lancedb::DistanceType::Cosine)
            .fast_search()
            .limit(limit)
            .offset(offset);

        if let Some(filter) = filter {
            query = query.only_if(filter);
        }

        let result = query.execute().await?;
        let result = result.try_collect::<Vec<_>>().await.ok();
        let result = record_batches_to_vec(result)?;
        Ok(result)
    }

    async fn fts_search(
        &self,
        text: &str,
        filter: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Value>> {
        let table = self
            .table
            .clone()
            .ok_or_else(|| anyhow!("Table not initialized"))?;

        let mut query = table
            .query()
            .full_text_search(FullTextSearchQuery::new(text.to_string()))
            .limit(limit)
            .offset(offset);

        if let Some(filter) = filter {
            query = query.only_if(filter);
        }

        let result = query.execute().await?;
        let result = result.try_collect::<Vec<_>>().await.ok();
        let result = record_batches_to_vec(result)?;
        Ok(result)
    }

    async fn hybrid_search(
        &self,
        vector: Vec<f64>,
        text: &str,
        filter: Option<&str>,
        limit: usize,
        offset: usize,
        rerank: bool,
    ) -> Result<Vec<Value>> {
        let table = self
            .table
            .clone()
            .ok_or_else(|| anyhow!("Table not initialized"))?;

        let mut query = table
            .query()
            .nearest_to(vector)?
            .distance_type(lancedb::DistanceType::Cosine)
            .full_text_search(FullTextSearchQuery::new(text.to_string()))
            .fast_search()
            .limit(limit)
            .offset(offset);

        if rerank {
            let reranker = Arc::new(lancedb::rerankers::rrf::RRFReranker::new(60.0));
            query = query.rerank(reranker);
        }

        if let Some(filter) = filter {
            query = query.only_if(filter);
        }

        let result = query.execute_hybrid().await?;
        let result = result.try_collect::<Vec<_>>().await.ok();
        let result = record_batches_to_vec(result)?;
        Ok(result)
    }

    async fn filter(&self, filter: &str, limit: usize, offset: usize) -> Result<Vec<Value>> {
        let table = self
            .table
            .clone()
            .ok_or_else(|| anyhow!("Table not initialized"))?;

        let query = table.query().limit(limit).only_if(filter).offset(offset);

        let result = query.execute().await?;
        let result = result.try_collect::<Vec<_>>().await.ok();
        let result = record_batches_to_vec(result)?;
        Ok(result)
    }

    async fn upsert(&mut self, items: Vec<Value>, id_field: String) -> Result<()> {
        let items = match value_to_batch_iterator(items) {
            Ok(items) => items,
            Err(err) => {
                return Err(anyhow!(err.to_string()));
            }
        };

        if self.table.is_none() {
            match self
                .connection
                .create_table(&self.table_name, items)
                .execute()
                .await
            {
                Ok(table) => {
                    self.table = Some(table);
                    return Ok(());
                }
                Err(err) => {
                    println!("Error creating table: {:?}", err);
                    return Err(anyhow!("Error creating table"));
                }
            }
        }

        let table = self.table.clone().unwrap();
        table
            .merge_insert(&[&id_field])
            .when_matched_update_all(None)
            .when_not_matched_insert_all()
            .to_owned()
            .execute(Box::new(items))
            .await?;
        Ok(())
    }

    async fn insert(&mut self, items: Vec<Value>) -> Result<()> {
        let items = match value_to_batch_iterator(items) {
            Ok(items) => items,
            Err(err) => {
                return Err(anyhow!(err.to_string()));
            }
        };

        if self.table.is_none() {
            match self
                .connection
                .create_table(&self.table_name, items)
                .execute()
                .await
            {
                Ok(table) => {
                    self.table = Some(table);
                    return Ok(());
                }
                Err(err) => {
                    println!("Error creating table: {:?}", err);
                    return Err(anyhow!("Error creating table"));
                }
            }
        }

        let table = self.table.clone().unwrap();
        match table.add(items).execute().await {
            Ok(_) => return Ok(()),
            Err(err) => {
                return Err(anyhow!(err.to_string()));
            }
        }
    }

    async fn delete(&self, filter: &str) -> Result<()> {
        let table = self.table.clone().ok_or(anyhow!("Table not initialized"))?;
        table.delete(filter).await?;
        return Ok(());
    }

    async fn optimize(&self, keep_versions: bool) -> Result<()> {
        let table = self.table.clone().ok_or(anyhow!("Table not initialized"))?;

        let older_than = if keep_versions {
            None
        } else {
            Some(Duration::milliseconds(1))
        };

        table
            .optimize(lancedb::table::OptimizeAction::Prune {
                delete_unverified: Some(true),
                error_if_tagged_old_versions: Some(true),
                older_than,
            })
            .await?;

        table
            .optimize(lancedb::table::OptimizeAction::Compact {
                options: CompactionOptions {
                    ..Default::default()
                },
                remap_options: None,
            })
            .await?;

        table
            .optimize(lancedb::table::OptimizeAction::Index(OptimizeOptions {
                ..Default::default()
            }))
            .await?;

        return Ok(());
    }

    async fn list(&self, limit: usize, offset: usize) -> Result<Vec<Value>> {
        let table = self
            .table
            .clone()
            .ok_or_else(|| anyhow!("Table not initialized"))?;

        let result = table
            .query()
            .limit(limit)
            .offset(offset)
            .execute()
            .await
            .ok();

        result.as_ref().ok_or(anyhow!("Error executing query"))?;

        let result = result.unwrap().try_collect::<Vec<_>>().await.ok();
        return record_batches_to_vec(result);
    }

    async fn index(&self, column: &str, fts: bool) -> Result<()> {
        let table = self.table.clone().ok_or(anyhow!("Table not initialized"))?;
        if fts {
            table
                .create_index(&[column], Index::FTS(FtsIndexBuilder::default()))
                .execute()
                .await?;
            return Ok(());
        }

        table.create_index(&[column], Index::Auto).execute().await?;
        Ok(())
    }

    async fn purge(&self) -> Result<()> {
        let table = self.table.clone().ok_or(anyhow!("Table not initialized"))?;
        table.delete("*").await?;
        Ok(())
    }

    async fn count(&self, filter: Option<String>) -> Result<usize> {
        let table = self.table.clone().ok_or(anyhow!("Table not initialized"))?;
        Ok(table.count_rows(filter).await?)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use flow_like_types::{json::{from_value, to_value}, create_id, tokio};
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
    struct TestStruct {
        id: i32,
        name: String,
        vector: Vec<f32>,
    }

    #[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
    struct TestStruct2 {
        id: i32,
        name: String,
    }

    #[tokio::test]
    async fn test_lance_ingest() -> Result<()> {
        let test_path = format!("./tmp/{}", create_id());
        std::fs::create_dir_all(&test_path).unwrap();
        let mut db = LanceDBVectorStore::new(PathBuf::from(&test_path), "t".to_string())
            .await
            .ok_or(anyhow!("Error creating LanceDB"))?;
        let records = vec![
            TestStruct {
                id: 1,
                name: "Alice".to_string(),
                vector: vec![1.0, 2.0, 3.0],
            },
            TestStruct {
                id: 2,
                name: "Bob".to_string(),
                vector: vec![2.0, 3.0, 4.0],
            },
        ];

        let json_records: Vec<Value> = records
            .into_iter()
            .map(to_value)
            .collect::<Result<_, _>>()?;

        db.upsert(json_records, "id".to_string()).await?;

        std::fs::remove_dir_all(&test_path).unwrap();

        Ok(())
    }

    #[tokio::test]
    async fn test_lance_search_first() -> Result<()> {
        let test_path = format!("./tmp/{}", create_id());
        std::fs::create_dir_all(&test_path).unwrap();
        let mut db = LanceDBVectorStore::new(PathBuf::from(&test_path), "t".to_string())
            .await
            .ok_or(anyhow!("Error creating LanceDB"))?;
        let records = vec![
            TestStruct {
                id: 1,
                name: "Alice".to_string(),
                vector: vec![1.0, 2.0, 3.0],
            },
            TestStruct {
                id: 2,
                name: "Bob".to_string(),
                vector: vec![2.0, 3.0, 4.0],
            },
        ];

        let json_records: Vec<Value> = records
            .clone()
            .into_iter()
            .map(to_value)
            .collect::<Result<_, _>>()?;

        db.upsert(json_records, "id".to_string()).await?;

        let search_results: Vec<Value> = db.vector_search(vec![1.0, 2.0, 3.0], None, 10, 0).await?;

        assert!(!search_results.is_empty());

        let first_item: TestStruct = from_value(search_results[0].clone())?;

        assert_eq!(first_item, records[0]);

        std::fs::remove_dir_all(&test_path).unwrap();

        Ok(())
    }

    #[tokio::test]
    async fn test_lance_search_fts() -> Result<()> {
        let test_path = format!("./tmp/{}", create_id());
        std::fs::create_dir_all(&test_path).unwrap();
        let mut db = LanceDBVectorStore::new(PathBuf::from(&test_path), "t".to_string())
            .await
            .ok_or(anyhow!("Error creating LanceDB"))?;
        let records = vec![
            TestStruct {
                id: 1,
                name: "Alice".to_string(),
                vector: vec![1.0, 2.0, 3.0],
            },
            TestStruct {
                id: 2,
                name: "Bob".to_string(),
                vector: vec![2.0, 3.0, 4.0],
            },
        ];

        let json_records: Vec<Value> = records
            .clone()
            .into_iter()
            .map(to_value)
            .collect::<Result<_, _>>()?;

        db.upsert(json_records, "id".to_string()).await?;
        db.index("name", true).await?;

        let search_results: Vec<Value> = db.fts_search("Alice", None, 10, 0).await?;

        assert!(!search_results.is_empty());

        let first_item: TestStruct = from_value(search_results[0].clone())?;

        assert_eq!(first_item, records[0]);

        std::fs::remove_dir_all(&test_path).unwrap();

        Ok(())
    }

    #[tokio::test]
    async fn test_lance_search_second() -> Result<()> {
        let test_path = format!("./tmp/{}", create_id());
        std::fs::create_dir_all(&test_path).unwrap();
        let mut db = LanceDBVectorStore::new(PathBuf::from(&test_path), "t".to_string())
            .await
            .ok_or(anyhow!("Error creating LanceDB"))?;
        let records = vec![
            TestStruct {
                id: 1,
                name: "Alice".to_string(),
                vector: vec![1.0, 2.0, 3.0],
            },
            TestStruct {
                id: 2,
                name: "Bob".to_string(),
                vector: vec![2.0, 3.0, 4.0],
            },
        ];

        let json_records: Vec<Value> = records
            .clone()
            .into_iter()
            .map(to_value)
            .collect::<Result<_, _>>()?;

        db.upsert(json_records, "id".to_string()).await?;

        let search_results: Vec<Value> = db.vector_search(vec![2.0, 3.0, 4.0], None, 10, 0).await?;

        assert!(!search_results.is_empty());

        let first_item: TestStruct = from_value(search_results[0].clone())?;

        assert_eq!(first_item, records[1]);

        std::fs::remove_dir_all(&test_path).unwrap();

        Ok(())
    }

    #[tokio::test]
    async fn test_lance_search_filter() -> Result<()> {
        let test_path = format!("./tmp/{}", create_id());
        std::fs::create_dir_all(&test_path).unwrap();
        let mut db = LanceDBVectorStore::new(PathBuf::from(&test_path), "t".to_string())
            .await
            .ok_or(anyhow!("Error creating LanceDB"))?;
        let records = vec![
            TestStruct {
                id: 1,
                name: "Alice".to_string(),
                vector: vec![1.0, 2.0, 3.0],
            },
            TestStruct {
                id: 2,
                name: "Bob".to_string(),
                vector: vec![2.0, 3.0, 4.0],
            },
        ];

        let json_records: Vec<Value> = records
            .clone()
            .into_iter()
            .map(to_value)
            .collect::<Result<_, _>>()?;

        db.upsert(json_records, "id".to_string()).await?;

        let search_results: Vec<Value> = db
            .vector_search(vec![1.0, 2.0, 3.0], Some("id = 2"), 10, 0)
            .await?;

        assert!(!search_results.is_empty());

        let first_item: TestStruct = from_value(search_results[0].clone())?;

        assert_eq!(first_item, records[1]);

        std::fs::remove_dir_all(&test_path).unwrap();

        Ok(())
    }

    #[tokio::test]
    async fn test_lance_no_vec() -> Result<()> {
        let test_path = format!("./tmp/{}", create_id());
        std::fs::create_dir_all(&test_path).unwrap();
        let mut db = LanceDBVectorStore::new(PathBuf::from(&test_path), "t".to_string())
            .await
            .ok_or(anyhow!("Error creating LanceDB"))?;
        let records = vec![
            TestStruct2 {
                id: 1,
                name: "Alice".to_string(),
            },
            TestStruct2 {
                id: 2,
                name: "Bob".to_string(),
            },
        ];

        let json_records: Vec<Value> = records
            .clone()
            .into_iter()
            .map(to_value)
            .collect::<Result<_, _>>()?;

        db.upsert(json_records, "id".to_string()).await?;

        let count = db.count(None).await?;

        assert_eq!(count, 2);

        std::fs::remove_dir_all(&test_path).unwrap();

        Ok(())
    }

    #[tokio::test]
    async fn test_casting() -> Result<()> {
        let test_path = format!("./tmp/{}", create_id());
        std::fs::create_dir_all(&test_path).unwrap();
        let db = LanceDBVectorStore::new(PathBuf::from(&test_path), "t".to_string())
            .await
            .ok_or(anyhow!("Error creating LanceDB"))
            .unwrap();
        let cacheable: Arc<dyn Cacheable> = Arc::new(db.clone());
        let resolved = cacheable
            .as_any()
            .downcast_ref::<LanceDBVectorStore>()
            .unwrap();
        let resolved = resolved.clone();
        assert_eq!(resolved.connection.uri(), db.connection.uri());

        Ok(())
    }
}
