use crate::state::AppState;
use axum::{
    Router,
    routing::{get, post},
};

pub mod create_default;
pub mod delete_profile;
pub mod get_profiles;
pub mod sync_profiles;
pub mod upsert_profile;

/// Sign a profile image URL for reading.
/// The icon/thumbnail fields store just the CUID, we construct the full path here.
pub async fn sign_profile_image(
    profile_id: &str,
    image_id: &str,
    state: &AppState,
) -> flow_like_types::Result<String> {
    let master_store = state.master_credentials().await?;
    let master_store = master_store.to_store(false).await?;
    let file_name = format!("{}.webp", image_id);
    let path = flow_like_storage::Path::from("media")
        .child("profiles")
        .child(profile_id)
        .child(file_name);
    let url = master_store
        .sign("GET", &path, std::time::Duration::from_secs(60 * 5))
        .await?;
    Ok(url.to_string())
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_profiles::get_profiles))
        .route("/sync", post(sync_profiles::sync_profiles))
        .route(
            "/{profile_id}",
            post(upsert_profile::upsert_profile).delete(delete_profile::delete_profile),
        )
}
