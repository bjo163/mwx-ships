//! `SeaORM` Entity for Servers

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "servers")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub organization_id: Option<i64>,
    pub name: String,
    pub host: String,
    pub port: i32,
    pub username: String,
    pub authentication_type: String,
    pub encrypted_private_key: Option<String>,
    pub known_host_fingerprint: Option<String>,
    pub status: String,
    pub tags_json: String,
    pub capacity_units: i32,
    pub last_seen_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::organizations::Entity",
        from = "Column::OrganizationId",
        to = "super::organizations::Column::Id",
        on_update = "NoAction",
        on_delete = "SetNull"
    )]
    Organizations,
    #[sea_orm(has_many = "super::applications::Entity")]
    Applications,
    #[sea_orm(has_many = "super::deployments::Entity")]
    Deployments,
    #[sea_orm(has_many = "super::server_health_checks::Entity")]
    ServerHealthChecks,
}

impl Related<super::organizations::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Organizations.def()
    }
}

impl Related<super::applications::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Applications.def()
    }
}

impl Related<super::deployments::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Deployments.def()
    }
}

impl Related<super::server_health_checks::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ServerHealthChecks.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
