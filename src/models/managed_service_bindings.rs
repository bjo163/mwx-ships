use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
};
use serde::{Deserialize, Serialize};

pub use super::_entities::managed_service_bindings::{self, ActiveModel, Entity, Model};

#[derive(Debug, Clone, Deserialize)]
pub struct CreateManagedServiceBindingParams {
    pub application_id: i64,
    pub env_prefix: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SafeManagedServiceBinding {
    pub id: i64,
    pub service_id: i64,
    pub application_id: i64,
    pub env_prefix: String,
    pub env_keys: Vec<String>,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
}

impl Model {
    pub async fn find_by_id(db: &DatabaseConnection, id: i64) -> Result<Model> {
        Ok(Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)?)
    }

    pub async fn by_service(db: &DatabaseConnection, service_id: i64) -> Result<Vec<Model>> {
        Ok(Entity::find()
            .filter(managed_service_bindings::Column::ServiceId.eq(service_id))
            .order_by_asc(managed_service_bindings::Column::CreatedAt)
            .all(db)
            .await?)
    }

    pub async fn create(
        db: &DatabaseConnection,
        service_id: i64,
        application_id: i64,
        env_prefix: String,
        env_keys: Vec<String>,
    ) -> Result<Model> {
        let env_keys_json = serde_json::to_string(&env_keys)
            .map_err(|error| Error::BadRequest(format!("binding key serialization failed: {error}")))?;
        Ok(ActiveModel {
            service_id: Set(service_id),
            application_id: Set(application_id),
            env_prefix: Set(env_prefix),
            env_keys_json: Set(env_keys_json),
            created_at: Set(Utc::now().into()),
            ..Default::default()
        }
        .insert(db)
        .await?)
    }

    pub fn env_keys(&self) -> Result<Vec<String>> {
        serde_json::from_str(&self.env_keys_json)
            .map_err(|error| Error::BadRequest(format!("invalid service binding keys: {error}")))
    }

    pub fn to_safe(&self) -> Result<SafeManagedServiceBinding> {
        Ok(SafeManagedServiceBinding {
            id: self.id,
            service_id: self.service_id,
            application_id: self.application_id,
            env_prefix: self.env_prefix.clone(),
            env_keys: self.env_keys()?,
            created_at: self.created_at,
        })
    }
}
