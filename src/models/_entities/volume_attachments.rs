//! `SeaORM` Entity for Volume Attachments

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "volume_attachments")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub volume_id: i64,
    pub application_id: i64,
    pub mount_path: String,
    pub read_only: bool,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::persistent_volumes::Entity",
        from = "Column::VolumeId",
        to = "super::persistent_volumes::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    PersistentVolumes,
    #[sea_orm(
        belongs_to = "super::applications::Entity",
        from = "Column::ApplicationId",
        to = "super::applications::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Applications,
}

impl Related<super::persistent_volumes::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PersistentVolumes.def()
    }
}

impl Related<super::applications::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Applications.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
