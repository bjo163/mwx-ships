use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};

use crate::services::crypto::CryptoService;

pub use super::_entities::registry_credentials::{self, ActiveModel, Entity, Model};

#[derive(Debug, Clone, Deserialize)]
pub struct CreateRegistryCredentialParams {
    pub name: String,
    pub registry: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SafeRegistryCredential {
    pub id: i64,
    pub organization_id: i64,
    pub name: String,
    pub registry: String,
    pub username: String,
    pub has_password: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

impl Model {
    pub async fn find_by_id(db: &DatabaseConnection, id: i64) -> Result<Model> {
        Ok(Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)?)
    }

    pub async fn list_for_organization(
        db: &DatabaseConnection,
        organization_id: i64,
    ) -> Result<Vec<Model>> {
        Ok(Entity::find()
            .filter(registry_credentials::Column::OrganizationId.eq(organization_id))
            .order_by_asc(registry_credentials::Column::Name)
            .all(db)
            .await?)
    }

    pub async fn create(
        db: &DatabaseConnection,
        organization_id: i64,
        params: &CreateRegistryCredentialParams,
    ) -> Result<Model> {
        validate_registry(&params.registry)?;
        validate_username(&params.username)?;
        if params.name.trim().is_empty() || params.password.is_empty() {
            return Err(Error::BadRequest(
                "registry credential name and password are required".to_string(),
            ));
        }

        let now = Utc::now();
        let encrypted_password = CryptoService::encrypt(&params.password)
            .map_err(|err| Error::BadRequest(err.to_string()))?;
        let active = ActiveModel {
            organization_id: Set(organization_id),
            name: Set(params.name.trim().to_string()),
            registry: Set(params.registry.trim().trim_end_matches('/').to_string()),
            username: Set(params.username.trim().to_string()),
            encrypted_password: Set(encrypted_password),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
            ..Default::default()
        };
        Ok(active.insert(db).await?)
    }

    pub fn password(&self) -> Result<String> {
        CryptoService::decrypt(&self.encrypted_password)
            .map_err(|err| Error::BadRequest(err.to_string()))
    }

    pub fn to_safe(&self) -> SafeRegistryCredential {
        SafeRegistryCredential {
            id: self.id,
            organization_id: self.organization_id,
            name: self.name.clone(),
            registry: self.registry.clone(),
            username: self.username.clone(),
            has_password: !self.encrypted_password.is_empty(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

fn validate_registry(value: &str) -> Result<()> {
    let value = value.trim().trim_end_matches('/');
    if value.is_empty()
        || value.contains(char::is_whitespace)
        || value.contains(['\n', '\r', '\0'])
        || value.starts_with('-')
    {
        return Err(Error::BadRequest("registry hostname is malformed".to_string()));
    }
    Ok(())
}

fn validate_username(value: &str) -> Result<()> {
    let value = value.trim();
    if value.is_empty() || value.contains(['\n', '\r', '\0']) {
        return Err(Error::BadRequest("registry username is malformed".to_string()));
    }
    Ok(())
}
