use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};

pub use super::_entities::applications::{self, ActiveModel, Entity, Model};

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateApplicationParams {
    pub project_id: i64,
    pub environment_id: i64,
    pub server_id: i64,
    pub server_pool_id: Option<i64>,
    pub resource_units: Option<i32>,
    pub name: String,
    pub slug: Option<String>,
    pub git_repository: String,
    pub git_branch: Option<String>,
    pub build_type: Option<String>,
    pub workload_type: Option<String>,
    pub compose_file_path: Option<String>,
    pub compose_project_name: Option<String>,
    pub registry_credential_id: Option<i64>,
    pub dockerfile_path: Option<String>,
    pub docker_context: Option<String>,
    pub docker_image: Option<String>,
    pub container_name: Option<String>,
    pub container_port: Option<i32>,
    pub published_port: Option<i32>,
    pub startup_command: Option<String>,
    pub healthcheck_path: Option<String>,
    pub healthcheck_port: Option<i32>,
    pub auto_deploy: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateApplicationParams {
    pub name: Option<String>,
    pub git_repository: Option<String>,
    pub git_branch: Option<String>,
    pub server_pool_id: Option<i64>,
    pub resource_units: Option<i32>,
    pub build_type: Option<String>,
    pub workload_type: Option<String>,
    pub compose_file_path: Option<String>,
    pub compose_project_name: Option<String>,
    pub registry_credential_id: Option<i64>,
    pub dockerfile_path: Option<String>,
    pub docker_context: Option<String>,
    pub docker_image: Option<String>,
    pub container_port: Option<i32>,
    pub published_port: Option<i32>,
    pub startup_command: Option<String>,
    pub healthcheck_path: Option<String>,
    pub healthcheck_port: Option<i32>,
    pub auto_deploy: Option<bool>,
}

impl Model {
    pub async fn find_by_id(db: &DatabaseConnection, id: i64) -> Result<Model> {
        Ok(Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)?)
    }

    pub async fn find_by_slug(db: &DatabaseConnection, slug: &str) -> Result<Model> {
        Ok(Entity::find()
            .filter(applications::Column::Slug.eq(slug))
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)?)
    }

    pub async fn all(db: &DatabaseConnection) -> Result<Vec<Model>> {
        let list = Entity::find()
            .order_by_asc(applications::Column::Name)
            .all(db)
            .await?;
        Ok(list)
    }

    pub async fn all_for_projects(
        db: &DatabaseConnection,
        project_ids: &[i64],
    ) -> Result<Vec<Model>> {
        if project_ids.is_empty() {
            return Ok(Vec::new());
        }
        Ok(Entity::find()
            .filter(applications::Column::ProjectId.is_in(project_ids.iter().copied()))
            .order_by_asc(applications::Column::Name)
            .all(db)
            .await?)
    }

    pub async fn by_project(db: &DatabaseConnection, project_id: i64) -> Result<Vec<Model>> {
        let list = Entity::find()
            .filter(applications::Column::ProjectId.eq(project_id))
            .order_by_asc(applications::Column::Name)
            .all(db)
            .await?;
        Ok(list)
    }

    pub async fn create_application(
        db: &DatabaseConnection,
        params: &CreateApplicationParams,
    ) -> Result<Model> {
        let slug = params.slug.clone().unwrap_or_else(|| {
            params
                .name
                .to_lowercase()
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '-' })
                .collect()
        });

        let container_name = params
            .container_name
            .clone()
            .unwrap_or_else(|| format!("moonships-app-{}", slug));

        let resource_units = params.resource_units.unwrap_or(1);
        if resource_units < 1 {
            return Err(Error::BadRequest(
                "resource_units must be at least 1".to_string(),
            ));
        }

        let workload_type = params
            .workload_type
            .as_deref()
            .unwrap_or("single")
            .to_ascii_lowercase();
        if !matches!(workload_type.as_str(), "single" | "compose") {
            return Err(Error::BadRequest(
                "workload_type must be 'single' or 'compose'".to_string(),
            ));
        }
        if workload_type == "compose"
            && params
                .compose_file_path
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .is_none()
        {
            return Err(Error::BadRequest(
                "compose_file_path is required for compose workloads".to_string(),
            ));
        }

        let now = Utc::now();
        let active = ActiveModel {
            project_id: Set(params.project_id),
            environment_id: Set(params.environment_id),
            server_id: Set(params.server_id),
            server_pool_id: Set(params.server_pool_id),
            resource_units: Set(resource_units),
            name: Set(params.name.clone()),
            slug: Set(slug.clone()),
            git_repository: Set(params.git_repository.clone()),
            git_branch: Set(params
                .git_branch
                .clone()
                .unwrap_or_else(|| "main".to_string())),
            build_type: Set(params
                .build_type
                .clone()
                .unwrap_or_else(|| "dockerfile".to_string())),
            workload_type: Set(workload_type),
            compose_file_path: Set(params.compose_file_path.clone()),
            compose_project_name: Set(params
                .compose_project_name
                .clone()
                .or_else(|| Some(format!("moonships-{}", slug)))),
            registry_credential_id: Set(params.registry_credential_id),
            dockerfile_path: Set(params
                .dockerfile_path
                .clone()
                .unwrap_or_else(|| "Dockerfile".to_string())),
            docker_context: Set(params
                .docker_context
                .clone()
                .unwrap_or_else(|| ".".to_string())),
            docker_image: Set(params.docker_image.clone()),
            container_name: Set(container_name),
            container_port: Set(params.container_port.unwrap_or(80)),
            published_port: Set(params.published_port),
            startup_command: Set(params.startup_command.clone()),
            healthcheck_path: Set(params.healthcheck_path.clone()),
            healthcheck_port: Set(params.healthcheck_port),
            auto_deploy: Set(params.auto_deploy.unwrap_or(false)),
            status: Set("created".to_string()),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
            ..Default::default()
        };

        let model = active.insert(db).await?;
        Ok(model)
    }

    pub async fn claim_active_deployment(
        db: &DatabaseConnection,
        id: i64,
        deployment_id: i64,
    ) -> Result<bool> {
        let patch = ActiveModel {
            active_deployment_id: Set(Some(deployment_id)),
            updated_at: Set(Utc::now().into()),
            ..Default::default()
        };

        let result = Entity::update_many()
            .set(patch)
            .filter(applications::Column::Id.eq(id))
            .filter(applications::Column::ActiveDeploymentId.is_null())
            .exec(db)
            .await?;

        Ok(result.rows_affected == 1)
    }

    pub async fn release_active_deployment(
        db: &DatabaseConnection,
        id: i64,
        deployment_id: i64,
    ) -> Result<bool> {
        let patch = ActiveModel {
            active_deployment_id: Set(None),
            updated_at: Set(Utc::now().into()),
            ..Default::default()
        };

        let result = Entity::update_many()
            .set(patch)
            .filter(applications::Column::Id.eq(id))
            .filter(applications::Column::ActiveDeploymentId.eq(deployment_id))
            .exec(db)
            .await?;

        Ok(result.rows_affected == 1)
    }

    pub async fn update_status(db: &DatabaseConnection, id: i64, status: &str) -> Result<Model> {
        let app = Self::find_by_id(db, id).await?;
        let mut active: ActiveModel = app.into();
        active.status = Set(status.to_string());
        active.updated_at = Set(Utc::now().into());
        let updated = active.update(db).await?;
        Ok(updated)
    }

    pub async fn promote_revision(
        db: &DatabaseConnection,
        id: i64,
        revision_id: i64,
    ) -> Result<Model> {
        let app = Self::find_by_id(db, id).await?;
        let runtime_name = app.active_runtime_name.clone();
        Self::promote_revision_with_runtime(db, id, revision_id, runtime_name).await
    }

    pub async fn promote_revision_with_runtime(
        db: &DatabaseConnection,
        id: i64,
        revision_id: i64,
        runtime_name: Option<String>,
    ) -> Result<Model> {
        let app = Self::find_by_id(db, id).await?;
        let current_revision_id = app.current_revision_id;
        let mut active: ActiveModel = app.into();

        if current_revision_id != Some(revision_id) {
            active.previous_revision_id = Set(current_revision_id);
            active.current_revision_id = Set(Some(revision_id));
        }

        active.active_runtime_name = Set(runtime_name);
        active.candidate_runtime_name = Set(None);
        active.status = Set("running".to_string());
        active.updated_at = Set(Utc::now().into());
        Ok(active.update(db).await?)
    }

    pub async fn set_candidate_runtime(
        db: &DatabaseConnection,
        id: i64,
        runtime_name: Option<String>,
    ) -> Result<Model> {
        let app = Self::find_by_id(db, id).await?;
        let mut active: ActiveModel = app.into();
        active.candidate_runtime_name = Set(runtime_name);
        active.updated_at = Set(Utc::now().into());
        Ok(active.update(db).await?)
    }

    pub fn resolved_runtime_name(&self) -> String {
        self.active_runtime_name
            .clone()
            .unwrap_or_else(|| self.container_name.clone())
    }
}
