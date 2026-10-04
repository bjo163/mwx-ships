//! SeaORM entity for immutable deployment revisions.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "deployment_revisions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub application_id: i64,
    pub server_id: i64,
    pub source_commit_hash: String,
    pub source_commit_message: Option<String>,
    pub image_reference: String,
    pub revision_hash: String,
    pub runtime_snapshot: String,
    pub status: String,
    pub healthy_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::applications::Entity",
        from = "Column::ApplicationId",
        to = "super::applications::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Applications,
    #[sea_orm(
        belongs_to = "super::servers::Entity",
        from = "Column::ServerId",
        to = "super::servers::Column::Id",
        on_update = "NoAction",
        on_delete = "Restrict"
    )]
    Servers,
    #[sea_orm(has_many = "super::deployments::Entity")]
    Deployments,
}

impl Related<super::applications::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Applications.def()
    }
}

impl Related<super::servers::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Servers.def()
    }
}

impl Related<super::deployments::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Deployments.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
