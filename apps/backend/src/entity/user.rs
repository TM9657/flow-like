//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.4

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "User")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,
    #[sea_orm(column_type = "Text", unique)]
    pub email: String,
    #[sea_orm(column_type = "Text", unique)]
    pub username: String,
    #[sea_orm(column_type = "Text")]
    pub name: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,
    #[sea_orm(column_name = "avatarUrl", column_type = "Text", nullable)]
    pub avatar_url: Option<String>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub themes: Option<Json>,
    #[sea_orm(
        column_name = "additionalInformation",
        column_type = "JsonBinary",
        nullable
    )]
    pub additional_information: Option<Json>,
    pub permission: i64,
    #[sea_orm(column_name = "createdAt")]
    pub created_at: DateTime,
    #[sea_orm(column_name = "updatedAt")]
    pub updated_at: DateTime,
    #[sea_orm(column_name = "acceptedTermsVersion", column_type = "Text", nullable)]
    pub accepted_terms_version: Option<String>,
    #[sea_orm(column_name = "tutorialCompleted")]
    pub tutorial_completed: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::comment::Entity")]
    Comment,
    #[sea_orm(has_many = "super::feedback::Entity")]
    Feedback,
    #[sea_orm(has_many = "super::invitations::Entity")]
    Invitations,
    #[sea_orm(has_many = "super::join_queue::Entity")]
    JoinQueue,
    #[sea_orm(has_many = "super::membership::Entity")]
    Membership,
    #[sea_orm(has_many = "super::provider_invocation::Entity")]
    ProviderInvocation,
    #[sea_orm(has_many = "super::publication_log::Entity")]
    PublicationLog,
    #[sea_orm(has_many = "super::publication_request::Entity")]
    PublicationRequest,
    #[sea_orm(has_many = "super::user_api_key::Entity")]
    UserApiKey,
}

impl Related<super::comment::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Comment.def()
    }
}

impl Related<super::feedback::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Feedback.def()
    }
}

impl Related<super::invitations::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Invitations.def()
    }
}

impl Related<super::join_queue::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::JoinQueue.def()
    }
}

impl Related<super::membership::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Membership.def()
    }
}

impl Related<super::provider_invocation::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProviderInvocation.def()
    }
}

impl Related<super::publication_log::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PublicationLog.def()
    }
}

impl Related<super::publication_request::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PublicationRequest.def()
    }
}

impl Related<super::user_api_key::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserApiKey.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
