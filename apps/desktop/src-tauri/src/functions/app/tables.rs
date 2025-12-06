use std::sync::Arc;

use anyhow::anyhow;
use flow_like::{
    credentials::SharedCredentials,
    flow_like_storage::{
        Path,
        arrow_schema::Schema,
        databases::vector::{
            VectorStore,
            lancedb::{IndexConfigDto, LanceDBVectorStore, record_batches_to_vec},
        },
        datafusion::prelude::SessionContext,
    },
};
use tauri::AppHandle;

use crate::{functions::TauriFunctionError, state::TauriFlowLikeState};

async fn db_connection(
    app_handle: &AppHandle,
    app_id: String,
    table_name: Option<String>,
    credentials: Option<Arc<SharedCredentials>>,
) -> flow_like_types::Result<LanceDBVectorStore> {
    let flow_like_state = TauriFlowLikeState::construct(app_handle).await?;
    let table_name = table_name.unwrap_or("default".to_string());
    let board_dir = Path::from("apps")
        .child(app_id.clone())
        .child("storage")
        .child("db");
    let db = if let Some(credentials) = &credentials {
        credentials.to_db(&app_id).await?
    } else {
        flow_like_state
            .config
            .read()
            .await
            .callbacks
            .build_project_database
            .clone()
            .ok_or(flow_like_types::anyhow!("No database builder found"))?(board_dir)
    };

    let db = db.execute().await?;
    let db = LanceDBVectorStore::from_connection(db, table_name).await;
    Ok(db)
}

#[tauri::command(async)]
pub async fn db_table_names(
    app_handle: AppHandle,
    app_id: String,
    table_name: Option<String>,
    credentials: Option<Arc<SharedCredentials>>,
) -> Result<Vec<String>, TauriFunctionError> {
    let db = db_connection(&app_handle, app_id, table_name, credentials).await?;
    let table_names = db.list_tables().await?;
    Ok(table_names)
}

#[tauri::command(async)]
pub async fn db_count(
    app_handle: AppHandle,
    app_id: String,
    table_name: Option<String>,
    credentials: Option<Arc<SharedCredentials>>,
) -> Result<usize, TauriFunctionError> {
    let db = db_connection(&app_handle, app_id, table_name, credentials).await?;
    let cnt = db.count(None).await?;
    Ok(cnt)
}

#[tauri::command(async)]
pub async fn db_schema(
    app_handle: AppHandle,
    app_id: String,
    table_name: String,
    credentials: Option<Arc<SharedCredentials>>,
) -> Result<Schema, TauriFunctionError> {
    let db = db_connection(&app_handle, app_id, Some(table_name), credentials).await?;
    let schema = db.schema().await?;
    Ok(schema)
}

#[tauri::command(async)]
pub async fn db_list(
    app_handle: AppHandle,
    app_id: String,
    table_name: String,
    credentials: Option<Arc<SharedCredentials>>,
    limit: Option<u64>,
    offset: Option<u64>,
) -> Result<Vec<flow_like_types::Value>, TauriFunctionError> {
    let db = db_connection(&app_handle, app_id, Some(table_name), credentials).await?;
    let limit = limit.unwrap_or(100).min(250) as usize;
    let offset = offset.unwrap_or(0) as usize;
    let items = db.list(None, limit, offset).await?;
    Ok(items)
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct VectorQueryPayload {
    pub column: String,
    pub vector: Vec<f64>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct QueryTablePayload {
    sql: Option<String>,
    vector_query: Option<VectorQueryPayload>,
    filter: Option<String>,
    fts_term: Option<String>,
    rerank: Option<bool>,
    select: Option<Vec<String>>,
}

#[tauri::command(async)]
pub async fn db_query(
    app_handle: AppHandle,
    app_id: String,
    table_name: String,
    credentials: Option<Arc<SharedCredentials>>,
    limit: Option<u64>,
    offset: Option<u64>,
    payload: QueryTablePayload,
) -> Result<Vec<flow_like_types::Value>, TauriFunctionError> {
    let db = db_connection(&app_handle, app_id, Some(table_name.clone()), credentials).await?;
    let limit = limit.unwrap_or(100).min(250) as usize;
    let offset = offset.unwrap_or(0) as usize;
    if let Some(sql) = payload.sql {
        let context = SessionContext::new();
        let fusion = db.to_datafusion().await?;
        context
            .register_table(table_name, Arc::new(fusion))
            .map_err(|e| anyhow!(e))?;
        let df = context.sql(&sql).await.map_err(|e| anyhow!(e))?;
        let items = df.collect().await.map_err(|e| anyhow!(e))?;
        let items = record_batches_to_vec(Some(items))?;
        return Ok(items);
    }

    match (payload.vector_query, payload.fts_term, payload.filter) {
        (Some(vector_query), None, filter) => {
            let filter_str = filter.as_deref();
            let items = db
                .vector_search(
                    vector_query.vector,
                    filter_str,
                    payload.select,
                    limit,
                    offset,
                )
                .await?;
            Ok(items)
        }
        (None, Some(fts_term), filter) => {
            let filter_str = filter.as_deref();
            let items = db
                .fts_search(&fts_term, filter_str, payload.select, limit, offset)
                .await?;
            Ok(items)
        }
        (Some(vector_query), Some(fts_term), filter) => {
            let filter_str = filter.as_deref();
            let items = db
                .hybrid_search(
                    vector_query.vector,
                    &fts_term,
                    filter_str,
                    payload.select,
                    limit,
                    offset,
                    payload.rerank.unwrap_or(true),
                )
                .await?;
            Ok(items)
        }
        (None, None, Some(filter)) => {
            let items = db.filter(&filter, payload.select, limit, offset).await?;
            Ok(items)
        }
        _ => Err(anyhow::anyhow!("No query parameters provided").into()),
    }
}

#[tauri::command(async)]
pub async fn db_indices(
    app_handle: AppHandle,
    app_id: String,
    table_name: String,
    credentials: Option<Arc<SharedCredentials>>,
) -> Result<Vec<IndexConfigDto>, TauriFunctionError> {
    let db = db_connection(&app_handle, app_id, Some(table_name), credentials).await?;
    let indices = db.list_indices().await?;
    Ok(indices)
}

#[tauri::command(async)]
pub async fn db_delete(
    app_handle: AppHandle,
    app_id: String,
    table_name: String,
    credentials: Option<Arc<SharedCredentials>>,
    query: String,
) -> Result<(), TauriFunctionError> {
    let db = db_connection(&app_handle, app_id, Some(table_name), credentials).await?;
    db.delete(&query).await?;
    Ok(())
}

#[tauri::command(async)]
pub async fn db_add(
    app_handle: AppHandle,
    app_id: String,
    table_name: String,
    credentials: Option<Arc<SharedCredentials>>,
    items: Vec<flow_like_types::Value>,
) -> Result<(), TauriFunctionError> {
    let mut db = db_connection(&app_handle, app_id, Some(table_name), credentials).await?;
    db.insert(items).await?;
    Ok(())
}

#[tauri::command(async)]
pub async fn build_index(
    app_handle: AppHandle,
    app_id: String,
    table_name: String,
    credentials: Option<Arc<SharedCredentials>>,
    column: String,
    index_type: String,
    _optimize: Option<bool>,
) -> Result<(), TauriFunctionError> {
    let db = db_connection(&app_handle, app_id, Some(table_name), credentials).await?;
    db.index(&column, Some(&index_type)).await?;
    Ok(())
}
