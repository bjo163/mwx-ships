use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "server_pools")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub organization_id: i64,
    pub name: String,
    pub slug: String,
    pub required_tags_json: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::server_pool_members::Entity")]
    Members,
}
impl Related<super::server_pool_members::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Members.def()
    }
}
impl ActiveModelBehavior for ActiveModel {}
