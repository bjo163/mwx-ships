use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
};
use serde::{Deserialize, Serialize};

pub use super::_entities::domains::{self, ActiveModel, Entity, Model};

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateDomainParams {
    pub hostname: String,
    pub port: Option<i32>,
    pub https_enabled: Option<bool>,
}

impl Model {
    pub async fn find_by_id(db: &DatabaseConnection, id: i64) -> Result<Model> {
        Ok(Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)?)
    }

    pub async fn by_application(
        db: &DatabaseConnection,
        application_id: i64,
    ) -> Result<Vec<Model>> {
        let list = Entity::find()
            .filter(domains::Column::ApplicationId.eq(application_id))
            .order_by_asc(domains::Column::Hostname)
            .all(db)
            .await?;
        Ok(list)
    }

    pub async fn update_verification(
        db: &DatabaseConnection,
        id: i64,
        verified: bool,
        error: Option<String>,
    ) -> Result<Model> {
        let domain = Self::find_by_id(db, id).await?;
        let mut active: ActiveModel = domain.into();
        let now = Utc::now();
        active.verification_status = Set(if verified {
            "verified".to_string()
        } else {
            "failed".to_string()
        });
        active.verified_at = Set(verified.then(|| now.into()));
        active.last_error = Set(error);
        active.updated_at = Set(now.into());
        Ok(active.update(db).await?)
    }

    pub async fn update_tls_status(
        db: &DatabaseConnection,
        id: i64,
        status: &str,
        error: Option<String>,
    ) -> Result<Model> {
        let domain = Self::find_by_id(db, id).await?;
        let mut active: ActiveModel = domain.into();
        active.tls_status = Set(status.to_string());
        active.last_error = Set(error);
        active.updated_at = Set(Utc::now().into());
        Ok(active.update(db).await?)
    }

    pub async fn create_domain(
        db: &DatabaseConnection,
        application_id: i64,
        params: &CreateDomainParams,
    ) -> Result<Model> {
        let now = Utc::now();
        let active = ActiveModel {
            application_id: Set(application_id),
            hostname: Set(params.hostname.trim().to_lowercase()),
            port: Set(params.port.unwrap_or(80)),
            https_enabled: Set(params.https_enabled.unwrap_or(true)),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
            ..Default::default()
        };

        let model = active.insert(db).await?;
        Ok(model)
    }
}
