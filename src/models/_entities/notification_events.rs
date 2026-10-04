use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "notification_events")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub fingerprint: String,
    pub event_kind: String,
    pub severity: String,
    pub message: String,
    pub status: String,
    pub attempts: i32,
    pub last_sent_at: Option<DateTimeWithTimeZone>,
    pub cooldown_until: Option<DateTimeWithTimeZone>,
    pub last_error: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
