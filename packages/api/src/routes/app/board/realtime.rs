use crate::{
    ensure_permission,
    entity::{board_sync, prelude::*},
    error::ApiError,
    middleware::jwt::AppUser,
    permission::role_permission::RolePermissions,
    state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use flow_like_types::base64::Engine;
use flow_like_types::base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use flow_like_types::{anyhow, create_id};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use p256::{
    PublicKey as P256PublicKey, elliptic_curve::sec1::ToEncodedPoint, pkcs8::DecodePublicKey,
};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use sea_orm::TransactionTrait;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

// ============================================================================
// Realtime collaboration auth (JWT + room key) + JWKS
// ============================================================================

// Generated via: tools/gen-pk.sh
static PRIVATE_KEY_PEM: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let b64 = std::env::var("REALTIME_KEY").expect("Missing REALTIME_KEY env var");
    STANDARD
        .decode(&b64)
        .expect("Failed to decode REALTIME_KEY b64")
});

static PUBLIC_KEY_PEM: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let b64 = std::env::var("REALTIME_PUB").expect("Missing REALTIME_PUB env var");
    STANDARD
        .decode(&b64)
        .expect("Failed to decode REALTIME_PUB b64")
});

static KID: LazyLock<String> = LazyLock::new(|| {
    std::env::var("REALTIME_KID").unwrap_or_else(|_| "realtime-es256-v1".to_string())
});

const ISSUER: &str = "flow-like";
const AUDIENCE: &str = "y-webrtc";
const SCOPE: &str = "realtime.read";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RealtimeClaims {
    sub: String,
    name: Option<String>,
    app_id: String,
    board_id: String,
    scope: String,
    iss: String,
    aud: String,
    iat: i64,
    nbf: i64,
    exp: i64,
    jti: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RealtimeParams {
    /// JWT authorizing the user for this (app_id, board_id) in y-webrtc
    jwt: String,
    /// Base64 256-bit room key (rotated daily)
    encryption_key: String,
    /// Key identifier (ISO date, e.g. "2025-10-23")
    key_id: String,
}

// ---- JWKS types ----

#[derive(Serialize)]
pub struct Jwk {
    kty: String,
    crv: String,
    x: String,
    y: String,
    alg: String,
    kid: String,
    #[serde(rename = "use")]
    r#use: String,
}

#[derive(Serialize)]
pub struct Jwks {
    keys: Vec<Jwk>,
}

fn generate_encryption_key() -> String {
    use flow_like_types::rand::{TryRngCore, rngs::OsRng};
    let mut key = [0u8; 32];
    let mut rng = OsRng;
    rng.try_fill_bytes(&mut key)
        .expect("Failed to generate random key");
    STANDARD.encode(key)
}

// ============================================================================
// JWKS (no auth) — mount at GET /apps/{app_id}/board/{board_id}/realtime
// ============================================================================
#[tracing::instrument(
    name = "GET /apps/{app_id}/board/{board_id}/realtime",
    skip(_state, user)
)]
pub async fn jwks(
    State(_state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((_app_id, _board_id)): Path<(String, String)>,
) -> Result<Json<Jwks>, ApiError> {
    user.sub()?;
    // Parse PEM -> P-256 public key, then extract uncompressed point (x,y).
    let pem = String::from_utf8_lossy(&PUBLIC_KEY_PEM);
    let pubkey: P256PublicKey = P256PublicKey::from_public_key_pem(&pem).map_err(|e| {
        ApiError::InternalError(anyhow!("Invalid ES256 public key PEM: {}", e).into())
    })?;
    let encoded = pubkey.to_encoded_point(false); // uncompressed
    let x = encoded
        .x()
        .ok_or_else(|| ApiError::InternalError(anyhow!("Missing X coord").into()))?;
    let y = encoded
        .y()
        .ok_or_else(|| ApiError::InternalError(anyhow!("Missing Y coord").into()))?;

    let jwk = Jwk {
        kty: "EC".to_string(),
        crv: "P-256".to_string(),
        x: URL_SAFE_NO_PAD.encode(x),
        y: URL_SAFE_NO_PAD.encode(y),
        alg: "ES256".to_string(),
        kid: KID.clone(),
        r#use: "sig".to_string(),
    };

    Ok(Json(Jwks { keys: vec![jwk] }))
}

// ============================================================================
// Access token + room key
// ============================================================================
#[tracing::instrument(
    name = "POST /apps/{app_id}/board/{board_id}/realtime",
    skip(state, user)
)]
pub async fn access(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, board_id)): Path<(String, String)>,
) -> Result<Json<RealtimeParams>, ApiError> {
    let permission = ensure_permission!(user, &app_id, &state, RolePermissions::ReadBoards);
    let sub = permission.sub()?;

    let user_model = User::find_by_id(&sub)
        .one(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound)?;

    let (encryption_key, key_id) = get_or_rotate_room_key(&state, &app_id, &board_id).await?;

    let iat = chrono::Utc::now().timestamp();
    let exp = iat + (3 * 60 * 60); // 3 hours
    let nbf = iat - 30; // small skew window

    let claims = RealtimeClaims {
        sub: sub.clone(),
        name: user_model.name,
        app_id: app_id.clone(),
        board_id: board_id.clone(),
        scope: SCOPE.to_string(),
        iss: ISSUER.to_string(),
        aud: AUDIENCE.to_string(),
        iat,
        nbf,
        exp,
        jti: create_id(),
    };

    let mut header = Header::new(Algorithm::ES256);
    header.kid = Some(KID.clone());

    let encoding_key = EncodingKey::from_ec_pem(&PRIVATE_KEY_PEM)
        .map_err(|e| ApiError::InternalError(anyhow!("Failed to load EC key: {}", e).into()))?;

    let jwt = encode(&header, &claims, &encoding_key)
        .map_err(|e| ApiError::InternalError(anyhow!("Failed to encode JWT: {}", e).into()))?;

    Ok(Json(RealtimeParams {
        jwt,
        encryption_key,
        key_id,
    }))
}

