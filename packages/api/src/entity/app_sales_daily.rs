//! `SeaORM` Entity for daily sales aggregation
//! Pre-aggregated daily sales data for efficient dashboard queries

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Daily aggregated sales data for an app
/// This table is populated by a scheduled job or trigger for efficient querying
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(schema_name = "public", table_name = "AppSalesDaily")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,
    /// The app these stats belong to
    #[sea_orm(column_name = "appId", column_type = "Text")]
    pub app_id: String,
    /// Date for this aggregation (YYYY-MM-DD stored as Date type)
    pub date: Date,
    /// Total revenue for the day (in cents)
    #[sea_orm(column_name = "totalRevenue")]
    pub total_revenue: i64,
    /// Gross revenue before discounts (in cents)
    #[sea_orm(column_name = "grossRevenue")]
    pub gross_revenue: i64,
    /// Total discount amount given (in cents)
    #[sea_orm(column_name = "totalDiscounts")]
    pub total_discounts: i64,
    /// Number of purchases
    #[sea_orm(column_name = "purchaseCount")]
    pub purchase_count: i64,
    /// Number of refunds
    #[sea_orm(column_name = "refundCount")]
    pub refund_count: i64,
    /// Total refund amount (in cents)
    #[sea_orm(column_name = "refundAmount")]
    pub refund_amount: i64,
    /// Number of unique buyers
    #[sea_orm(column_name = "uniqueBuyers")]
    pub unique_buyers: i64,
    /// Average order value (in cents)
    #[sea_orm(column_name = "avgOrderValue")]
    pub avg_order_value: i64,
    /// Number of discount codes used
    #[sea_orm(column_name = "discountCodesUsed")]
    pub discount_codes_used: i64,
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
