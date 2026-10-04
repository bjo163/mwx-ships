use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};

pub use super::_entities::deployments::{self, ActiveModel, Entity, Model};

#[derive(Debug, Deserialize, Serialize)]
pub struct TriggerDeployParams {
    pub commit_hash: Option<String>,
    pub commit_message: Option<String>,
}

impl Model {
    pub async fn find_by_id(db: &DatabaseConnection, id: i64) -> Result<Model> {
        Ok(Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)?)
    }

    pub async fn all(db: &DatabaseConnection) -> Result<Vec<Model>> {
        let list = Entity::find()
            .order_by_desc(deployments::Column::QueuedAt)
            .all(db)
            .await?;
        Ok(list)
    }

    pub async fn by_application(
        db: &DatabaseConnection,
        application_id: i64,
    ) -> Result<Vec<Model>> {
        let list = Entity::find()
            .filter(deployments::Column::ApplicationId.eq(application_id))
            .order_by_desc(deployments::Column::QueuedAt)
            .all(db)
            .await?;
        Ok(list)
    }

    pub async fn latest_for_application(
        db: &DatabaseConnection,
        application_id: i64,
    ) -> Result<Option<Model>> {
        let latest = Entity::find()
            .filter(deployments::Column::ApplicationId.eq(application_id))
            .order_by_desc(deployments::Column::QueuedAt)
            .one(db)
            .await?;
        Ok(latest)
    }

    pub async fn has_active_deployment(
        db: &DatabaseConnection,
        application_id: i64,
    ) -> Result<bool> {
        let active_statuses = vec![
            "queued".to_string(),
            "connecting".to_string(),
            "cloning".to_string(),
            "building".to_string(),
            "stopping_old".to_string(),
            "starting_new".to_string(),
            "healthchecking".to_string(),
        ];

        let count = Entity::find()
            .filter(deployments::Column::ApplicationId.eq(application_id))
            .filter(deployments::Column::Status.is_in(active_statuses))
            .count(db)
            .await?;

        Ok(count > 0)
    }

    pub async fn create_deployment(
        db: &DatabaseConnection,
        application_id: i64,
        server_id: i64,
        commit_hash: Option<String>,
        commit_message: Option<String>,
    ) -> Result<Model> {
        let now = Utc::now();
        let active = ActiveModel {
            application_id: Set(application_id),
            server_id: Set(server_id),
            commit_hash: Set(commit_hash),
            commit_message: Set(commit_message),
            status: Set("queued".to_string()),
            queued_at: Set(now.into()),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
            ..Default::default()
        };

        let model = active.insert(db).await?;
        Ok(model)
    }

    pub async fn update_status(db: &DatabaseConnection, id: i64, status: &str) -> Result<Model> {
        let dep = Self::find_by_id(db, id).await?;
        let mut active: ActiveModel = dep.into();
        let now = Utc::now();

        if (status == "connecting" || status == "cloning") && active.started_at.as_ref().is_none() {
            active.started_at = Set(Some(now.into()));
        }

        if status == "success" || status == "failed" || status == "cancelled" {
            active.finished_at = Set(Some(now.into()));
        }

        active.status = Set(status.to_string());
        active.updated_at = Set(now.into());
        let updated = active.update(db).await?;
        Ok(updated)
    }

    pub async fn attach_revision(
        db: &DatabaseConnection,
        id: i64,
        revision_id: i64,
        commit_hash: &str,
        commit_message: Option<String>,
    ) -> Result<Model> {
        let dep = Self::find_by_id(db, id).await?;
        let mut active: ActiveModel = dep.into();
        active.revision_id = Set(Some(revision_id));
        active.commit_hash = Set(Some(commit_hash.to_string()));
        active.commit_message = Set(commit_message);
        active.updated_at = Set(Utc::now().into());
        Ok(active.update(db).await?)
    }

    pub async fn record_failure(
        db: &DatabaseConnection,
        id: i64,
        error_code: &str,
        error_message: &str,
        exit_code: Option<i32>,
    ) -> Result<Model> {
        let dep = Self::find_by_id(db, id).await?;
        let mut active: ActiveModel = dep.into();
        let now = Utc::now();

        active.status = Set("failed".to_string());
        active.error_code = Set(Some(error_code.to_string()));
        active.error_message = Set(Some(error_message.to_string()));
        active.exit_code = Set(exit_code);
        active.finished_at = Set(Some(now.into()));
        active.updated_at = Set(now.into());

        let updated = active.update(db).await?;
        Ok(updated)
    }
}
