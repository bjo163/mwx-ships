use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

pub use super::_entities::webhook_deliveries::{self, ActiveModel, Entity, Model};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WebhookDeliveryInput {
    pub application_id: i64,
    pub provider: String,
    pub delivery_id: String,
    pub event_kind: String,
    pub source_ref: Option<String>,
    pub commit_sha: Option<String>,
    pub external_request_id: Option<String>,
    pub action: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateOrGetResult {
    pub delivery: Model,
    pub inserted: bool,
}

impl Model {
    pub async fn create_or_get(
        db: &DatabaseConnection,
        input: &WebhookDeliveryInput,
    ) -> Result<CreateOrGetResult> {
        if let Some(existing) = Self::find_deduped(
            db,
            input.application_id,
            &input.provider,
            &input.delivery_id,
        )
        .await?
        {
            return Ok(CreateOrGetResult {
                delivery: existing,
                inserted: false,
            });
        }

        let now = Utc::now();
        let active = ActiveModel {
            application_id: Set(input.application_id),
            provider: Set(input.provider.clone()),
            delivery_id: Set(input.delivery_id.clone()),
            event_kind: Set(input.event_kind.clone()),
            source_ref: Set(input.source_ref.clone()),
            commit_sha: Set(input.commit_sha.clone()),
            external_request_id: Set(input.external_request_id.clone()),
            action: Set(input.action.clone()),
            deployment_id: Set(None),
            status: Set("received".to_string()),
            error_message: Set(None),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
            ..Default::default()
        };

        match active.insert(db).await {
            Ok(delivery) => Ok(CreateOrGetResult {
                delivery,
                inserted: true,
            }),
            Err(insert_error) => {
                if let Some(existing) = Self::find_deduped(
                    db,
                    input.application_id,
                    &input.provider,
                    &input.delivery_id,
                )
                .await?
                {
                    Ok(CreateOrGetResult {
                        delivery: existing,
                        inserted: false,
                    })
                } else {
                    Err(insert_error.into())
                }
            }
        }
    }

    pub async fn find_deduped(
        db: &DatabaseConnection,
        application_id: i64,
        provider: &str,
        delivery_id: &str,
    ) -> Result<Option<Model>> {
        Ok(Entity::find()
            .filter(webhook_deliveries::Column::ApplicationId.eq(application_id))
            .filter(webhook_deliveries::Column::Provider.eq(provider))
            .filter(webhook_deliveries::Column::DeliveryId.eq(delivery_id))
            .one(db)
            .await?)
    }

    pub async fn by_deployment(
        db: &DatabaseConnection,
        deployment_id: i64,
    ) -> Result<Option<Model>> {
        Ok(Entity::find()
            .filter(webhook_deliveries::Column::DeploymentId.eq(deployment_id))
            .one(db)
            .await?)
    }

    pub async fn mark_queued(
        db: &DatabaseConnection,
        id: i64,
        deployment_id: i64,
    ) -> Result<Model> {
        Self::update_state(db, id, "queued", Some(deployment_id), None).await
    }

    pub async fn mark_ignored(db: &DatabaseConnection, id: i64) -> Result<Model> {
        Self::update_state(db, id, "ignored", None, None).await
    }

    pub async fn mark_failed(
        db: &DatabaseConnection,
        id: i64,
        message: impl Into<String>,
    ) -> Result<Model> {
        Self::update_state(db, id, "failed", None, Some(message.into())).await
    }

    pub async fn mark_completed(db: &DatabaseConnection, id: i64) -> Result<Model> {
        Self::update_state(db, id, "completed", None, None).await
    }

    async fn update_state(
        db: &DatabaseConnection,
        id: i64,
        status: &str,
        deployment_id: Option<i64>,
        error_message: Option<String>,
    ) -> Result<Model> {
        let model = Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)?;
        let mut active: ActiveModel = model.into();
        active.status = Set(status.to_string());
        if let Some(deployment_id) = deployment_id {
            active.deployment_id = Set(Some(deployment_id));
        }
        active.error_message = Set(error_message);
        active.updated_at = Set(Utc::now().into());
        Ok(active.update(db).await?)
    }
}
