use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
};
use serde::{Deserialize, Serialize};

pub use super::_entities::managed_service_backups::{self, ActiveModel, Entity, Model};

#[derive(Debug, Clone, Deserialize)]
pub struct CreateManagedServiceBackupParams {
    pub deletion_protected: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateManagedServiceBackupProtectionParams {
    pub deletion_protected: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SafeManagedServiceBackup {
    pub id: i64,
    pub organization_id: i64,
    pub service_id: i64,
    pub server_id: i64,
    pub volume_id: i64,
    pub status: String,
    pub size_bytes: Option<i64>,
    pub sha256: Option<String>,
    pub verified: bool,
    pub verification_message: Option<String>,
    pub deletion_protected: bool,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
    pub completed_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub restored_at: Option<chrono::DateTime<chrono::FixedOffset>>,
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
            .filter(managed_service_backups::Column::ServiceId.eq(service_id))
            .order_by_desc(managed_service_backups::Column::CreatedAt)
            .all(db)
            .await?)
    }

    pub async fn start(
        db: &DatabaseConnection,
        organization_id: i64,
        service_id: i64,
        server_id: i64,
        volume_id: i64,
        artifact_path: &str,
        helper_image: &str,
        deletion_protected: bool,
    ) -> Result<Model> {
        Ok(ActiveModel {
            organization_id: Set(organization_id),
            service_id: Set(service_id),
            server_id: Set(server_id),
            volume_id: Set(volume_id),
            artifact_path: Set(artifact_path.to_string()),
            helper_image: Set(helper_image.to_string()),
            status: Set("running".to_string()),
            verified: Set(false),
            deletion_protected: Set(deletion_protected),
            created_at: Set(Utc::now().into()),
            ..Default::default()
        }
        .insert(db)
        .await?)
    }

    pub async fn complete(
        db: &DatabaseConnection,
        id: i64,
        size_bytes: i64,
        sha256: &str,
        verification_message: &str,
    ) -> Result<Model> {
        let model = Self::find_by_id(db, id).await?;
        let mut active: ActiveModel = model.into();
        active.status = Set("success".to_string());
        active.size_bytes = Set(Some(size_bytes));
        active.sha256 = Set(Some(sha256.to_string()));
        active.verified = Set(true);
        active.verification_message = Set(Some(verification_message.to_string()));
        active.error_message = Set(None);
        active.completed_at = Set(Some(Utc::now().into()));
        Ok(active.update(db).await?)
    }

    pub async fn fail(db: &DatabaseConnection, id: i64, error: &str) -> Result<Model> {
        let model = Self::find_by_id(db, id).await?;
        let mut active: ActiveModel = model.into();
        active.status = Set("failed".to_string());
        active.error_message = Set(Some(error.to_string()));
        active.completed_at = Set(Some(Utc::now().into()));
        Ok(active.update(db).await?)
    }

    pub async fn mark_restored(db: &DatabaseConnection, id: i64) -> Result<Model> {
        let model = Self::find_by_id(db, id).await?;
        let mut active: ActiveModel = model.into();
        active.restored_at = Set(Some(Utc::now().into()));
        Ok(active.update(db).await?)
    }

    pub async fn set_deletion_protection(
        db: &DatabaseConnection,
        id: i64,
        deletion_protected: bool,
    ) -> Result<Model> {
        let model = Self::find_by_id(db, id).await?;
        let mut active: ActiveModel = model.into();
        active.deletion_protected = Set(deletion_protected);
        Ok(active.update(db).await?)
    }

    pub fn to_safe(&self) -> SafeManagedServiceBackup {
        SafeManagedServiceBackup {
            id: self.id,
            organization_id: self.organization_id,
            service_id: self.service_id,
            server_id: self.server_id,
            volume_id: self.volume_id,
            status: self.status.clone(),
            size_bytes: self.size_bytes,
            sha256: self.sha256.clone(),
            verified: self.verified,
            verification_message: self.verification_message.clone(),
            deletion_protected: self.deletion_protected,
            created_at: self.created_at,
            completed_at: self.completed_at,
            restored_at: self.restored_at,
        }
    }
}
