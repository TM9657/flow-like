//! `SeaORM` Entity for SinkToken
//!
//! Service sink tokens - JWT tokens issued to internal services for triggering sinks.
//! Each token is scoped to a specific sink type and can be individually revoked.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(schema_name = "public", table_name = "SinkToken")]
pub struct Model {
    /// JWT ID (jti claim) - unique identifier for the token
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,
    /// The sink type this token is authorized to trigger
    #[sea_orm(column_name = "sinkType", column_type = "Text")]
    pub sink_type: String,
    /// Human-readable name/description
    #[sea_orm(column_type = "Text", nullable)]
    pub name: Option<String>,
    /// Whether this token has been revoked
    pub revoked: bool,
    /// When the token was revoked
    #[sea_orm(column_name = "revokedAt")]
    pub revoked_at: Option<DateTime>,
    /// Who/what revoked the token
    #[sea_orm(column_name = "revokedBy", column_type = "Text", nullable)]
    pub revoked_by: Option<String>,
    #[sea_orm(column_name = "createdAt")]
    pub created_at: DateTime,
    #[sea_orm(column_name = "updatedAt")]
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