// ----------------------------------------------------------------------------
// Helper: get or rotate the per-board room key (daily rotation), returns (key, key_id)
// ----------------------------------------------------------------------------
async fn get_or_rotate_room_key(
    state: &AppState,
    app_id: &str,
    board_id: &str,
) -> Result<(String, String), ApiError> {
    let now = chrono::Utc::now().naive_utc();
    let today = now.date(); // NaiveDate
    let key_id = today.format("%Y-%m-%d").to_string();

    let txn = state.db.begin().await?;

    // Lock row if exists
    let existing = BoardSync::find()
        .filter(board_sync::Column::AppId.eq(app_id))
        .filter(board_sync::Column::BoardId.eq(board_id))
        .one(&txn)
        .await?;

    let encryption_key = match existing {
        Some(sync) => {
            // Rotate if last_synced_at is from a previous day
            if sync.last_synced_at.date() < today {
                let new_key = generate_encryption_key();
                let mut active_sync: board_sync::ActiveModel = sync.into();
                active_sync.sync_encryption_key = Set(new_key.clone());
                active_sync.last_synced_at = Set(now);
                active_sync.updated_at = Set(now);
                active_sync.update(&txn).await?;
                new_key
            } else {
                sync.sync_encryption_key
            }
        }
        None => {
            let new_key = generate_encryption_key();
            let new_sync = board_sync::ActiveModel {
                id: Set(create_id()),
                app_id: Set(app_id.to_string()),
                board_id: Set(board_id.to_string()),
                last_synced_at: Set(now),
                sync_encryption_key: Set(new_key.clone()),
                created_at: Set(now),
                updated_at: Set(now),
            };
            new_sync.insert(&txn).await?;
            new_key
        }
    };

    txn.commit().await?;

    Ok((encryption_key, key_id))
}
