use crate::state::AppState;
use axum::{
    Router,
    routing::{get, post},
};
use billing::get_billing_session;
use info::user_info;
use pricing::get_pricing;
use subscribe::create_subscription_checkout;

pub async fn sign_avatar(
    sub: &str,
    avatar_id: &str,
    state: &AppState,
) -> flow_like_types::Result<String> {
    let master_store = state.master_credentials().await?;
    let master_store = master_store.to_store(false).await?;
    let file_name = format!("{}.webp", avatar_id);
    let path = flow_like_storage::Path::from("media")
        .child("users")
        .child(sub)
        .child(file_name);
    let url = master_store
        .sign("GET", &path, std::time::Duration::from_secs(60 * 5))
        .await?;
    Ok(url.to_string())
}

pub mod billing;
pub mod get_invites;
pub mod info;
pub mod lookup;
pub mod manage_invite;
pub mod notifications;
pub mod pat;
pub mod pricing;
pub mod subscribe;
pub mod templates;
pub mod upsert_info;
pub mod widgets;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/pat",
            get(pat::get_pats::get_pats)
                .put(pat::create_pat::create_pat)
                .delete(pat::delete_pat::delete_pat),
        )
        .route("/info", get(user_info).put(upsert_info::upsert_info))
        .route("/billing", get(get_billing_session))
        .route("/pricing", get(get_pricing))
        .route("/subscribe", post(create_subscription_checkout))
        .route("/lookup/{sub}", get(lookup::user_lookup))
        .route("/search/{query}", get(lookup::user_search))
        .route("/invites", get(get_invites::get_invites))
        .route("/templates", get(templates::get_templates))
        .route("/widgets", get(widgets::get_widgets))
        .route("/notifications", get(notifications::get_notifications))
        .route(
            "/notifications/list",
            get(notifications::list_notifications),
        )
        .route(
            "/notifications/read-all",
            post(notifications::mark_all_read),
        )
        .route(
            "/notifications/{notification_id}",
            post(notifications::mark_notification_read).delete(notifications::delete_notification),
        )
        .route(
            "/invites/{invite_id}",
            post(manage_invite::accept_invite).delete(manage_invite::reject_invite),
        )
}
