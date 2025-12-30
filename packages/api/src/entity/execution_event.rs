//! Execution event entity for streaming events from execution environments
//!
//! These events are batch-pushed by executors and fetched via long polling by clients.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(schema_name = "public", table_name = "ExecutionEvent")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,

    #[sea_orm(column_name = "runId", column_type = "Text")]
    pub run_id: String,

    /// Event sequence number for ordering
    pub sequence: i32,

    /// Event type (log, progress, output, error, etc.)
    #[sea_orm(column_name = "eventType", column_type = "Text")]
    pub event_type: String,

    /// Event payload (JSON)
    #[sea_orm(column_type = "JsonBinary")]
    pub payload: serde_json::Value,

    /// Whether this event has been delivered to the client
    #[sea_orm(default_value = false)]
    pub delivered: bool,

    /// Auto-cleanup after TTL (max 1 day)
    #[sea_orm(column_name = "expiresAt")]
    pub expires_at: DateTime,

    #[sea_orm(column_name = "createdAt")]
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::execution_run::Entity",
        from = "Column::RunId",
        to = "super::execution_run::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    ExecutionRun,
}

impl Related<super::execution_run::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ExecutionRun.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
