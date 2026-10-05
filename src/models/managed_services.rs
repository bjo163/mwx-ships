use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
};
use serde::{Deserialize, Serialize};

pub use super::_entities::managed_services::{self, ActiveModel, Entity, Model};

#[derive(Debug, Clone)]
pub struct CreateManagedServiceRecord {
    pub organization_id: i64,
    pub server_id: i64,
    pub volume_id: i64,
    pub name: String,
    pub slug: String,
    pub kind: String,
    pub image: String,
    pub container_name: String,
    pub internal_port: i32,
    pub database_name: Option<String>,
    pub username: Option<String>,
    pub encrypted_credentials: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManagedServiceCredentials {
    pub password: String,
    #[serde(default)]
    pub root_password: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ManagedServiceConnectionView {
    pub host: String,
    pub port: i32,
    pub database: Option<String>,
    pub username: Option<String>,
    pub password_secret_ref: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SafeManagedService {
    pub id: i64,
    pub organization_id: i64,
    pub server_id: i64,
    pub volume_id: i64,
    pub name: String,
    pub slug: String,
    pub kind: String,
    pub image: String,
    pub container_name: String,
    pub internal_port: i32,
    pub status: String,
    pub connection: ManagedServiceConnectionView,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
    pub updated_at: chrono::DateTime<chrono::FixedOffset>,
}

impl Model {
    pub async fn find_by_id(db: &DatabaseConnection, id: i64) -> Result<Model> {
        Ok(Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)?)
    }

    pub async fn all_for_organizations(
        db: &DatabaseConnection,
        organization_ids: &[i64],
    ) -> Result<Vec<Model>> {
        if organization_ids.is_empty() {
            return Ok(Vec::new());
        }
        Ok(Entity::find()
            .filter(
                managed_services::Column::OrganizationId.is_in(organization_ids.iter().copied()),
            )
            .order_by_asc(managed_services::Column::Name)
            .all(db)
            .await?)
    }

    pub async fn create_declared(
        db: &DatabaseConnection,
        record: CreateManagedServiceRecord,
    ) -> Result<Model> {
        let now = Utc::now();
        Ok(ActiveModel {
            organization_id: Set(record.organization_id),
            server_id: Set(record.server_id),
            volume_id: Set(record.volume_id),
            name: Set(record.name),
            slug: Set(record.slug),
            kind: Set(record.kind),
            image: Set(record.image),
            container_name: Set(record.container_name),
            internal_port: Set(record.internal_port),
            database_name: Set(record.database_name),
            username: Set(record.username),
            encrypted_credentials: Set(record.encrypted_credentials),
            status: Set("declared".to_string()),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
            ..Default::default()
        }
        .insert(db)
        .await?)
    }

    pub async fn update_status(db: &DatabaseConnection, id: i64, status: &str) -> Result<Model> {
        let service = Self::find_by_id(db, id).await?;
        let mut active: ActiveModel = service.into();
        active.status = Set(status.to_string());
        active.updated_at = Set(Utc::now().into());
        Ok(active.update(db).await?)
    }

    pub fn to_safe(&self) -> SafeManagedService {
        SafeManagedService {
            id: self.id,
            organization_id: self.organization_id,
            server_id: self.server_id,
            volume_id: self.volume_id,
            name: self.name.clone(),
            slug: self.slug.clone(),
            kind: self.kind.clone(),
            image: self.image.clone(),
            container_name: self.container_name.clone(),
            internal_port: self.internal_port,
            status: self.status.clone(),
            connection: ManagedServiceConnectionView {
                host: self.container_name.clone(),
                port: self.internal_port,
                database: self.database_name.clone(),
                username: self.username.clone(),
                password_secret_ref: format!("managed-service:{}:password", self.id),
            },
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
