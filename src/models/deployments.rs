use chrono::{Duration, Utc};
use loco_rs::prelude::*;
use sea_orm::{ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use super::_entities::deployments::{self, ActiveModel, Entity, Model};

pub const ACTIVE_STATUSES: &[&str] = &[
    "queued",
    "connecting",
    "cloning",
    "building",
    "starting_candidate",
    "switching_traffic",
    "draining_old",
    "stopping_old",
    "starting_new",
    "healthchecking",
];

pub const SAFE_CANCEL_STATUSES: &[&str] = &["queued", "connecting", "cloning", "building"];
pub const RETRYABLE_STATUSES: &[&str] = &["failed", "cancelled"];

#[derive(Debug, Clone)]
pub struct ExecutionClaim {
    pub deployment: Model,
    pub token: String,
    pub recovered_from: Option<String>,
}

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

    pub async fn find_reusable_commit_attempt(
        db: &DatabaseConnection,
        application_id: i64,
        commit_hash: &str,
        trigger_kind: &str,
    ) -> Result<Option<Model>> {
        Ok(Entity::find()
            .filter(deployments::Column::ApplicationId.eq(application_id))
            .filter(deployments::Column::CommitHash.eq(commit_hash))
            .filter(deployments::Column::TriggerKind.eq(trigger_kind))
            .filter(
                deployments::Column::Status
                    .is_not_in(["failed".to_string(), "cancelled".to_string()]),
            )
            .order_by_desc(deployments::Column::QueuedAt)
            .one(db)
            .await?)
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
        Self::transition_status_if_owned(db, id, allowed_from, status, None).await
    }

    pub async fn transition_status_if_owned(
        db: &DatabaseConnection,
        id: i64,
        allowed_from: &[&str],
        status: &str,
        execution_token: Option<&str>,
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
            patch.execution_token = Set(None);
            patch.lease_expires_at = Set(None);
        }

        let mut update = Entity::update_many()
            .set(patch)
            .filter(deployments::Column::Id.eq(id))
            .filter(
                deployments::Column::Status
                    .is_in(allowed_from.iter().map(|value| value.to_string())),
            );

        if let Some(token) = execution_token {
            update = update.filter(deployments::Column::ExecutionToken.eq(token));
        }

        let result = update.exec(db).await?;
        if result.rows_affected == 0 {
            return Ok(None);
        }

        Ok(Some(Self::find_by_id(db, id).await?))
    }

    pub async fn claim_for_execution(
        db: &DatabaseConnection,
        id: i64,
        lease_seconds: i64,
    ) -> Result<Option<ExecutionClaim>> {
        let current = Self::find_by_id(db, id).await?;
        if matches!(current.status.as_str(), "success" | "failed" | "cancelled") {
            return Ok(None);
        }
        if !ACTIVE_STATUSES.contains(&current.status.as_str()) {
            return Ok(None);
        }

        let now = Utc::now();
        let queued = current.status == "queued";
        let lease_expired = current
            .lease_expires_at
            .as_ref()
            .map(|expires| expires.timestamp() < now.timestamp())
            .unwrap_or(true);

        if !queued && !lease_expired {
            return Ok(None);
        }

        let token = Uuid::new_v4().to_string();
        let lease_expires_at = now + Duration::seconds(lease_seconds.max(60));
        let recovered_from = (!queued).then(|| current.status.clone());

        let patch = ActiveModel {
            status: Set("connecting".to_string()),
            execution_token: Set(Some(token.clone())),
            lease_expires_at: Set(Some(lease_expires_at.into())),
            attempt_count: Set(current.attempt_count.saturating_add(1)),
            started_at: Set(current.started_at.or(Some(now.into()))),
            updated_at: Set(now.into()),
            ..Default::default()
        };

        let mut update = Entity::update_many()
            .set(patch)
            .filter(deployments::Column::Id.eq(id))
            .filter(deployments::Column::Status.eq(current.status.clone()));

        update = match current.execution_token.as_deref() {
            Some(existing_token) => {
                update.filter(deployments::Column::ExecutionToken.eq(existing_token))
            }
            None => update.filter(deployments::Column::ExecutionToken.is_null()),
        };

        if !queued {
            update = match current.lease_expires_at {
                Some(_) => update.filter(deployments::Column::LeaseExpiresAt.lte(now)),
                None => update.filter(deployments::Column::LeaseExpiresAt.is_null()),
            };
        }

        let result = update.exec(db).await?;
        if result.rows_affected == 0 {
            return Ok(None);
        }

        Ok(Some(ExecutionClaim {
            deployment: Self::find_by_id(db, id).await?,
            token,
            recovered_from,
        }))
    }

    pub async fn renew_execution_lease(
        db: &DatabaseConnection,
        id: i64,
        execution_token: &str,
        lease_seconds: i64,
    ) -> Result<bool> {
        let now = Utc::now();
        let patch = ActiveModel {
            lease_expires_at: Set(Some(
                (now + Duration::seconds(lease_seconds.max(60))).into(),
            )),
            updated_at: Set(now.into()),
            ..Default::default()
        };

        let result = Entity::update_many()
            .set(patch)
            .filter(deployments::Column::Id.eq(id))
            .filter(deployments::Column::ExecutionToken.eq(execution_token))
            .filter(
                deployments::Column::Status
                    .is_in(ACTIVE_STATUSES.iter().map(|value| value.to_string())),
            )
            .exec(db)
            .await?;

        Ok(result.rows_affected == 1)
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
            active.execution_token = Set(None);
            active.lease_expires_at = Set(None);
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
        active.execution_token = Set(None);
        active.lease_expires_at = Set(None);
        active.updated_at = Set(now.into());

        let updated = active.update(db).await?;
        Ok(updated)
    }
}
