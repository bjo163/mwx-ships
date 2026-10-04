use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "auth_rate_limits")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub action: String,
    pub key_hash: String,
    pub attempts: i32,
    pub window_started_at: DateTimeWithTimeZone,
    pub blocked_until: Option<DateTimeWithTimeZone>,
    pub updated_at: DateTimeWithTimeZone,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
