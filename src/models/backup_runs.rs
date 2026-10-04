use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, QueryOrder};

pub use super::_entities::backup_runs::{self, ActiveModel, Entity, Model};

impl Model {
    pub async fn start(
        db: &DatabaseConnection,
        backup_path: &str,
        encrypted: bool,
    ) -> Result<Model> {
        let active = ActiveModel {
            backup_path: Set(backup_path.to_string()),
            status: Set("running".to_string()),
            encrypted: Set(encrypted),
            verified: Set(false),
            created_at: Set(Utc::now().into()),
            ..Default::default()
        };
        Ok(active.insert(db).await?)
    }

    pub async fn complete(
        db: &DatabaseConnection,
        id: i64,
        size_bytes: i64,
        sha256: &str,
        verified: bool,
        verification_message: Option<String>,
    ) -> Result<Model> {
        let model = Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)?;
        let mut active: ActiveModel = model.into();
        active.status = Set("success".to_string());
        active.size_bytes = Set(Some(size_bytes));
        active.sha256 = Set(Some(sha256.to_string()));
        active.verified = Set(verified);
        active.verification_message = Set(verification_message);
        active.error_message = Set(None);
        active.completed_at = Set(Some(Utc::now().into()));
        Ok(active.update(db).await?)
    }

    pub async fn fail(db: &DatabaseConnection, id: i64, error: &str) -> Result<Model> {
        let model = Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)?;
        let mut active: ActiveModel = model.into();
        active.status = Set("failed".to_string());
        active.error_message = Set(Some(error.to_string()));
        active.completed_at = Set(Some(Utc::now().into()));
        Ok(active.update(db).await?)
    }

    pub async fn recent(db: &DatabaseConnection, limit: u64) -> Result<Vec<Model>> {
        Ok(Entity::find()
            .order_by_desc(backup_runs::Column::CreatedAt)
            .limit(limit.min(100))
            .all(db)
            .await?)
    }
}
