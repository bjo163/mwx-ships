//! `SeaORM` Entity for Persistent Volumes

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "persistent_volumes")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub organization_id: i64,
    pub server_id: i64,
    pub name: String,
    pub docker_volume_name: String,
    pub deletion_protected: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::organizations::Entity",
        from = "Column::OrganizationId",
        to = "super::organizations::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Organizations,
    #[sea_orm(
        belongs_to = "super::servers::Entity",
        from = "Column::ServerId",
        to = "super::servers::Column::Id",
        on_update = "NoAction",
        on_delete = "Restrict"
    )]
    Servers,
    #[sea_orm(has_many = "super::volume_attachments::Entity")]
    VolumeAttachments,
}

impl Related<super::organizations::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Organizations.def()
    }
}

impl Related<super::servers::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Servers.def()
    }
}

impl Related<super::volume_attachments::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::VolumeAttachments.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
