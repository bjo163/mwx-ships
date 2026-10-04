use chrono::{Duration, Utc};
use loco_rs::prelude::*;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub use super::_entities::api_tokens::{self, ActiveModel, Entity, Model};

const TOKEN_PREFIX: &str = "msk_";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateApiTokenParams {
    pub organization_id: i64,
    pub name: String,
    pub scopes: Vec<String>,
    pub expires_in_days: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreatedApiToken {
    pub token: String,
    pub record: Model,
}

#[derive(Debug, Clone, Serialize)]
pub struct SafeApiToken {
    pub id: i64,
    pub organization_id: i64,
    pub user_id: i64,
    pub name: String,
    pub token_prefix: String,
    pub scopes: Vec<String>,
    pub expires_at: Option<DateTimeWithTimeZone>,
    pub revoked_at: Option<DateTimeWithTimeZone>,
    pub last_used_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

impl Model {
    pub async fn create_token(
        db: &DatabaseConnection,
        user_id: i64,
        params: &CreateApiTokenParams,
    ) -> Result<CreatedApiToken> {
        let name = params.name.trim();
        if name.is_empty() {
            return Err(Error::BadRequest(
                "token name must not be empty".to_string(),
            ));
        }

        let scopes = normalize_scopes(&params.scopes)?;
        let raw = format!(
            "{TOKEN_PREFIX}{}{}",
            Uuid::new_v4().simple(),
            Uuid::new_v4().simple()
        );
        let hash = token_hash(&raw);
        let prefix = raw.chars().take(12).collect::<String>();
        let now = Utc::now();
        let expires_at = params
            .expires_in_days
            .map(|days| (now + Duration::days(days.clamp(1, 3650))).into());

        let record = ActiveModel {
            organization_id: Set(params.organization_id),
            user_id: Set(user_id),
            name: Set(name.to_string()),
            token_prefix: Set(prefix),
            token_hash: Set(hash),
            scopes: Set(serde_json::to_string(&scopes)?),
            expires_at: Set(expires_at),
            revoked_at: Set(None),
            last_used_at: Set(None),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
            ..Default::default()
        }
        .insert(db)
        .await?;

        Ok(CreatedApiToken { token: raw, record })
    }

    pub async fn authenticate(db: &DatabaseConnection, token: &str) -> Result<Option<Model>> {
        if !token.starts_with(TOKEN_PREFIX) {
            return Ok(None);
        }
        let hash = token_hash(token);
        let Some(model) = Entity::find()
            .filter(api_tokens::Column::TokenHash.eq(hash))
            .one(db)
            .await?
        else {
            return Ok(None);
        };

        let now = Utc::now().fixed_offset();
        if model.revoked_at.is_some()
            || model
                .expires_at
                .as_ref()
                .map(|value| value <= &now)
                .unwrap_or(false)
        {
            return Ok(None);
        }

        let mut active: ActiveModel = model.clone().into();
        active.last_used_at = Set(Some(now));
        active.updated_at = Set(Utc::now().into());
        let _ = active.update(db).await;
        Ok(Some(model))
    }

    pub async fn list_for_user(db: &DatabaseConnection, user_id: i64) -> Result<Vec<Model>> {
        Ok(Entity::find()
            .filter(api_tokens::Column::UserId.eq(user_id))
            .order_by_desc(api_tokens::Column::CreatedAt)
            .all(db)
            .await?)
    }

    pub async fn revoke_for_organization(
        db: &DatabaseConnection,
        id: i64,
        organization_id: i64,
    ) -> Result<Model> {
        let model = Entity::find()
            .filter(api_tokens::Column::Id.eq(id))
            .filter(api_tokens::Column::OrganizationId.eq(organization_id))
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)?;
        let mut active: ActiveModel = model.into();
        let now = Utc::now();
        active.revoked_at = Set(Some(now.into()));
        active.updated_at = Set(now.into());
        Ok(active.update(db).await?)
    }

    pub async fn revoke(db: &DatabaseConnection, id: i64, user_id: i64) -> Result<Model> {
        let model = Entity::find()
            .filter(api_tokens::Column::Id.eq(id))
            .filter(api_tokens::Column::UserId.eq(user_id))
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)?;
        let mut active: ActiveModel = model.into();
        let now = Utc::now();
        active.revoked_at = Set(Some(now.into()));
        active.updated_at = Set(now.into());
        Ok(active.update(db).await?)
    }

    pub fn to_safe(&self) -> SafeApiToken {
        SafeApiToken {
            id: self.id,
            organization_id: self.organization_id,
            user_id: self.user_id,
            name: self.name.clone(),
            token_prefix: self.token_prefix.clone(),
            scopes: self.scopes(),
            expires_at: self.expires_at,
            revoked_at: self.revoked_at,
            last_used_at: self.last_used_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    pub fn scopes(&self) -> Vec<String> {
        serde_json::from_str(&self.scopes).unwrap_or_default()
    }

    pub fn has_scope(&self, required: &str) -> bool {
        let scopes = self.scopes();
        scopes.iter().any(|scope| scope == "*" || scope == required)
    }
}

pub fn normalize_scopes(scopes: &[String]) -> Result<Vec<String>> {
    let allowed = [
        "*",
        "read",
        "deploy",
        "manage:projects",
        "manage:servers",
        "manage:applications",
        "manage:organization",
    ];

    let mut normalized = Vec::new();
    for scope in scopes {
        let scope = scope.trim().to_ascii_lowercase();
        if !allowed.contains(&scope.as_str()) {
            return Err(Error::BadRequest(format!(
                "unsupported API token scope '{scope}'"
            )));
        }
        if !normalized.contains(&scope) {
            normalized.push(scope);
        }
    }
    if normalized.is_empty() {
        normalized.push("read".to_string());
    }
    Ok(normalized)
}

fn token_hash(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}
