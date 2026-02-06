//! `SeaORM` Entity for EventSink
//!
//! Stores sink-specific data. Most event config comes from the Event itself via join.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(schema_name = "public", table_name = "EventSink")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,
    /// Unique event ID - 1:1 mapping between event and sink
    #[sea_orm(column_name = "eventId", column_type = "Text", unique)]
    pub event_id: String,
    /// App for permission checks
    #[sea_orm(column_name = "appId", column_type = "Text")]
    pub app_id: String,
    /// Sink type: "http", "cron", "discord", "telegram", etc.
    #[sea_orm(column_name = "sinkType", column_type = "Text")]
    pub sink_type: String,
    /// Is the sink active on the server?
    pub active: bool,
    /// For HTTP sinks: the unique path to listen on
    #[sea_orm(column_type = "Text", nullable)]
    pub path: Option<String>,
    /// Auth token for securing HTTP endpoints
    #[sea_orm(column_name = "authToken", column_type = "Text", nullable)]
    pub auth_token: Option<String>,
    /// Secret token for Telegram webhook verification (X-Telegram-Bot-Api-Secret-Token header)
    #[sea_orm(column_name = "webhookSecret", column_type = "Text", nullable)]
    pub webhook_secret: Option<String>,
    /// For cron sinks: the cron expression
    #[sea_orm(column_name = "cronExpression", column_type = "Text", nullable)]
    pub cron_expression: Option<String>,
    /// For cron sinks: timezone (default UTC)
    #[sea_orm(column_name = "cronTimezone", column_type = "Text", nullable)]
    pub cron_timezone: Option<String>,
    /// Encrypted PAT for execution (optional - enables model/file access in triggered flows)
    #[sea_orm(column_name = "patEncrypted", column_type = "Text", nullable)]
    pub pat_encrypted: Option<String>,
    /// Encrypted OAuth tokens JSON (optional - for provider-specific access)
    #[sea_orm(column_name = "oauthTokensEncrypted", column_type = "Text", nullable)]
    pub oauth_tokens_encrypted: Option<String>,
    /// Snapshot of the last updater's profile (bits, hubs) for trigger execution
    #[sea_orm(column_name = "profileJson", column_type = "Json", nullable)]
    pub profile_json: Option<Json>,
    #[sea_orm(column_name = "createdAt")]
    pub created_at: DateTime,
    #[sea_orm(column_name = "updatedAt")]
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::app::Entity",
        from = "Column::AppId",
        to = "super::app::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    App,
    #[sea_orm(
        belongs_to = "super::event::Entity",
        from = "Column::EventId",
        to = "super::event::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Event,
}

impl Related<super::app::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::App.def()
    }
}

impl Related<super::event::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Event.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
