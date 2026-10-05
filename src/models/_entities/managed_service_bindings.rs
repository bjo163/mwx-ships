//! SeaORM entity for managed service bindings.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "managed_service_bindings")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub service_id: i64,
    pub application_id: i64,
    pub env_prefix: String,
    pub env_keys_json: String,
    pub created_at: DateTimeWithTimeZone,
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
        belongs_to = "super::applications::Entity",
        from = "Column::ApplicationId",
        to = "super::applications::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Applications,
}

impl Related<super::managed_services::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ManagedServices.def()
    }
}

impl Related<super::applications::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Applications.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
