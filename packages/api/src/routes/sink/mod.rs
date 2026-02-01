//! Sink routes module
//!
//! Provides endpoints for:
//! - Sink activation/deactivation (user can only toggle active state)
//! - HTTP sink triggers (/sink/trigger/http/{app_id}/{path})
//! - Telegram webhook triggers (/sink/trigger/telegram/{event_id})
//! - Discord interactions webhook triggers (/sink/trigger/discord/{event_id})
//! - Listing all active sinks for user's apps
//!
//! Note: Sink config comes from the Event itself. We only store sink-specific
//! data like path and auth token. Sinks are auto-created when events are created.

use crate::state::AppState;
use axum::{
    Router,
    routing::{delete, get, patch, post},
};

mod management;
mod trigger;

/// Re-export the trigger types and utility for programmatic use (Lambda handlers, SQS, etc.)
pub use trigger::{TriggerEventInput, TriggerResponse, trigger_event};

pub fn routes() -> Router<AppState> {
    Router::new()
        // List all active sinks for apps user has access to
        .route("/", get(management::list_user_sinks))
        // List sinks for a specific app
        .route("/app/{app_id}", get(management::list_app_sinks))
        // Get/update/toggle a specific sink
        .route("/{event_id}", get(management::get_sink))
        .route("/{event_id}", patch(management::update_sink))
        .route("/{event_id}/toggle", post(management::toggle_sink))
        // HTTP sink trigger - matches any method and path after app_id
        .route("/trigger/http/{app_id}/{*path}", get(trigger::http_trigger))
        .route("/trigger/http/{app_id}/{*path}", post(trigger::http_trigger))
        .route(
            "/trigger/http/{app_id}/{*path}",
            patch(trigger::http_trigger),
        )
        .route(
            "/trigger/http/{app_id}/{*path}",
            delete(trigger::http_trigger),
        )
        // Telegram webhook trigger - async execution with secret token verification
        .route(
            "/trigger/telegram/{event_id}",
            post(trigger::telegram_trigger),
        )
        // Discord interactions webhook trigger - async execution with Ed25519 signature verification
        .route(
            "/trigger/discord/{event_id}",
            post(trigger::discord_trigger),
        )
}
