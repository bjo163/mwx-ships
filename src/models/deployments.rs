use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};

pub use super::_entities::deployments::{self, ActiveModel, Entity, Model};

pub const ACTIVE_STATUSES: &[&str] = &[
    "queued",
    "connecting",
    "cloning",
    "building",
    "stopping_old",
    "starting_new",
    "healthchecking",
];

pub const SAFE_CANCEL_STATUSES: &[&str] = &["queued", "connecting", "cloning", "building"];
pub const RETRYABLE_STATUSES: &[&str] = &["failed", "cancelled"];

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

    pub async fn latest_success_for_application(
        db: &DatabaseConnection,
        application_id: i64,
    ) -> Result<Option<Model>> {
        Ok(Entity::find()
            .filter(deployments::Column::ApplicationId.eq(application_id))
            .filter(deployments::Column::Status.eq("success"))
            .order_by_desc(deployments::Column::FinishedAt)
            .one(db)
            .await?)
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
        let count = Entity::find()
            .filter(deployments::Column::ApplicationId.eq(application_id))
            .filter(
                deployments::Column::Status
                    .is_in(ACTIVE_STATUSES.iter().map(|status| status.to_string())),
            )
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
        Self::create_deployment_attempt(
            db,
            application_id,
            server_id,
            commit_hash,
            commit_message,
            "manual",
            None,
            None,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_deployment_attempt(
        db: &DatabaseConnection,
        application_id: i64,
        server_id: i64,
        commit_hash: Option<String>,
        commit_message: Option<String>,
        trigger_kind: &str,
        source_deployment_id: Option<i64>,
        revision_id: Option<i64>,
    ) -> Result<Model> {
        let now = Utc::now();
        let active = ActiveModel {
            application_id: Set(application_id),
            server_id: Set(server_id),
            revision_id: Set(revision_id),
            trigger_kind: Set(trigger_kind.to_string()),
            source_deployment_id: Set(source_deployment_id),
            commit_hash: Set(commit_hash),
            commit_message: Set(commit_message),
            status: Set("queued".to_string()),
            queued_at: Set(now.into()),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
            ..Default::default()
        };

        Ok(active.insert(db).await?)
    }

    pub async fn transition_status_if(
        db: &DatabaseConnection,
        id: i64,
        allowed_from: &[&str],
        status: &str,
    ) -> Result<Option<Model>> {
        let now = Utc::now();
        let mut patch = ActiveModel {
            status: Set(status.to_string()),
            updated_at: Set(now.into()),
            ..Default::default()
        };

        if status == "connecting" {
            patch.started_at = Set(Some(now.into()));
        }

        if matches!(status, "success" | "failed" | "cancelled") {
            patch.finished_at = Set(Some(now.into()));
        }

        let result = Entity::update_many()
            .set(patch)
            .filter(deployments::Column::Id.eq(id))
            .filter(
                deployments::Column::Status
                    .is_in(allowed_from.iter().map(|value| value.to_string())),
            )
            .exec(db)
            .await?;

        if result.rows_affected == 0 {
            return Ok(None);
        }

        Ok(Some(Self::find_by_id(db, id).await?))
    }

    pub async fn cancel_if_safe(db: &DatabaseConnection, id: i64) -> Result<Option<Model>> {
        Self::transition_status_if(db, id, SAFE_CANCEL_STATUSES, "cancelled").await
    }

    pub fn is_retryable(&self) -> bool {
        RETRYABLE_STATUSES.contains(&self.status.as_str())
    }

    pub fn is_cancelled(&self) -> bool {
        self.status == "cancelled"
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
