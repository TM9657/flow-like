use crate::{
    audit_branch, ensure_any_permission,
    error::ApiError,
    middleware::jwt::AppUser,
    permission::role_permission::RolePermissions,
    routes::app::db::{
        ScopeParams, column_reader, resolve_write_connection, table_input_error,
        validate_table_name,
    },
    state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use flow_like_storage::contracts::column_metadata::{
    ColumnMetadata, DataClassification, DataSensitivity,
};
use flow_like_storage::contracts::database::DatabaseSelector;
use flow_like_storage::databases::vector::lancedb::LanceDBVectorStore;
use std::collections::BTreeSet;
use utoipa::ToSchema;

/// What a column means and who may read it. Saving replaces all of it: an omitted field
/// is cleared.
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(default)]
pub struct ColumnMetadataPayload {
    /// Human-readable name shown in place of the column name. At most 128 characters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// What the column holds and how to use it. At most 4096 characters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Sensitivity level: public, internal, confidential or restricted.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>, example = "confidential")]
    pub classification: Option<DataClassification>,
    /// Whether the column holds personal data.
    pub pii: bool,
    /// Role attributes allowed to read the column. Empty: everyone who may read the table.
    /// Otherwise only admins and roles with at least one of these attributes see the
    /// column. Only admins may change this list.
    pub read_attributes: Vec<String>,
}

impl From<ColumnMetadataPayload> for ColumnMetadata {
    fn from(payload: ColumnMetadataPayload) -> Self {
        Self {
            display_name: payload.display_name,
            description: payload.description,
            sensitivity: DataSensitivity {
                classification: payload.classification,
                pii: payload.pii,
            },
            read_attributes: payload.read_attributes,
        }
    }
}

impl From<ColumnMetadata> for ColumnMetadataPayload {
    fn from(metadata: ColumnMetadata) -> Self {
        Self {
            display_name: metadata.display_name,
            description: metadata.description,
            classification: metadata.sensitivity.classification,
            pii: metadata.sensitivity.pii,
            read_attributes: metadata.read_attributes,
        }
    }
}

#[utoipa::path(
    put,
    path = "/apps/{app_id}/db/{table}/columns/{column}/metadata",
    tag = "database",
    description = "Describe a column: display name, description, classification, personal-data flag and the role attributes allowed to read it. Replaces the column's previous metadata.",
    params(
        ("app_id" = String, Path, description = "Application ID"),
        ("table" = String, Path, description = "Table name"),
        ("column" = String, Path, description = "Column name"),
        ("scope" = Option<String>, Query, description = "Use 'user' for a user-scoped database")
    ),
    request_body = ColumnMetadataPayload,
    responses(
        (status = 200, description = "The stored metadata, trimmed and de-duplicated", body = ColumnMetadataPayload),
        (status = 400, description = "The column does not exist or a value exceeds its limit"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden, or a non-admin tried to change who may read the column")
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = []),
        ("pat" = [])
    )
)]
#[tracing::instrument(
    name = "PUT /apps/{app_id}/db/{table}/columns/{column}/metadata",
    skip(state, user, scope, payload)
)]
pub async fn set_column_metadata(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, table, column)): Path<(String, String, String)>,
    Query(scope): Query<ScopeParams>,
    Query(selector): Query<DatabaseSelector>,
    Json(payload): Json<ColumnMetadataPayload>,
) -> Result<Json<ColumnMetadataPayload>, ApiError> {
    let permission = ensure_any_permission!(
        user,
        &app_id,
        &state,
        RolePermissions::WriteFiles,
        RolePermissions::WriteDatabase
    );
    validate_table_name(&table)?;
    super::validate_writable_selector(&selector)?;

    let connection = resolve_write_connection(&state, &user, &app_id, &scope).await?;
    let db = LanceDBVectorStore::from_connection_with_selector(connection, table.clone(), selector)
        .await?
        .with_column_reader(column_reader(&permission, &scope));
    let previous = db
        .column_metadata(&column)
        .await
        .map_err(table_input_error)?;
    let stored = db
        .set_column_metadata(&column, payload.into())
        .await
        .map_err(table_input_error)?;

    let readers = |metadata: &ColumnMetadata| {
        metadata
            .read_attributes
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
    };
    let action = if readers(&previous) == readers(&stored) {
        "database.column.metadata"
    } else {
        "database.column.access"
    };
    audit_branch!(
        state,
        user,
        app_id,
        action,
        "DatabaseColumn",
        format!("{table}.{column}"),
        serde_json::json!({
            "classification": stored.sensitivity.classification,
            "pii": stored.sensitivity.pii,
            "read_attributes": stored.read_attributes,
            "user_scoped": scope.is_user_scoped(),
        })
    );

    Ok(Json(stored.into()))
}
