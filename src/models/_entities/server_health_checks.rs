use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "server_health_checks")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub server_id: i64,
    pub ssh_connected: bool,
    pub docker_running: bool,
    pub disk_available_gb: Option<f64>,
    pub memory_available_mb: Option<i64>,
    pub cpu_cores: Option<i32>,
    pub healthy: bool,
    pub issues_json: String,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::servers::Entity",
        from = "Column::ServerId",
        to = "super::servers::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Servers,
}

impl Related<super::servers::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Servers.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
