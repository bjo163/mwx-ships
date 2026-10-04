use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "api_tokens")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub organization_id: i64,
    pub user_id: i64,
    pub name: String,
    pub token_prefix: String,
    #[serde(skip_serializing)]
    pub token_hash: String,
    pub scopes: String,
    pub expires_at: Option<DateTimeWithTimeZone>,
    pub revoked_at: Option<DateTimeWithTimeZone>,
    pub last_used_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(belongs_to = "super::organizations::Entity", from = "Column::OrganizationId", to = "super::organizations::Column::Id", on_update = "NoAction", on_delete = "Cascade")]
    Organizations,
    #[sea_orm(belongs_to = "super::users::Entity", from = "Column::UserId", to = "super::users::Column::Id", on_update = "NoAction", on_delete = "Cascade")]
    Users,
}
impl Related<super::organizations::Entity> for Entity { fn to() -> RelationDef { Relation::Organizations.def() } }
impl Related<super::users::Entity> for Entity { fn to() -> RelationDef { Relation::Users.def() } }
impl ActiveModelBehavior for ActiveModel {}
