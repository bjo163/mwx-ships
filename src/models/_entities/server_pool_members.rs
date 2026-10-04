use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "server_pool_members")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub server_pool_id: i64,
    pub server_id: i64,
    pub weight: i32,
    pub created_at: DateTimeWithTimeZone,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::server_pools::Entity",
        from = "Column::ServerPoolId",
        to = "super::server_pools::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    ServerPools,
    #[sea_orm(
        belongs_to = "super::servers::Entity",
        from = "Column::ServerId",
        to = "super::servers::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Servers,
}
impl Related<super::server_pools::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ServerPools.def()
    }
}
impl Related<super::servers::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Servers.def()
    }
}
impl ActiveModelBehavior for ActiveModel {}
