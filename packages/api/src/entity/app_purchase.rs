//! `SeaORM` Entity for tracking app purchases

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a completed purchase of an app
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(schema_name = "public", table_name = "AppPurchase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,
    /// The user who made the purchase
    #[sea_orm(column_name = "userId", column_type = "Text")]
    pub user_id: String,
    /// The app that was purchased
    #[sea_orm(column_name = "appId", column_type = "Text")]
    pub app_id: String,
    /// Price paid in cents at time of purchase
    #[sea_orm(column_name = "pricePaid")]
    pub price_paid: i64,
    /// Original price before discount (in cents)
    #[sea_orm(column_name = "originalPrice")]
    pub original_price: i64,
    /// Discount amount in cents (0 if none)
    #[sea_orm(column_name = "discountAmount")]
    pub discount_amount: i64,
    /// ID of discount code used, if any
    #[sea_orm(column_name = "discountId", column_type = "Text", nullable)]
    pub discount_id: Option<String>,
    /// Currency code (e.g., "EUR", "USD")
    #[sea_orm(column_type = "Text")]
    pub currency: String,
    /// Stripe checkout session ID for this purchase
    #[sea_orm(column_name = "stripeSessionId", column_type = "Text")]
    pub stripe_session_id: String,
    /// Stripe payment intent ID
    #[sea_orm(column_name = "stripePaymentIntentId", column_type = "Text", nullable)]
    pub stripe_payment_intent_id: Option<String>,
    /// Purchase status
    pub status: super::sea_orm_active_enums::PurchaseStatus,
    /// When purchase was completed
    #[sea_orm(column_name = "completedAt", nullable)]
    pub completed_at: Option<DateTime>,
    /// When refunded (if applicable)
    #[sea_orm(column_name = "refundedAt", nullable)]
    pub refunded_at: Option<DateTime>,
    /// Refund reason if refunded
    #[sea_orm(column_name = "refundReason", column_type = "Text", nullable)]
    pub refund_reason: Option<String>,
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
        on_delete = "Cascade"
    )]
    User,
    #[sea_orm(
        belongs_to = "super::app_discount::Entity",
        from = "Column::DiscountId",
        to = "super::app_discount::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    AppDiscount,
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

impl Related<super::app_discount::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AppDiscount.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
