use crate::{
    credentials::CredentialsAccess, error::ApiError, middleware::jwt::AppUser,
    permission::role_permission::RolePermissions, state::AppState,
};
use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use flow_like::{
    app_build::{AppBuildStore, AppBuildStoreError, MAX_APP_BUILD_BYTES},
    flow_like_storage::files::store::FlowLikeStore,
    state::FlowLikeState,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use utoipa::ToSchema;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/{build_id}", get(read_app_build).put(write_app_build))
        // Escaped JSON can be larger on the wire than the compact stored form.
        .layer(axum::extract::DefaultBodyLimit::max(
            MAX_APP_BUILD_BYTES
                .saturating_mul(6)
                .saturating_add(64 * 1024),
        ))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct WriteAppBuildRequest {
    #[schema(value_type = Object)]
    pub record: Value,
    /// `null` creates revision zero. A number updates the matching revision to exactly `n + 1`.
    pub expected_revision: Option<u64>,
}

#[derive(Debug)]
pub(crate) enum AppBuildRouteError {
    Api(ApiError),
    Store(AppBuildStoreError),
}

impl From<ApiError> for AppBuildRouteError {
    fn from(error: ApiError) -> Self {
        Self::Api(error)
    }
}

impl From<flow_like_types::Error> for AppBuildRouteError {
    fn from(error: flow_like_types::Error) -> Self {
        Self::Api(error.into())
    }
}

impl From<AppBuildStoreError> for AppBuildRouteError {
    fn from(error: AppBuildStoreError) -> Self {
        Self::Store(error)
    }
}

impl IntoResponse for AppBuildRouteError {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct ErrorEnvelope<T> {
            error: T,
        }

        #[derive(Serialize)]
        struct ConflictBody {
            code: &'static str,
            message: &'static str,
            expected_revision: Option<u64>,
            current_revision: Option<u64>,
            current: Option<Value>,
        }

        #[derive(Serialize)]
        struct BasicBody {
            code: &'static str,
            message: String,
        }

        match self {
            Self::Api(error) => error.into_response(),
            Self::Store(AppBuildStoreError::Conflict {
                expected_revision,
                current_revision,
                current,
            }) => (
                StatusCode::CONFLICT,
                Json(ErrorEnvelope {
                    error: ConflictBody {
                        code: "APP_BUILD_REVISION_CONFLICT",
                        message: "The app build changed; re-read it and retry from the current revision.",
                        expected_revision,
                        current_revision,
                        current,
                    },
                }),
            )
                .into_response(),
            Self::Store(
                error @ (AppBuildStoreError::InvalidId { .. }
                | AppBuildStoreError::InvalidRecord { .. }),
            ) => ApiError::bad_request(error.to_string()).into_response(),
            Self::Store(error @ AppBuildStoreError::TooLarge { .. }) => (
                StatusCode::PAYLOAD_TOO_LARGE,
                Json(ErrorEnvelope {
                    error: BasicBody {
                        code: "APP_BUILD_TOO_LARGE",
                        message: error.to_string(),
                    },
                }),
            )
                .into_response(),
            Self::Store(error @ AppBuildStoreError::ConditionalWriteUnsupported { .. }) => {
                ApiError::service_unavailable(error.to_string()).into_response()
            }
            Self::Store(
                error @ (AppBuildStoreError::CorruptStoredRecord { .. }
                | AppBuildStoreError::Storage { .. }),
            ) => ApiError::internal(error.to_string()).into_response(),
        }
    }
}

async fn scoped_store(
    state: &AppState,
    user: &AppUser,
    app_id: &str,
    permission: RolePermissions,
    access: CredentialsAccess,
) -> Result<AppBuildStore, AppBuildRouteError> {
    let app_permission = user.app_permission(app_id, state).await?;
    if !app_permission.has_permission(permission) {
        if let Ok(user_id) = app_permission.sub() {
            state.invalidate_permission(&user_id, app_id);
        }
        return Err(ApiError::FORBIDDEN.into());
    }
    let sub = app_permission.sub()?;
    let credentials = state.scoped_credentials(&sub, app_id, access).await?;
    let flow_state = Arc::new(credentials.to_state(state.clone()).await?);
    let store = FlowLikeState::project_meta_store(&flow_state).await?;
    Ok(match store {
        FlowLikeStore::Local(store) => AppBuildStore::new_local(store, app_id)?,
        store => AppBuildStore::new(store.as_generic(), app_id)?,
    })
}

#[utoipa::path(
    get,
    path = "/apps/{app_id}/flowpilot-builds/{build_id}",
    tag = "flowpilot-builds",
    params(
        ("app_id" = String, Path, description = "Application ID"),
        ("build_id" = String, Path, description = "FlowPilot build ID")
    ),
    responses(
        (status = 200, description = "Stored app-build checkpoint", body = Object),
        (status = 404, description = "Build not found"),
        (status = 403, description = "Forbidden")
    ),
    security(("bearer_auth" = []), ("pat" = []))
)]
#[tracing::instrument(
    name = "GET /apps/{app_id}/flowpilot-builds/{build_id}",
    skip(state, user)
)]
pub async fn read_app_build(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, build_id)): Path<(String, String)>,
) -> Result<Json<Value>, AppBuildRouteError> {
    let store = scoped_store(
        &state,
        &user,
        &app_id,
        RolePermissions::ReadBoards,
        CredentialsAccess::ReadApp,
    )
    .await?;
    let record = store
        .read(&build_id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("FlowPilot build {build_id} was not found")))?;
    Ok(Json(record))
}

#[utoipa::path(
    put,
    path = "/apps/{app_id}/flowpilot-builds/{build_id}",
    tag = "flowpilot-builds",
    params(
        ("app_id" = String, Path, description = "Application ID"),
        ("build_id" = String, Path, description = "FlowPilot build ID")
    ),
    request_body = WriteAppBuildRequest,
    responses(
        (status = 200, description = "Stored app-build checkpoint", body = Object),
        (status = 400, description = "Invalid envelope or revision transition"),
        (status = 403, description = "Forbidden"),
        (status = 409, description = "Revision conflict; response includes the current checkpoint"),
        (status = 413, description = "Checkpoint exceeds the size limit")
    ),
    security(("bearer_auth" = []), ("pat" = []))
)]
#[tracing::instrument(
    name = "PUT /apps/{app_id}/flowpilot-builds/{build_id}",
    skip(state, user, request)
)]
pub async fn write_app_build(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, build_id)): Path<(String, String)>,
    Json(request): Json<WriteAppBuildRequest>,
) -> Result<Json<Value>, AppBuildRouteError> {
    let store = scoped_store(
        &state,
        &user,
        &app_id,
        RolePermissions::WriteBoards,
        CredentialsAccess::EditApp,
    )
    .await?;
    let stored = match request.expected_revision {
        Some(expected_revision) => {
            store
                .compare_and_swap(&build_id, expected_revision, request.record)
                .await?
        }
        None => store.create(&build_id, request.record).await?,
    };
    Ok(Json(stored))
}
