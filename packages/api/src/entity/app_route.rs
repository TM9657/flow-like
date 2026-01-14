//! `SeaORM` Entity for AppRoute

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use super::sea_orm_active_enums::RouteTargetType;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(schema_name = "public", table_name = "AppRoute")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,
    #[sea_orm(column_name = "appId", column_type = "Text")]
    pub app_id: String,
    #[sea_orm(column_type = "Text")]
    pub path: String,
    #[sea_orm(column_name = "targetType")]
    pub target_type: RouteTargetType,
    #[sea_orm(column_name = "pageId", column_type = "Text", nullable)]
    pub page_id: Option<String>,
    #[sea_orm(column_name = "boardId", column_type = "Text", nullable)]
    pub board_id: Option<String>,
    #[sea_orm(column_name = "pageVersion", column_type = "Text", nullable)]
    pub page_version: Option<String>,
    #[sea_orm(column_name = "eventId", column_type = "Text", nullable)]
    pub event_id: Option<String>,
    #[sea_orm(column_name = "isDefault")]
    pub is_default: bool,
    pub priority: i32,
    #[sea_orm(column_type = "Text", nullable)]
    pub label: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub icon: Option<String>,
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
}

impl Related<super::app::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::App.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
