//! SeaORM entity for managed stateful services.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "managed_services")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub organization_id: i64,
    pub server_id: i64,
    pub volume_id: i64,
    pub name: String,
    pub slug: String,
    pub kind: String,
    pub image: String,
    pub container_name: String,
    pub internal_port: i32,
    pub database_name: Option<String>,
    pub username: Option<String>,
    pub encrypted_credentials: String,
    pub status: String,
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
    #[sea_orm(
        belongs_to = "super::persistent_volumes::Entity",
        from = "Column::VolumeId",
        to = "super::persistent_volumes::Column::Id",
        on_update = "NoAction",
        on_delete = "Restrict"
    )]
    PersistentVolumes,
    #[sea_orm(has_many = "super::managed_service_bindings::Entity")]
    Bindings,
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

impl Related<super::persistent_volumes::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PersistentVolumes.def()
    }
}

impl Related<super::managed_service_bindings::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Bindings.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
