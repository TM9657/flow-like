use crate::{
    audit_branch, credentials::CredentialsAccess, ensure_permission, error::ApiError,
    middleware::jwt::AppUser, permission::role_permission::RolePermissions, state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use flow_like_storage::files::rename::{RenameError, rename_destination, rename_storage_item};
use utoipa::ToSchema;

#[derive(Debug, Clone, serde::Deserialize, ToSchema)]
pub struct RenameFilePayload {
    pub prefix: String,
    pub new_name: String,
    pub is_dir: bool,
}

fn rename_error(error: RenameError) -> ApiError {
    match error {
        RenameError::InvalidName | RenameError::InvalidPath => {
            ApiError::bad_request(error.to_string())
        }
        RenameError::NotFound => ApiError::not_found(error.to_string()),
        RenameError::AlreadyExists | RenameError::SourceChanged => {
            ApiError::conflict(error.to_string())
        }
        RenameError::Incomplete(cause) => {
            let cause = rename_error(*cause);
            let mut message = "Rename did not finish. Copied items may remain under the new name. Refresh before trying again.".to_owned();
            if let Some(reason) = cause.public_message() {
                message.push(' ');
                message.push_str(reason);
            }
            ApiError::coded(cause.status(), "RENAME_INCOMPLETE", message)
        }
        RenameError::Storage(error) => ApiError::internal(error.to_string()),
    }
}

#[utoipa::path(
    post,
    path = "/apps/{app_id}/data/rename",
    tag = "data",
    description = "Rename a file or folder within its current parent folder.",
    params(("app_id" = String, Path, description = "Application ID")),
    request_body = RenameFilePayload,
    responses(
        (status = 200, description = "File or folder renamed", body = ()),
        (status = 400, description = "Invalid path or name"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Source not found"),
        (status = 409, description = "Destination exists or source changed")
    ),
    security(("bearer_auth" = []), ("api_key" = []), ("pat" = []))
)]
#[tracing::instrument(name = "POST /apps/{app_id}/data/rename", skip(state, user, payload))]
pub async fn rename_file(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(app_id): Path<String>,
    Json(payload): Json<RenameFilePayload>,
) -> Result<Json<()>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::WriteFiles);
    rename(&state, &user, &app_id, payload, false).await
}

#[utoipa::path(
    post,
    path = "/apps/{app_id}/data/user/rename",
    tag = "data",
    description = "Rename a private app file or folder within its current parent folder.",
    params(("app_id" = String, Path, description = "Application ID")),
    request_body = RenameFilePayload,
    responses(
        (status = 200, description = "File or folder renamed", body = ()),
        (status = 400, description = "Invalid path or name"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Source not found"),
        (status = 409, description = "Destination exists or source changed")
    ),
    security(("bearer_auth" = []), ("api_key" = []), ("pat" = []))
)]
#[tracing::instrument(
    name = "POST /apps/{app_id}/data/user/rename",
    skip(state, user, payload)
)]
pub async fn rename_user_file(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(app_id): Path<String>,
    Json(payload): Json<RenameFilePayload>,
) -> Result<Json<()>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::WriteFiles);
    rename(&state, &user, &app_id, payload, true).await
}

async fn rename(
    state: &AppState,
    user: &AppUser,
    app_id: &str,
    payload: RenameFilePayload,
    personal: bool,
) -> Result<Json<()>, ApiError> {
    let sub = user.sub()?;
    let (base, source, access, scope) = if personal {
        (
            super::paths::user_upload_base(&sub, app_id),
            super::paths::resolve_user_upload(&sub, app_id, &payload.prefix),
            CredentialsAccess::EditUser,
            "user",
        )
    } else {
        (
            super::paths::app_upload_base(app_id),
            super::paths::resolve_app_upload(app_id, &payload.prefix),
            CredentialsAccess::EditApp,
            "app",
        )
    };
    let destination =
        rename_destination(&base, &source, &payload.new_name).map_err(rename_error)?;
    let credentials = state.scoped_credentials(&sub, app_id, access).await?;
    let project_dir = credentials.to_store(false).await?;
    let result = rename_storage_item(
        project_dir.as_generic().as_ref(),
        &base,
        &source,
        &payload.new_name,
        payload.is_dir,
    )
    .await;

    // Failed deletes can leave both names present, so invalidate either outcome.
    let shared = credentials.into_shared_credentials();
    for path in [
        source.parent().unwrap_or(base),
        source.clone(),
        destination.clone(),
    ] {
        super::list_files::invalidate_listing(&state.cache, &shared, app_id, &sub, scope, &path)
            .await;
    }
    result.map_err(rename_error)?;
    audit_branch!(
        state,
        user,
        app_id,
        "file.rename",
        "Storage",
        app_id,
        serde_json::json!({
            "scope": scope,
            "is_dir": payload.is_dir,
            "source_hash": blake3::hash(source.as_ref().as_bytes()).to_hex().to_string(),
            "destination_hash": blake3::hash(destination.as_ref().as_bytes()).to_hex().to_string(),
        })
    );
    Ok(Json(()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[test]
    fn rename_failures_have_actionable_statuses() {
        for (error, expected) in [
            (RenameError::InvalidName, StatusCode::BAD_REQUEST),
            (RenameError::InvalidPath, StatusCode::BAD_REQUEST),
            (RenameError::NotFound, StatusCode::NOT_FOUND),
            (RenameError::AlreadyExists, StatusCode::CONFLICT),
            (RenameError::SourceChanged, StatusCode::CONFLICT),
        ] {
            let api_error = rename_error(error);
            assert_eq!(api_error.status(), expected);
            assert!(api_error.public_message().is_some());
        }
    }

    #[test]
    fn partial_failures_explain_remaining_copies_without_exposing_provider_details() {
        let conflict = rename_error(RenameError::Incomplete(Box::new(
            RenameError::SourceChanged,
        )));
        assert_eq!(conflict.status(), StatusCode::CONFLICT);
        assert!(
            conflict
                .public_message()
                .unwrap()
                .contains("Copied items may remain")
        );

        let failure = rename_error(RenameError::Incomplete(Box::new(RenameError::Storage(
            flow_like_storage::object_store::Error::Generic {
                store: "provider",
                source: "private provider configuration".into(),
            },
        ))));
        assert_eq!(failure.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let message = failure.public_message().unwrap();
        assert!(message.contains("Copied items may remain"));
        assert!(!message.contains("private provider"));
    }
}
