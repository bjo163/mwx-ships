//! SeaORM entity for managed service backups.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "managed_service_backups")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub organization_id: i64,
    pub service_id: i64,
    pub server_id: i64,
    pub volume_id: i64,
    pub artifact_path: String,
    pub helper_image: String,
    pub status: String,
    pub size_bytes: Option<i64>,
    pub sha256: Option<String>,
    pub verified: bool,
    pub verification_message: Option<String>,
    pub error_message: Option<String>,
    pub deletion_protected: bool,
    pub created_at: DateTimeWithTimeZone,
    pub completed_at: Option<DateTimeWithTimeZone>,
    pub restored_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::managed_services::Entity",
        from = "Column::ServiceId",
        to = "super::managed_services::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    ManagedServices,
    #[sea_orm(
        belongs_to = "super::persistent_volumes::Entity",
        from = "Column::VolumeId",
        to = "super::persistent_volumes::Column::Id",
        on_update = "NoAction",
        on_delete = "Restrict"
    )]
    PersistentVolumes,
    #[sea_orm(
        belongs_to = "super::servers::Entity",
        from = "Column::ServerId",
        to = "super::servers::Column::Id",
        on_update = "NoAction",
        on_delete = "Restrict"
    )]
    Servers,
    #[sea_orm(
        belongs_to = "super::organizations::Entity",
        from = "Column::OrganizationId",
        to = "super::organizations::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Organizations,
}

impl Related<super::managed_services::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ManagedServices.def()
    }
}

impl Related<super::persistent_volumes::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PersistentVolumes.def()
    }
}

impl Related<super::servers::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Servers.def()
    }
}

impl Related<super::organizations::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Organizations.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
