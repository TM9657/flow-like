use std::sync::Arc;

use crate::{
    ensure_permission,
    error::ApiError,
    middleware::jwt::AppUser,
    permission::role_permission::RolePermissions,
    routes::{LanguageParams, PaginationParams},
    state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use flow_like_storage::{
    databases::vector::{
        VectorStore,
        lancedb::{LanceDBVectorStore, record_batches_to_vec},
    },
    datafusion::{self, prelude::SessionContext},
};
use flow_like_types::{anyhow, bail};
use futures_util::{StreamExt, TryStreamExt};

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
}

#[tracing::instrument(name = "POST /apps/{app_id}/db/{table}/query", skip(state, user))]
pub async fn query_table(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, table)): Path<(String, String)>,
    Query(params): Query<PaginationParams>,
    Json(payload): Json<QueryTablePayload>,
) -> Result<Json<Vec<flow_like_types::Value>>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::ReadFiles);

    let offset = params.offset.unwrap_or(0) as usize;
    let limit = params.limit.unwrap_or(25).min(250) as usize;

    let credentials = state.master_credentials().await?;
    let connection = credentials.to_db(&app_id).await?.execute().await?;
    let db = LanceDBVectorStore::from_connection(connection, table.clone()).await;

    if let Some(sql) = payload.sql {
        let context = SessionContext::new();
        let fusion = db.to_datafusion().await?;
        context.register_table(table, Arc::new(fusion))?;
        let df = context.sql(&sql).await?;
        let items = df.collect().await?;
        let items = record_batches_to_vec(Some(items))?;
        return Ok(Json(items));
    }

    match (payload.vector_query, payload.fts_term, payload.filter) {
        (Some(vector_query), None, filter) => {
            let filter_str = filter.as_deref();
            let items = db
                .vector_search(vector_query.vector, filter_str, limit, offset)
                .await?;
            return Ok(Json(items));
        }
        (None, Some(fts_term), filter) => {
            let filter_str = filter.as_deref();
            let items = db.fts_search(&fts_term, filter_str, limit, offset).await?;
            return Ok(Json(items));
        }
        (Some(vector_query), Some(fts_term), filter) => {
            let filter_str = filter.as_deref();
            let items = db
                .hybrid_search(
                    vector_query.vector,
                    &fts_term,
                    filter_str,
                    limit,
                    offset,
                    payload.rerank.unwrap_or(true),
                )
                .await?;
            return Ok(Json(items));
        }
        (None, None, Some(filter)) => {
            let items = db.filter(&filter, limit, offset).await?;
            return Ok(Json(items));
        }
        _ => {
            return Err(ApiError::BadRequest(
                "No valid query parameters provided".to_string(),
            ));
        }
    }
}
