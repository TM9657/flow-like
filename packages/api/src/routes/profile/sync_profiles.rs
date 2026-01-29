use crate::{entity::profile, error::ApiError, middleware::jwt::AppUser, state::AppState};
use axum::{Extension, Json, extract::State};
use flow_like::profile::{ProfileApp, Settings};
use flow_like_types::{Value, create_id};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use serde_json::to_value;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SyncProfileRequest {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub interests: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub theme: Option<Value>,
    pub bit_ids: Option<Vec<String>>,
    pub apps: Option<Vec<ProfileApp>>,
    pub hubs: Option<Vec<String>>,
    pub settings: Option<Settings>,
    #[serde(rename = "createdAt")]
    pub created_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncProfileResponse {
    pub synced: Vec<String>,
    pub created: Vec<SyncedProfile>,
    pub updated: Vec<String>,
    pub skipped: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncedProfile {
    pub local_id: String,
    pub server_id: String,
}

/// Sync multiple profiles from desktop to server
/// For existing profiles (matched by ID), updates if local is newer
/// For new profiles, creates with a server-generated ID and returns the mapping
#[tracing::instrument(name = "POST /profile/sync", skip(state, user, profiles))]
pub async fn sync_profiles(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Json(profiles): Json<Vec<SyncProfileRequest>>,
) -> Result<Json<SyncProfileResponse>, ApiError> {
    let sub = user.sub()?;

    let mut created: Vec<SyncedProfile> = Vec::new();
    let mut updated = Vec::new();
    let mut skipped = Vec::new();

    for profile_req in profiles {
        // Check if profile exists on server
        let found_profile = profile::Entity::find()
            .filter(
                profile::Column::Id
                    .eq(&profile_req.id)
                    .and(profile::Column::UserId.eq(&sub)),
            )
            .one(&state.db)
            .await?;

        if let Some(existing) = found_profile {
            // Update existing profile only if local is newer
            let should_update = if let Some(local_updated) = &profile_req.updated_at {
                // Parse and compare timestamps
                if let Ok(local_time) = chrono::DateTime::parse_from_rfc3339(local_updated) {
                    local_time.naive_utc() > existing.updated_at
                } else {
                    true // If can't parse, update anyway
                }
            } else {
                true // If no timestamp, update
            };

            if should_update {
                let mut active_model: profile::ActiveModel = existing.into();

                active_model.name = Set(profile_req.name.clone());
                active_model.description = Set(profile_req.description.clone());
                active_model.interests = Set(profile_req.interests.clone());
                active_model.tags = Set(profile_req.tags.clone());
                active_model.theme = Set(profile_req.theme.clone());
                active_model.bit_ids = Set(profile_req.bit_ids.clone());

                if let Some(apps) = profile_req.apps {
                    let apps: Vec<Value> = apps.iter().map(|v| to_value(v).unwrap()).collect();
                    active_model.apps = Set(Some(Value::Array(apps)));
                }

                if let Some(settings) = profile_req.settings {
                    let settings = to_value(&settings)?;
                    active_model.settings = Set(Some(settings));
                }

                active_model.hubs = Set(profile_req.hubs.clone());
                active_model.updated_at = Set(chrono::Utc::now().naive_utc());

                active_model.update(&state.db).await?;
                updated.push(profile_req.id.clone());
            }
        } else {
            // Create new profile with SERVER-GENERATED ID (never use client ID)
            let server_id = create_id();

            let apps = if let Some(apps) = profile_req.apps {
                let apps: Vec<Value> = apps.iter().map(|v| to_value(v).unwrap()).collect();
                Some(Value::Array(apps))
            } else {
                None
            };

            let settings = if let Some(settings) = profile_req.settings {
                Some(to_value(&settings)?)
            } else {
                None
            };

            let default_hub = if state.platform_config.domain.is_empty() {
                "api.flow-like.com".to_string()
            } else {
                state.platform_config.domain.clone()
            };

            let new_profile = profile::ActiveModel {
                id: Set(server_id.clone()),
                user_id: Set(sub.clone()),
                name: Set(profile_req.name.clone()),
                description: Set(profile_req.description.clone()),
                interests: Set(profile_req.interests.clone()),
                tags: Set(profile_req.tags.clone()),
                theme: Set(profile_req.theme.clone()),
                bit_ids: Set(profile_req.bit_ids.clone()),
                apps: Set(apps),
                settings: Set(settings),
                hub: Set(default_hub.clone()),
                hubs: Set(profile_req.hubs.or(Some(vec![default_hub]))),
                created_at: Set(chrono::Utc::now().naive_utc()),
                updated_at: Set(chrono::Utc::now().naive_utc()),
                ..Default::default()
            };

            new_profile.insert(&state.db).await?;
            created.push(SyncedProfile {
                local_id: profile_req.id.clone(),
                server_id,
            });
        }
    }

    let synced: Vec<String> = created
        .iter()
        .map(|p| p.server_id.clone())
        .chain(updated.iter().cloned())
        .collect();

    Ok(Json(SyncProfileResponse {
        synced,
        created,
        updated,
        skipped,
    }))
}
