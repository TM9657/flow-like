use crate::{entity::profile, error::ApiError, middleware::jwt::AppUser, state::AppState};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use flow_like::profile::{ProfileApp, Settings};
use flow_like_types::{Value, create_id};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, sqlx::types::chrono,
};
use serde::{Deserialize, Serialize};
use serde_json::to_value;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProfileBody {
    name: Option<String>,
    description: Option<String>,
    interests: Option<Vec<String>>,
    tags: Option<Vec<String>>,
    theme: Option<Value>,
    bit_ids: Option<Vec<String>>,
    apps: Option<Vec<ProfileApp>>,
    hubs: Option<Vec<String>>,
    settings: Option<Settings>,
}

#[tracing::instrument(name = "POST /profile/{profile_id}", skip(state, user, profile_body))]
pub async fn upsert_profile(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(profile_id): Path<String>,
    Json(profile_body): Json<ProfileBody>,
) -> Result<Json<profile::Model>, ApiError> {
    let sub = user.sub()?;
    let found_profile = profile::Entity::find()
        .filter(
            profile::Column::Id
                .eq(&profile_id)
                .and(profile::Column::UserId.eq(&sub)),
        )
        .one(&state.db)
        .await?;

    if let Some(found_profile) = found_profile {
        let mut active_model: profile::ActiveModel = found_profile.into();
        if let Some(name) = profile_body.name {
            active_model.name = Set(name);
        }
        if let Some(description) = profile_body.description {
            active_model.description = Set(Some(description));
        }
        if let Some(interests) = profile_body.interests {
            active_model.interests = Set(Some(interests));
        }
        if let Some(tags) = profile_body.tags {
            active_model.tags = Set(Some(tags));
        }
        if let Some(theme) = profile_body.theme {
            active_model.theme = Set(Some(theme));
        }

        if let Some(bit_ids) = profile_body.bit_ids {
            active_model.bit_ids = Set(Some(bit_ids));
        }

        if let Some(apps) = profile_body.apps {
            let apps: Vec<Value> = apps.iter().map(|v| to_value(v).unwrap()).collect();
            let apps: Value = Value::Array(apps);
            active_model.apps = Set(Some(apps));
        }

        if let Some(settings) = profile_body.settings {
            let settings = to_value(&settings)?;
            active_model.settings = Set(Some(settings));
        }

        if let Some(hubs) = profile_body.hubs {
            active_model.hubs = Set(Some(hubs));
        }

        active_model.updated_at = Set(chrono::Utc::now().naive_utc());

        let updated_profile = active_model.update(&state.db).await?;
        return Ok(Json(updated_profile));
    }

    let id = create_id();

    let apps = if let Some(apps) = profile_body.apps {
        let apps: Vec<Value> = apps.iter().map(|v| to_value(v).unwrap()).collect();
        Some(apps)
    } else {
        None
    };

    let settings = if let Some(settings) = profile_body.settings {
        Some(to_value(&settings)?)
    } else {
        None
    };

    let apps = apps.map(Value::Array);

    let new_profile = profile::ActiveModel {
        id: Set(id),
        user_id: Set(sub),
        name: Set(profile_body.name.unwrap_or_default()),
        description: Set(profile_body.description),
        interests: Set(profile_body.interests),
        tags: Set(profile_body.tags),
        theme: Set(profile_body.theme),
        bit_ids: Set(profile_body.bit_ids),
        apps: Set(apps),
        settings: Set(settings),
        hubs: Set(profile_body.hubs),
        created_at: Set(chrono::Utc::now().naive_utc()),
        updated_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    };

    let created_profile = new_profile.insert(&state.db).await?;
    Ok(Json(created_profile))
}
