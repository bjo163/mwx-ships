use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};

pub use super::_entities::servers::{self, ActiveModel, Entity, Model};

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateServerParams {
    pub name: String,
    pub host: String,
    pub port: Option<i32>,
    pub username: String,
    pub authentication_type: Option<String>,
    pub private_key: Option<String>,
    pub known_host_fingerprint: Option<String>,
    pub tags: Option<Vec<String>>,
    pub capacity_units: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateServerParams {
    pub name: Option<String>,
    pub host: Option<String>,
    pub port: Option<i32>,
    pub username: Option<String>,
    pub authentication_type: Option<String>,
    pub private_key: Option<String>,
    pub known_host_fingerprint: Option<String>,
    pub tags: Option<Vec<String>>,
    pub capacity_units: Option<i32>,
}

impl Model {
    pub async fn find_by_id(db: &DatabaseConnection, id: i64) -> Result<Model> {
        Ok(Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)?)
    }

    pub async fn all(db: &DatabaseConnection) -> Result<Vec<Model>> {
        let servers = Entity::find()
            .order_by_asc(servers::Column::Name)
            .all(db)
            .await?;
        Ok(servers)
    }

    pub async fn all_for_organizations(
        db: &DatabaseConnection,
        organization_ids: &[i64],
    ) -> Result<Vec<Model>> {
        if organization_ids.is_empty() {
            return Ok(Vec::new());
        }
        Ok(Entity::find()
            .filter(servers::Column::OrganizationId.is_in(organization_ids.iter().copied()))
            .order_by_asc(servers::Column::Name)
            .all(db)
            .await?)
    }

    pub fn tags(&self) -> Result<Vec<String>> {
        serde_json::from_str(&self.tags_json)
            .map_err(|err| Error::BadRequest(format!("invalid server tags: {err}")))
    }

    pub fn normalized_tags(tags: &[String]) -> Result<Vec<String>> {
        let mut values = tags
            .iter()
            .map(|tag| tag.trim().to_ascii_lowercase())
            .filter(|tag| !tag.is_empty())
            .collect::<Vec<_>>();
        if values.iter().any(|tag| {
            tag.len() > 64
                || !tag
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        }) {
            return Err(Error::BadRequest("server tag is malformed".to_string()));
        }
        values.sort();
        values.dedup();
        Ok(values)
    }

    pub async fn update_status(db: &DatabaseConnection, id: i64, status: &str) -> Result<Model> {
        let server = Self::find_by_id(db, id).await?;
        let mut active: ActiveModel = server.into();
        active.status = Set(status.to_string());
        active.last_seen_at = Set(Some(Utc::now().into()));
        active.updated_at = Set(Utc::now().into());
        let updated = active.update(db).await?;
        Ok(updated)
    }
}
