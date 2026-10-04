//! `SeaORM` Entity for Deployments

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "deployments")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub application_id: i64,
    pub server_id: i64,
    pub revision_id: Option<i64>,
    pub trigger_kind: String,
    pub source_deployment_id: Option<i64>,
    pub execution_token: Option<String>,
    pub lease_expires_at: Option<DateTimeWithTimeZone>,
    pub attempt_count: i32,
    pub commit_hash: Option<String>,
    pub commit_message: Option<String>,
    pub status: String,
    pub queued_at: DateTimeWithTimeZone,
    pub started_at: Option<DateTimeWithTimeZone>,
    pub finished_at: Option<DateTimeWithTimeZone>,
    pub exit_code: Option<i32>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::applications::Entity",
        from = "Column::ApplicationId",
        to = "super::applications::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Applications,
    #[sea_orm(
        belongs_to = "super::servers::Entity",
        from = "Column::ServerId",
        to = "super::servers::Column::Id",
        on_update = "NoAction",
        on_delete = "Restrict"
    )]
    Servers,
    #[sea_orm(
        belongs_to = "super::deployment_revisions::Entity",
        from = "Column::RevisionId",
        to = "super::deployment_revisions::Column::Id",
        on_update = "NoAction",
        on_delete = "SetNull"
    )]
    DeploymentRevisions,
    #[sea_orm(has_many = "super::deployment_logs::Entity")]
    DeploymentLogs,
}

impl Related<super::applications::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Applications.def()
    }
}

impl Related<super::servers::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Servers.def()
    }
}

impl Related<super::deployment_logs::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DeploymentLogs.def()
    }
}

impl Related<super::deployment_revisions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DeploymentRevisions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
