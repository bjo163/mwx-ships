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
    pub name: String,
    pub slug: Option<String>,
    pub git_repository: String,
    pub git_branch: Option<String>,
    pub build_type: Option<String>,
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
    pub build_type: Option<String>,
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

        let now = Utc::now();
        let active = ActiveModel {
            project_id: Set(params.project_id),
            environment_id: Set(params.environment_id),
            server_id: Set(params.server_id),
            name: Set(params.name.clone()),
            slug: Set(slug),
            git_repository: Set(params.git_repository.clone()),
            git_branch: Set(params
                .git_branch
                .clone()
                .unwrap_or_else(|| "main".to_string())),
            build_type: Set(params
                .build_type
                .clone()
                .unwrap_or_else(|| "dockerfile".to_string())),
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
