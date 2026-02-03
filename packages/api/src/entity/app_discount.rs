//! `SeaORM` Entity for app discount codes

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a discount code for an app
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(schema_name = "public", table_name = "AppDiscount")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,
    /// The app this discount applies to
    #[sea_orm(column_name = "appId", column_type = "Text")]
    pub app_id: String,
    /// Discount code (e.g., "LAUNCH20")
    #[sea_orm(column_type = "Text")]
    pub code: String,
    /// Human-readable name for the discount
    #[sea_orm(column_type = "Text")]
    pub name: String,
    /// Description of the discount
    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,
    /// Discount type: percentage or fixed amount
    #[sea_orm(column_name = "discountType")]
    pub discount_type: super::sea_orm_active_enums::DiscountType,
    /// Discount value (percentage 0-100 or fixed amount in cents)
    #[sea_orm(column_name = "discountValue")]
    pub discount_value: i64,
    /// Maximum number of uses (null = unlimited)
    #[sea_orm(column_name = "maxUses", nullable)]
    pub max_uses: Option<i64>,
    /// Current number of times used
    #[sea_orm(column_name = "usedCount")]
    pub used_count: i64,
    /// Minimum purchase amount required (in cents, null = no minimum)
    #[sea_orm(column_name = "minPurchaseAmount", nullable)]
    pub min_purchase_amount: Option<i64>,
    /// When the discount becomes active
    #[sea_orm(column_name = "startsAt")]
    pub starts_at: DateTime,
    /// When the discount expires (null = never)
    #[sea_orm(column_name = "expiresAt", nullable)]
    pub expires_at: Option<DateTime>,
    /// Whether the discount is currently active
    #[sea_orm(column_name = "isActive")]
    pub is_active: bool,
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
    #[sea_orm(has_many = "super::app_purchase::Entity")]
    AppPurchase,
}

impl Related<super::app::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::App.def()
    }
}

impl Related<super::app_purchase::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AppPurchase.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
