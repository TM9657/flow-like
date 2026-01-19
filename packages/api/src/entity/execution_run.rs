//! Execution run entity for tracking board/event executions
//!
//! Note: This entity should be regenerated via sea-orm-codegen after running
//! prisma migrate. This is a manual implementation to match the schema.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "RunStatus")]
pub enum RunStatus {
    #[sea_orm(string_value = "PENDING")]
    Pending,
    #[sea_orm(string_value = "RUNNING")]
    Running,
    #[sea_orm(string_value = "COMPLETED")]
    Completed,
    #[sea_orm(string_value = "FAILED")]
    Failed,
    #[sea_orm(string_value = "CANCELLED")]
    Cancelled,
    #[sea_orm(string_value = "TIMEOUT")]
    Timeout,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "RunMode")]
pub enum RunMode {
    #[sea_orm(string_value = "LOCAL")]
    Local,
    #[sea_orm(string_value = "HTTP")]
    Http,
    #[sea_orm(string_value = "LAMBDA")]
    Lambda,
    #[sea_orm(string_value = "KUBERNETES_ISOLATED")]
    KubernetesIsolated,
    #[sea_orm(string_value = "KUBERNETES_POOL")]
    KubernetesPool,
    #[sea_orm(string_value = "FUNCTION")]
    Function,
    #[sea_orm(string_value = "QUEUE")]
    Queue,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(schema_name = "public", table_name = "ExecutionRun")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,

    #[sea_orm(column_name = "boardId", column_type = "Text")]
    pub board_id: String,

    #[sea_orm(column_type = "Text", nullable)]
    pub version: Option<String>,

    #[sea_orm(column_name = "eventId", column_type = "Text", nullable)]
    pub event_id: Option<String>,

    #[sea_orm(column_name = "nodeId", column_type = "Text", nullable)]
    pub node_id: Option<String>,

    pub status: RunStatus,

    pub mode: RunMode,

    #[sea_orm(column_name = "logLevel", default_value = 0)]
    pub log_level: i32,

    #[sea_orm(column_name = "inputPayloadLen", default_value = 0)]
    pub input_payload_len: i64,

    #[sea_orm(column_name = "inputPayloadKey", column_type = "Text", nullable)]
    pub input_payload_key: Option<String>,

    #[sea_orm(column_name = "outputPayloadLen", default_value = 0)]
    pub output_payload_len: i64,

    #[sea_orm(column_name = "errorMessage", column_type = "Text", nullable)]
    pub error_message: Option<String>,

    #[sea_orm(default_value = 0)]
    pub progress: i32,

    #[sea_orm(column_name = "currentStep", column_type = "Text", nullable)]
    pub current_step: Option<String>,

    #[sea_orm(column_name = "startedAt", nullable)]
    pub started_at: Option<DateTime>,

    #[sea_orm(column_name = "completedAt", nullable)]
    pub completed_at: Option<DateTime>,

    #[sea_orm(column_name = "expiresAt", nullable)]
    pub expires_at: Option<DateTime>,

    #[sea_orm(column_name = "userId", column_type = "Text", nullable)]
    pub user_id: Option<String>,

    #[sea_orm(column_name = "appId", column_type = "Text")]
    pub app_id: String,

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
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    User,
    #[sea_orm(has_many = "super::execution_event::Entity")]
    ExecutionEvent,
}

impl Related<super::app::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::App.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::execution_event::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ExecutionEvent.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
