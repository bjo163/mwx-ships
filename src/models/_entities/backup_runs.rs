use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "backup_runs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub backup_path: String,
    pub status: String,
    pub size_bytes: Option<i64>,
    pub sha256: Option<String>,
    pub encrypted: bool,
    pub verified: bool,
    pub verification_message: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub completed_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
