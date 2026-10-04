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
    pub git_username: Option<String>,
    pub token: Option<String>,
    pub webhook_secret: String,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderCapabilities {
    pub signed_webhooks: bool,
    pub commit_status: bool,
    pub pull_request_previews: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SafeGitIntegration {
    pub id: i64,
    pub application_id: i64,
    pub provider: String,
    pub repository_ref: String,
    pub api_base_url: Option<String>,
    pub git_username: Option<String>,
    pub has_token: bool,
    pub enabled: bool,
    pub capabilities: ProviderCapabilities,
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
        let repository_ref = normalize_repository_ref(&provider, &params.repository_ref)?;
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
                active.repository_ref = Set(repository_ref.clone());
                active.api_base_url = Set(params.api_base_url.clone());
                active.git_username = Set(normalize_git_username(&provider, params.git_username.as_deref())?);
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
                    provider: Set(provider.clone()),
                    repository_ref: Set(repository_ref),
                    api_base_url: Set(params.api_base_url.clone()),
                    git_username: Set(normalize_git_username(&provider, params.git_username.as_deref())?),
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
            git_username: self.git_username.clone(),
            has_token: self.encrypted_token.is_some(),
            enabled: self.enabled,
            capabilities: provider_capabilities(&self.provider),
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

pub fn provider_capabilities(provider: &str) -> ProviderCapabilities {
    let supported = matches!(provider, "github" | "gitlab" | "gitea");
    ProviderCapabilities {
        signed_webhooks: supported,
        commit_status: supported,
        pull_request_previews: supported,
    }
}

pub fn normalize_repository_ref(provider: &str, raw: &str) -> Result<String> {
    let mut value = raw.trim().trim_end_matches('/').to_string();
    if value.is_empty()
        || value.contains(char::is_whitespace)
        || value.contains('?')
        || value.contains('#')
    {
        return Err(Error::BadRequest("repository_ref is malformed".to_string()));
    }

    let known_https = match provider {
        "github" => Some("https://github.com/"),
        "gitlab" => Some("https://gitlab.com/"),
        _ => None,
    };
    if let Some(prefix) = known_https {
        if let Some(path) = value.strip_prefix(prefix) {
            value = path.to_string();
        }
    }

    let known_ssh = match provider {
        "github" => Some("git@github.com:"),
        "gitlab" => Some("git@gitlab.com:"),
        _ => None,
    };
    if let Some(prefix) = known_ssh {
        if let Some(path) = value.strip_prefix(prefix) {
            value = path.to_string();
        }
    }

    if provider == "gitea" && (value.starts_with("http://") || value.starts_with("https://")) {
        if let Some((_, rest)) = value.split_once("://") {
            if let Some((_, path)) = rest.split_once('/') {
                value = path.to_string();
            }
        }
    } else if provider == "gitea" && value.starts_with("git@") {
        if let Some((_, path)) = value.split_once(':') {
            value = path.to_string();
        }
    }

    value = value
        .trim_matches('/')
        .trim_end_matches(".git")
        .trim_end_matches('/')
        .to_string();

    let segments = value
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if segments.len() < 2
        || segments.iter().any(|segment| {
            !segment
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        })
    {
        return Err(Error::BadRequest(
            "repository_ref must be an owner/repository or group/repository path".to_string(),
        ));
    }

    Ok(segments.join("/"))
}


pub fn normalize_git_username(provider: &str, value: Option<&str>) -> Result<Option<String>> {
    let explicit = value.map(str::trim).filter(|value| !value.is_empty());
    let username = match (provider, explicit) {
        (_, Some(value)) => value.to_string(),
        ("github", None) => "x-access-token".to_string(),
        ("gitlab", None) => "oauth2".to_string(),
        ("gitea", None) => return Ok(None),
        _ => return Ok(None),
    };

    if username.starts_with('-')
        || username.contains(char::is_whitespace)
        || username
            .chars()
            .any(|ch| matches!(ch, '\n' | '\r' | ':' | '@' | '/' | '\\'))
    {
        return Err(Error::BadRequest("git_username is malformed".to_string()));
    }

    Ok(Some(username))
}

impl Model {
    pub async fn find_for_repository(
        db: &DatabaseConnection,
        application_id: i64,
        repository: &str,
    ) -> Result<Option<Model>> {
        let integrations = Self::list_for_application(db, application_id).await?;
        for integration in integrations {
            if let Ok(normalized) = normalize_repository_ref(&integration.provider, repository) {
                if normalized == integration.repository_ref {
                    return Ok(Some(integration));
                }
            }
        }
        Ok(None)
    }

    pub fn resolved_git_username(&self) -> Result<Option<String>> {
        normalize_git_username(&self.provider, self.git_username.as_deref())
    }
}
