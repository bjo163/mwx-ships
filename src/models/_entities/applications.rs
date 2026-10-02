//! `SeaORM` Entity for Applications

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "applications")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,
    pub environment_id: i64,
    pub server_id: i64,
    pub name: String,
    #[sea_orm(unique)]
    pub slug: String,
    pub git_repository: String,
    pub git_branch: String,
    pub build_type: String,
    pub dockerfile_path: String,
    pub docker_context: String,
    pub docker_image: Option<String>,
    pub container_name: String,
    pub container_port: i32,
    pub published_port: Option<i32>,
    pub startup_command: Option<String>,
    pub healthcheck_path: Option<String>,
    pub healthcheck_port: Option<i32>,
    pub auto_deploy: bool,
    pub status: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::projects::Entity",
        from = "Column::ProjectId",
        to = "super::projects::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Projects,
    #[sea_orm(
        belongs_to = "super::environments::Entity",
        from = "Column::EnvironmentId",
        to = "super::environments::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Environments,
    #[sea_orm(
        belongs_to = "super::servers::Entity",
        from = "Column::ServerId",
        to = "super::servers::Column::Id",
        on_update = "NoAction",
        on_delete = "Restrict"
    )]
    Servers,
    #[sea_orm(has_many = "super::environment_variables::Entity")]
    EnvironmentVariables,
    #[sea_orm(has_many = "super::domains::Entity")]
    Domains,
    #[sea_orm(has_many = "super::deployments::Entity")]
    Deployments,
}

impl Related<super::projects::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Projects.def()
    }
}

impl Related<super::environments::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Environments.def()
    }
}

impl Related<super::servers::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Servers.def()
    }
}

impl Related<super::environment_variables::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::EnvironmentVariables.def()
    }
}

impl Related<super::domains::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Domains.def()
    }
}

impl Related<super::deployments::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Deployments.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
