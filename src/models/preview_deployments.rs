use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

pub use super::_entities::preview_deployments::{self, ActiveModel, Entity, Model};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpsertPreviewInput {
    pub application_id: i64,
    pub provider: String,
    pub external_request_id: String,
    pub source_ref: String,
    pub commit_sha: String,
}

impl Model {
    pub async fn find_by_request(
        db: &DatabaseConnection,
        application_id: i64,
        provider: &str,
        external_request_id: &str,
    ) -> Result<Option<Model>> {
        Ok(Entity::find()
            .filter(preview_deployments::Column::ApplicationId.eq(application_id))
            .filter(preview_deployments::Column::Provider.eq(provider))
            .filter(preview_deployments::Column::ExternalRequestId.eq(external_request_id))
            .one(db)
            .await?)
    }

    pub async fn upsert(
        db: &DatabaseConnection,
        input: &UpsertPreviewInput,
    ) -> Result<Model> {
        let existing = Self::find_by_request(
            db,
            input.application_id,
            &input.provider,
            &input.external_request_id,
        )
        .await?;

        let now = Utc::now();
        let preview_slug = format!(
            "preview-{}-{}",
            input.application_id,
            sanitize_request_id(&input.external_request_id)
        );

        match existing {
            Some(model) => {
                let mut active: ActiveModel = model.into();
                active.source_ref = Set(input.source_ref.clone());
                active.commit_sha = Set(input.commit_sha.clone());
                active.status = Set("active".to_string());
                active.updated_at = Set(now.into());
                Ok(active.update(db).await?)
            }
            None => {
                let active = ActiveModel {
                    application_id: Set(input.application_id),
                    provider: Set(input.provider.clone()),
                    external_request_id: Set(input.external_request_id.clone()),
                    source_ref: Set(input.source_ref.clone()),
                    commit_sha: Set(input.commit_sha.clone()),
                    preview_slug: Set(preview_slug),
                    status: Set("active".to_string()),
                    created_at: Set(now.into()),
                    updated_at: Set(now.into()),
                    ..Default::default()
                };
                Ok(active.insert(db).await?)
            }
        }
    }

    pub async fn close(
        db: &DatabaseConnection,
        application_id: i64,
        provider: &str,
        external_request_id: &str,
    ) -> Result<Option<Model>> {
        let Some(model) =
            Self::find_by_request(db, application_id, provider, external_request_id).await?
        else {
            return Ok(None);
        };
        let mut active: ActiveModel = model.into();
        active.status = Set("closed".to_string());
        active.updated_at = Set(Utc::now().into());
        Ok(Some(active.update(db).await?))
    }
}

fn sanitize_request_id(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>();
    sanitized.trim_matches('-').to_ascii_lowercase()
}
