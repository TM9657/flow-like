use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::event_sink::cron::CronSink;
use crate::event_sink::deeplink::DeeplinkSink;
use crate::event_sink::discord::DiscordSink;
use crate::event_sink::email::EmailSink;
use crate::event_sink::file::FileSink;
use crate::event_sink::geolocation::GeoLocationSink;
use crate::event_sink::github::GitHubSink;
use crate::event_sink::http::HttpSink;
use crate::event_sink::mcp::MCPSink;
use crate::event_sink::mqtt::MQTTSink;
use crate::event_sink::nfc::NFCSink;
use crate::event_sink::notion::NotionSink;
use crate::event_sink::rss::RSSSink;
use crate::event_sink::shortcut::ShortcutSink;
use crate::event_sink::slack::SlackSink;
use crate::event_sink::telegram::TelegramSink;
use crate::event_sink::web_watcher::WebWatcherSink;
use crate::event_sink::webhook::WebhookSink;

pub mod cron;
pub mod deeplink;
pub mod discord;
pub mod email;
pub mod file;
pub mod geolocation;
pub mod github;
pub mod http;
pub mod manager;
pub mod mcp;
pub mod mqtt;
pub mod nfc;
pub mod notion;
pub mod rss;
pub mod shortcut;
pub mod slack;
pub mod storage;
pub mod stubs; // Stub implementations for not-yet-complete sinks
pub mod telegram;
pub mod web_watcher;
pub mod webhook;

use manager::DbConnection;

// Re-export manager for convenience
pub use manager::EventSinkManager;

#[async_trait::async_trait]
pub trait EventSink: Send + Sync {
    /// Initialize sink infrastructure (e.g., start HTTP server, Discord client, cron worker)
    /// This is called once when the sink type is first used
    async fn start(&self, app_handle: &AppHandle, db: DbConnection) -> anyhow::Result<()>;

    /// Shutdown sink infrastructure and cleanup resources
    async fn stop(&self, app_handle: &AppHandle, db: DbConnection) -> anyhow::Result<()>;

    /// Called when a new event registration is added for this sink
    /// The sink should set up the specific trigger (e.g., add route to HTTP server, register Discord handler)
    async fn on_register(
        &self,
        app_handle: &AppHandle,
        registration: &EventRegistration,
        db: DbConnection,
    ) -> anyhow::Result<()>;

    /// Called when an event registration is removed
    /// The sink should clean up the specific trigger
    async fn on_unregister(
        &self,
        app_handle: &AppHandle,
        registration: &EventRegistration,
        db: DbConnection,
    ) -> anyhow::Result<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EventConfig {
    Cron(CronSink),
    Discord(DiscordSink),
    Email(EmailSink),

    Slack(SlackSink),
    Telegram(TelegramSink),

    // Check for the state of a website and trigger the event when it changes
    WebWatcher(WebWatcherSink),
    Rss(RSSSink),

    Deeplink(DeeplinkSink),
    Http(HttpSink),
    Webhook(WebhookSink),
    Mqtt(MQTTSink),
    Mcp(MCPSink),
    File(FileSink),
    GitHub(GitHubSink),

    Nfc(NFCSink),
    GeoLocation(GeoLocationSink),
    Notion(NotionSink),
    Shortcut(ShortcutSink),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRegistration {
    pub event_id: String, // Primary key - each event can only be attached to one sink
    pub name: String,
    pub r#type: String,
    pub updated_at: SystemTime,
    pub created_at: SystemTime,
    pub config: EventConfig,
    pub offline: bool,
    pub app_id: String,
    pub default_payload: Option<flow_like_types::Value>,
}
