use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
};
use serde::{Deserialize, Serialize};

pub use super::_entities::organizations::{self, ActiveModel, Entity, Model};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateOrganizationParams {
    pub name: String,
    pub slug: Option<String>,
}

impl Model {
    pub async fn find_by_id(db: &DatabaseConnection, id: i64) -> Result<Model> {
        Ok(Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)?)
    }

    pub async fn find_by_slug(db: &DatabaseConnection, slug: &str) -> Result<Model> {
        Ok(Entity::find()
            .filter(organizations::Column::Slug.eq(slug))
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)?)
    }

    pub async fn all(db: &DatabaseConnection) -> Result<Vec<Model>> {
        Ok(Entity::find()
            .order_by_asc(organizations::Column::Name)
            .all(db)
            .await?)
    }

    pub async fn create(
        db: &DatabaseConnection,
        params: &CreateOrganizationParams,
    ) -> Result<Model> {
        let name = params.name.trim();
        if name.len() < 2 {
            return Err(Error::BadRequest(
                "organization name must contain at least 2 characters".to_string(),
            ));
        }

        let slug = params
            .slug
            .as_deref()
            .map(normalize_slug)
            .unwrap_or_else(|| normalize_slug(name));
        if slug.is_empty() {
            return Err(Error::BadRequest(
                "organization slug must not be empty".to_string(),
            ));
        }

        let now = Utc::now();
        Ok(ActiveModel {
            name: Set(name.to_string()),
            slug: Set(slug),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
            ..Default::default()
        }
        .insert(db)
        .await?)
    }
}

fn normalize_slug(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
