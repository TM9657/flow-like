use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use flow_like::bit::{Bit, BitTypes, ForecastingAssetRef};
use flow_like_catalog_ml::inspection::forecasting::studio::{
    self, StudioForecastRequest, StudioForecastResult,
};
use flow_like_storage::databases::workbench::{WorkbenchSurface, saved_query::SavedQuerySurface};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::Deserialize;

use super::{ScopeParams, column_reader, resolve_connection};
use crate::{
    ensure_any_permission,
    entity::{bit, profile, user_bit},
    error::ApiError,
    middleware::jwt::AppUser,
    permission::role_permission::RolePermissions,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct ForecastQueryPayload {
    pub profile_id: String,
    #[serde(flatten)]
    pub request: StudioForecastRequest,
}

fn activated_reference(
    references: &[String],
    model_id: &str,
    default_hub: &str,
) -> Result<ForecastingAssetRef, ApiError> {
    let matching = references
        .iter()
        .filter(|reference| {
            reference
                .rsplit_once(':')
                .map_or(reference.as_str(), |(_, id)| id)
                == model_id
        })
        .collect::<Vec<_>>();
    let first = matching.first().ok_or_else(|| {
        ApiError::forbidden("Add this forecasting model to the selected profile before running it")
    })?;
    let selected = ForecastingAssetRef {
        bit: (*first).clone(),
        relative_path: String::new(),
    };
    let (hub, id) = selected
        .reference_parts(default_hub)
        .map_err(|_| ApiError::bad_request("The activated model has an invalid Bit reference"))?;
    let identity = Bit {
        id: id.into(),
        hub: hub.into(),
        ..Default::default()
    };
    for reference in matching {
        let candidate = ForecastingAssetRef {
            bit: reference.clone(),
            relative_path: String::new(),
        };
        if !candidate
            .matches_reference(&identity, default_hub)
            .unwrap_or(false)
        {
            return Err(ApiError::bad_request(
                "This model ID is activated from more than one hub; remove the duplicate before forecasting",
            ));
        }
    }
    Ok(selected)
}

async fn profile_model_reference(
    db: &DatabaseConnection,
    sub: &str,
    profile_id: &str,
    model_id: &str,
) -> Result<(ForecastingAssetRef, String), ApiError> {
    if profile_id.trim().is_empty() || profile_id.trim() != profile_id || profile_id.len() > 256 {
        return Err(ApiError::bad_request(
            "A valid profile_id is required for forecasting",
        ));
    }
    let profile = profile::Entity::find()
        .filter(profile::Column::Id.eq(profile_id))
        .filter(profile::Column::UserId.eq(sub))
        .filter(profile::Column::DeletedAt.is_null())
        .one(db)
        .await?
        .ok_or_else(|| ApiError::not_found("Profile not found"))?;
    let reference =
        activated_reference(&profile.bit_ids.unwrap_or_default(), model_id, &profile.hub)?;
    Ok((reference, profile.hub))
}

async fn authorized_model(
    state: &AppState,
    sub: &str,
    payload: &ForecastQueryPayload,
) -> Result<Bit, ApiError> {
    let (reference, profile_hub) = profile_model_reference(
        &state.db,
        sub,
        &payload.profile_id,
        &payload.request.model_bit_id,
    )
    .await?;
    let private = user_bit::Entity::find_by_id(&payload.request.model_bit_id)
        .filter(user_bit::Column::UserId.eq(sub))
        .one(&state.db)
        .await?;
    let resolved = if let Some(private) = private {
        crate::routes::user::bits::user_bit_to_core(private, state, false)
    } else {
        let published = bit::Entity::find_by_id(&payload.request.model_bit_id)
            .one(&state.db)
            .await?
            .ok_or_else(|| {
                ApiError::not_found(
                    "Forecasting model is not available in your library or this catalog",
                )
            })?;
        let published: Bit = published.into();
        if state.platform_config.features.unauthorized_read {
            published
        } else {
            crate::routes::bit::get_bit::temporary_bit(published, &state.cdn_bucket).await?
        }
    };
    if resolved.bit_type != BitTypes::Forecasting {
        return Err(ApiError::bad_request("Select a Forecasting Bit"));
    }
    if !reference
        .matches_reference(&resolved, &profile_hub)
        .unwrap_or(false)
    {
        return Err(ApiError::forbidden(
            "The forecasting model does not match the hub activated in this profile",
        ));
    }
    Ok(resolved)
}

#[utoipa::path(
    post,
    path = "/apps/{app_id}/db/queries/forecast",
    tag = "data-studio",
    description = "Forecast the result of read-only SQL over native app tables and saved views. Requires an activated Forecasting Bit in the caller's profile. Returns history, future points, quantiles, covariates and optional holdout metrics without writing tables.",
    params(
        ("app_id" = String, Path, description = "Application ID"),
        ("scope" = Option<String>, Query, description = "Scope: 'user' or omit for project")
    ),
    request_body = serde_json::Value,
    responses(
        (status = 200, description = "Forecast points and metadata"),
        (status = 400, description = "Invalid mapping or forecast request"),
        (status = 403, description = "App database or model access denied"),
        (status = 503, description = "Native forecasting is unavailable in this API build")
    ),
    security(("bearer_auth" = []), ("api_key" = []), ("pat" = []))
)]
pub async fn forecast_query(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(app_id): Path<String>,
    Query(scope): Query<ScopeParams>,
    Json(payload): Json<ForecastQueryPayload>,
) -> Result<Json<StudioForecastResult>, ApiError> {
    let permission = ensure_any_permission!(
        user,
        &app_id,
        &state,
        RolePermissions::ReadFiles,
        RolePermissions::ReadDatabase
    );
    if scope
        .scope
        .as_deref()
        .is_some_and(|scope| scope != "user" && scope != "project")
    {
        return Err(ApiError::bad_request("scope must be 'user' or 'project'"));
    }
    let bit = authorized_model(&state, &user.sub()?, &payload).await?;
    if !studio::inference_available() {
        return Err(ApiError::service_unavailable(
            "Native forecasting is unavailable in this API build. Use a server built with the forecasting feature.",
        ));
    }
    let connection = resolve_connection(&state, &user, &app_id, &scope).await?;
    let views =
        super::saved_queries::collect_views(&connection, SavedQuerySurface::Native, None).await?;
    let flow_state = state.master_state(&state).await?;
    let result = studio::run_forecast(
        flow_state,
        bit,
        &connection,
        WorkbenchSurface::Native,
        views,
        &column_reader(&permission, &scope),
        payload.request,
    )
    .await
    .map_err(|error| {
        if let Some(message) = studio::input_error_message(&error) {
            ApiError::bad_request(message)
        } else {
            super::table_input_error(error)
        }
    })?;
    Ok(Json(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{
        ActiveValue::Set, ConnectOptions, ConnectionTrait, Database, Schema, sea_query::Table,
    };

    #[test]
    fn model_activation_checks_ids_and_hubs_without_accepting_client_bits() {
        let references = vec!["hub.example:model".into()];
        assert_eq!(
            activated_reference(&references, "other", "hub.example")
                .unwrap_err()
                .status(),
            axum::http::StatusCode::FORBIDDEN
        );
        assert!(
            activated_reference(
                &["hub.example:model".into(), "other.example:model".into()],
                "model",
                "hub.example"
            )
            .is_err()
        );
        let reference = activated_reference(
            &[
                "hub.example:model".into(),
                "https://hub.example:model".into(),
            ],
            "model",
            "hub.example",
        )
        .unwrap();
        assert!(
            reference
                .matches_reference(
                    &Bit {
                        id: "model".into(),
                        hub: "hub.example".into(),
                        ..Default::default()
                    },
                    "hub.example"
                )
                .unwrap()
        );
    }

    #[tokio::test]
    async fn profile_activation_is_owned_current_and_required() {
        let mut options = ConnectOptions::new("sqlite::memory:");
        options.max_connections(1);
        let db = Database::connect(options).await.unwrap();
        db.execute_unprepared("ATTACH DATABASE ':memory:' AS public")
            .await
            .unwrap();
        let schema = Schema::new(db.get_database_backend());
        let source = schema.create_table_from_entity(profile::Entity);
        let mut table = Table::create();
        table.table(source.get_table_name().unwrap().clone());
        for column in source.get_columns() {
            table.col(column.clone());
        }
        db.execute(&table).await.unwrap();
        let now = chrono::Utc::now().fixed_offset();
        for (id, owner, deleted) in [
            ("active", "alice", false),
            ("deleted", "alice", true),
            ("other-owner", "bob", false),
        ] {
            profile::Entity::insert(profile::ActiveModel {
                id: Set(id.into()),
                user_id: Set(owner.into()),
                name: Set(id.into()),
                hub: Set("hub.example".into()),
                bit_ids: Set(Some(vec!["hub.example:model".into()].into())),
                created_at: Set(now),
                updated_at: Set(now),
                deleted_at: Set(deleted.then_some(now)),
                ..Default::default()
            })
            .exec(&db)
            .await
            .unwrap();
        }
        profile_model_reference(&db, "alice", "active", "model")
            .await
            .unwrap();
        for id in ["deleted", "other-owner", "missing"] {
            let error = profile_model_reference(&db, "alice", id, "model")
                .await
                .unwrap_err();
            assert_eq!(error.status(), axum::http::StatusCode::NOT_FOUND);
        }
        let error = profile_model_reference(&db, "alice", "active", "unactivated")
            .await
            .unwrap_err();
        assert_eq!(error.status(), axum::http::StatusCode::FORBIDDEN);
    }
}
