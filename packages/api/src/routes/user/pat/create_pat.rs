use crate::{entity::pat, error::ApiError, middleware::jwt::AppUser, state::AppState};
use axum::{Extension, Json, extract::State};
use flow_like_types::{
    anyhow,
    base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD},
    create_id,
    rand::{TryRngCore, rngs::OsRng},
};
use sea_orm::{ActiveModelTrait, ActiveValue::Set};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct PatOut {
    pub pat: String,
    pub permission: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PatInput {
    pub name: String,

    // Optional expiration timestamp (in seconds since epoch)
    pub valid_until: Option<i64>,
    pub permissions: Option<i64>,
}

#[tracing::instrument(name = "PUT /user/pat", skip(state, user, input))]
pub async fn create_pat(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Json(input): Json<PatInput>,
) -> Result<Json<PatOut>, ApiError> {
    let sub = user.sub()?;

    let permissions = input.permissions.unwrap_or(1);
    if permissions == 0 {
        return Err(ApiError::bad_request("Permissions cannot be zero"));
    }

    let valid_until = match input.valid_until {
        Some(ts) => Some(
            chrono::DateTime::from_timestamp(ts, 0)
                .ok_or_else(|| ApiError::bad_request("Invalid valid_until timestamp"))?,
        ),
        None => None,
    };
    let naive_datetime = valid_until.map(|dt| dt.naive_utc());

    let mut secret_bytes = [0u8; 32];
    OsRng.try_fill_bytes(&mut secret_bytes)
        .map_err(|e| ApiError::internal(format!("Failed to generate random bytes: {}", e)))?;
    let secret_b64 = URL_SAFE_NO_PAD.encode(secret_bytes);

    let mut hasher = blake3::Hasher::new();
    hasher.update(secret_b64.as_bytes());
    let secret_hash = hasher.finalize().to_hex().to_string().to_lowercase();

    let pat = pat::ActiveModel {
        id: Set(create_id()),
        key: Set(secret_hash),
        name: Set(input.name),
        user_id: Set(sub.to_string()),
        valid_until: Set(naive_datetime),
        permissions: Set(permissions),
        created_at: Set(chrono::Utc::now().naive_utc()),
        updated_at: Set(chrono::Utc::now().naive_utc()),
    };

    let pat = pat.insert(&state.db).await?;
    let pat_out = PatOut {
        pat: format!("pat_{}.{}", pat.id, secret_b64),
        permission: pat.permissions,
    };
    Ok(Json(pat_out))
}
