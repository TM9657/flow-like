use std::time::Duration;

use crate::{entity::profile, error::ApiError, middleware::jwt::AppUser, state::AppState};
use axum::{Extension, Json, extract::State};
use flow_like::profile::{ProfileApp, Settings};
use flow_like_storage::object_store::path::Path;
use flow_like_types::{Value, create_id};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use serde_json::to_value;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SyncProfileRequest {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    /// File extension for icon upload (e.g., "png", "jpg"). If set, server will generate a signed URL.
    pub icon_upload_ext: Option<String>,
    /// File extension for thumbnail upload (e.g., "png", "jpg"). If set, server will generate a signed URL.
    pub thumbnail_upload_ext: Option<String>,
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
    pub updated: Vec<UpdatedProfile>,
    pub skipped: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncedProfile {
    pub local_id: String,
    pub server_id: String,
    /// Signed URL for uploading icon (if requested)
    pub icon_upload_url: Option<String>,
    /// Signed URL for uploading thumbnail (if requested)
    pub thumbnail_upload_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdatedProfile {
    pub id: String,
    /// Signed URL for uploading icon (if requested)
    pub icon_upload_url: Option<String>,
    /// Signed URL for uploading thumbnail (if requested)
    pub thumbnail_upload_url: Option<String>,
}

/// Generate a signed upload URL and return the CUID to store in DB
/// - Upload path: media/profiles/{profile_id}/{cuid}.{ext} (original format)
/// - Final path: media/profiles/{profile_id}/{cuid}.webp (after conversion)
/// - DB stores: just the cuid
async fn generate_upload_url(
    state: &AppState,
    profile_id: &str,
    extension: &str,
) -> Result<(String, String), ApiError> {
    let id = create_id();

    // Upload path: media/profiles/{profile_id}/{id}.{ext}
    let upload_path = Path::from("media")
        .child("profiles")
        .child(profile_id)
        .child(format!("{}.{}", id, extension));

    let signed_url = state
        .cdn_bucket
        .sign("PUT", &upload_path, Duration::from_secs(60 * 60))
        .await?;

    // Return just the cuid to store in DB
    Ok((signed_url.to_string(), id))
}

/// Delete an old profile image from storage
async fn delete_old_image(
    state: &AppState,
    profile_id: &str,
    image_id: &str,
) -> Result<(), ApiError> {
    // Construct path: media/profiles/{profile_id}/{image_id}.webp
    let path = Path::from("media")
        .child("profiles")
        .child(profile_id)
        .child(format!("{}.webp", image_id));

    let store = state.cdn_bucket.as_generic();
    if let Err(e) = store.delete(&path).await {
        tracing::warn!("Failed to delete old profile image: {}", e);
    }

    Ok(())
}

/// Sync multiple profiles from desktop to server
/// For existing profiles (matched by ID), updates if local is newer
/// For new profiles, creates with a server-generated ID and returns the mapping
/// Returns signed URLs for direct S3 upload when icon/thumbnail uploads are requested
#[tracing::instrument(name = "POST /profile/sync", skip(state, user, profiles))]
pub async fn sync_profiles(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Json(profiles): Json<Vec<SyncProfileRequest>>,
) -> Result<Json<SyncProfileResponse>, ApiError> {
    let sub = user.sub()?;

    let mut created: Vec<SyncedProfile> = Vec::new();
    let mut updated: Vec<UpdatedProfile> = Vec::new();
    let skipped = Vec::new();

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
                if let Ok(local_time) = chrono::DateTime::parse_from_rfc3339(local_updated) {
                    local_time.naive_utc() > existing.updated_at
                } else {
                    true
                }
            } else {
                true
            };

            if should_update {
                let mut active_model: profile::ActiveModel = existing.clone().into();

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

                // Handle icon upload request
                let icon_upload_url = if let Some(ext) = &profile_req.icon_upload_ext {
                    // Delete old icon if exists
                    if let Some(old_icon_id) = &existing.icon {
                        delete_old_image(&state, &profile_req.id, old_icon_id).await?;
                    }
                    let (upload_url, image_id) =
                        generate_upload_url(&state, &profile_req.id, ext).await?;
                    active_model.icon = Set(Some(image_id));
                    Some(upload_url)
                } else {
                    None
                };

                // Handle thumbnail upload request
                let thumbnail_upload_url = if let Some(ext) = &profile_req.thumbnail_upload_ext {
                    // Delete old thumbnail if exists
                    if let Some(old_thumb_id) = &existing.thumbnail {
                        delete_old_image(&state, &profile_req.id, old_thumb_id).await?;
                    }
                    let (upload_url, image_id) =
                        generate_upload_url(&state, &profile_req.id, ext).await?;
                    active_model.thumbnail = Set(Some(image_id));
                    Some(upload_url)
                } else {
                    None
                };

                active_model.updated_at = Set(chrono::Utc::now().naive_utc());
                active_model.update(&state.db).await?;

                updated.push(UpdatedProfile {
                    id: profile_req.id.clone(),
                    icon_upload_url,
                    thumbnail_upload_url,
                });
            }
        } else {
            // Create new profile with SERVER-GENERATED ID
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

            // Generate upload URLs if requested (using server_id since that's the profile's ID)
            let (icon_upload_url, icon_id) = if let Some(ext) = &profile_req.icon_upload_ext {
                let (url, id) = generate_upload_url(&state, &server_id, ext).await?;
                (Some(url), Some(id))
            } else {
                (None, None)
            };

            let (thumbnail_upload_url, thumbnail_id) =
                if let Some(ext) = &profile_req.thumbnail_upload_ext {
                    let (url, id) = generate_upload_url(&state, &server_id, ext).await?;
                    (Some(url), Some(id))
                } else {
                    (None, None)
                };

            let new_profile = profile::ActiveModel {
                id: Set(server_id.clone()),
                user_id: Set(sub.clone()),
                name: Set(profile_req.name.clone()),
                description: Set(profile_req.description.clone()),
                icon: Set(icon_id),
                thumbnail: Set(thumbnail_id),
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
                icon_upload_url,
                thumbnail_upload_url,
            });
        }
    }

    let synced: Vec<String> = created
        .iter()
        .map(|p| p.server_id.clone())
        .chain(updated.iter().map(|p| p.id.clone()))
        .collect();

    Ok(Json(SyncProfileResponse {
        synced,
        created,
        updated,
        skipped,
    }))
}
