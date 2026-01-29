//! `SeaORM` Entity for Event
//!
//! Mirrors the Event structure from the bucket for fast database lookups.
//! The full Event data with versioning is kept in the bucket, this is the DB mirror.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(schema_name = "public", table_name = "Event")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,

    #[sea_orm(column_name = "appId", column_type = "Text")]
    pub app_id: String,

    #[sea_orm(column_type = "Text")]
    pub name: String,

    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,

    /// Event type: "chat", "api", "email", "quick_action", etc.
    #[sea_orm(column_name = "eventType", column_type = "Text")]
    pub event_type: String,

    /// Whether this event is active
    pub active: bool,

    /// Priority for ordering events
    pub priority: i32,

    /// Board this event triggers (mutually exclusive with pageId for A2UI events)
    #[sea_orm(column_name = "boardId", column_type = "Text", nullable)]
    pub board_id: Option<String>,

    /// Board version as semver string "major.minor.patch"
    #[sea_orm(column_name = "boardVersion", column_type = "Text", nullable)]
    pub board_version: Option<String>,

    /// Node ID within the board
    #[sea_orm(column_name = "nodeId", column_type = "Text", nullable)]
    pub node_id: Option<String>,

    /// For A2UI events: default page to render
    #[sea_orm(column_name = "pageId", column_type = "Text", nullable)]
    pub page_id: Option<String>,

    /// URL route path that maps to this event (e.g., "/", "/dashboard")
    #[sea_orm(column_type = "Text", nullable)]
    pub route: Option<String>,

    /// Whether this is the default event/route for the app
    #[sea_orm(column_name = "isDefault")]
    pub is_default: bool,

    /// Event version as semver string "major.minor.patch"
    #[sea_orm(column_name = "eventVersion", column_type = "Text")]
    pub event_version: String,

    /// Variables for this event (JSON blob)
    #[sea_orm(column_type = "Json", nullable)]
    pub variables: Option<Json>,

    /// Event config (stored as JSON)
    #[sea_orm(column_type = "Json", nullable)]
    pub config: Option<Json>,

    /// Input pins metadata for quick access without loading board
    #[sea_orm(column_type = "Json", nullable)]
    pub inputs: Option<Json>,

    /// Release notes - can be plain text or URL
    #[sea_orm(column_type = "Json", nullable)]
    pub notes: Option<Json>,

    /// Canary configuration for A/B testing
    #[sea_orm(column_type = "Json", nullable)]
    pub canary: Option<Json>,

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
    #[sea_orm(has_one = "super::event_sink::Entity")]
    EventSink,
}

impl Related<super::app::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::App.def()
    }
}

impl Related<super::event_sink::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::EventSink.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
