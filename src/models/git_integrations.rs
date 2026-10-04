use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

use crate::services::crypto::CryptoService;

pub use super::_entities::git_integrations::{self, ActiveModel, Entity, Model};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpsertGitIntegrationParams {
    pub provider: String,
    pub repository_ref: String,
    pub api_base_url: Option<String>,
    pub token: Option<String>,
    pub webhook_secret: String,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SafeGitIntegration {
    pub id: i64,
    pub application_id: i64,
    pub provider: String,
    pub repository_ref: String,
    pub api_base_url: Option<String>,
    pub has_token: bool,
    pub enabled: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

impl Model {
    pub async fn find_for_application_provider(
        db: &DatabaseConnection,
        application_id: i64,
        provider: &str,
    ) -> Result<Model> {
        Ok(Entity::find()
            .filter(git_integrations::Column::ApplicationId.eq(application_id))
            .filter(git_integrations::Column::Provider.eq(provider))
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)?)
    }

    pub async fn list_for_application(
        db: &DatabaseConnection,
        application_id: i64,
    ) -> Result<Vec<Model>> {
        Ok(Entity::find()
            .filter(git_integrations::Column::ApplicationId.eq(application_id))
            .all(db)
            .await?)
    }

    pub async fn upsert(
        db: &DatabaseConnection,
        application_id: i64,
        params: &UpsertGitIntegrationParams,
    ) -> Result<Model> {
        let provider = normalize_provider(&params.provider)?;
        let repository_ref = params.repository_ref.trim();
        if repository_ref.is_empty() {
            return Err(Error::BadRequest(
                "repository_ref must not be empty".to_string(),
            ));
        }
        if params.webhook_secret.trim().len() < 16 {
            return Err(Error::BadRequest(
                "webhook_secret must be at least 16 characters".to_string(),
            ));
        }

        let encrypted_webhook_secret = CryptoService::encrypt(params.webhook_secret.trim())
            .map_err(|err| Error::BadRequest(err.to_string()))?;
        let encrypted_token = params
            .token
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(CryptoService::encrypt)
            .transpose()
            .map_err(|err| Error::BadRequest(err.to_string()))?;

        let existing = Entity::find()
            .filter(git_integrations::Column::ApplicationId.eq(application_id))
            .filter(git_integrations::Column::Provider.eq(&provider))
            .one(db)
            .await?;

        let now = Utc::now();
        match existing {
            Some(model) => {
                let mut active: ActiveModel = model.into();
                active.repository_ref = Set(repository_ref.to_string());
                active.api_base_url = Set(params.api_base_url.clone());
                if let Some(encrypted_token) = encrypted_token {
                    active.encrypted_token = Set(Some(encrypted_token));
                }
                active.encrypted_webhook_secret = Set(encrypted_webhook_secret);
                active.enabled = Set(params.enabled.unwrap_or(true));
                active.updated_at = Set(now.into());
                Ok(active.update(db).await?)
            }
            None => {
                let active = ActiveModel {
                    application_id: Set(application_id),
                    provider: Set(provider),
                    repository_ref: Set(repository_ref.to_string()),
                    api_base_url: Set(params.api_base_url.clone()),
                    encrypted_token: Set(encrypted_token),
                    encrypted_webhook_secret: Set(encrypted_webhook_secret),
                    enabled: Set(params.enabled.unwrap_or(true)),
                    created_at: Set(now.into()),
                    updated_at: Set(now.into()),
                    ..Default::default()
                };
                Ok(active.insert(db).await?)
            }
        }
    }

    pub fn webhook_secret(&self) -> Result<String> {
        CryptoService::decrypt(&self.encrypted_webhook_secret)
            .map_err(|err| Error::BadRequest(err.to_string()))
    }

    pub fn token(&self) -> Result<Option<String>> {
        self.encrypted_token
            .as_deref()
            .map(CryptoService::decrypt)
            .transpose()
            .map_err(|err| Error::BadRequest(err.to_string()))
    }

    pub fn to_safe(&self) -> SafeGitIntegration {
        SafeGitIntegration {
            id: self.id,
            application_id: self.application_id,
            provider: self.provider.clone(),
            repository_ref: self.repository_ref.clone(),
            api_base_url: self.api_base_url.clone(),
            has_token: self.encrypted_token.is_some(),
            enabled: self.enabled,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

pub fn normalize_provider(provider: &str) -> Result<String> {
    let normalized = provider.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "github" | "gitlab" | "gitea" => Ok(normalized),
        _ => Err(Error::BadRequest(format!(
            "unsupported Git provider '{provider}'"
        ))),
    }
}
