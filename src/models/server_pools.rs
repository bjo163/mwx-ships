use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};

pub use super::_entities::server_pools::{self, ActiveModel, Entity, Model};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateServerPoolParams {
    pub name: String,
    pub slug: Option<String>,
    pub required_tags: Vec<String>,
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
            .filter(server_pools::Column::OrganizationId.eq(organization_id))
            .order_by_asc(server_pools::Column::Name)
            .all(db)
            .await?)
    }

    pub async fn create(
        db: &DatabaseConnection,
        organization_id: i64,
        params: &CreateServerPoolParams,
    ) -> Result<Model> {
        if params.name.trim().is_empty() {
            return Err(Error::BadRequest("server pool name is required".to_string()));
        }
        let slug = params.slug.clone().unwrap_or_else(|| slugify(&params.name));
        let required_tags = normalize_tags(&params.required_tags)?;
        let now = Utc::now();
        let active = ActiveModel {
            organization_id: Set(organization_id),
            name: Set(params.name.trim().to_string()),
            slug: Set(slug),
            required_tags_json: Set(serde_json::to_string(&required_tags)?),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
            ..Default::default()
        };
        Ok(active.insert(db).await?)
    }

    pub fn required_tags(&self) -> Result<Vec<String>> {
        serde_json::from_str(&self.required_tags_json)
            .map_err(|err| Error::BadRequest(format!("invalid server-pool tags: {err}")))
    }
}

pub fn normalize_tags(tags: &[String]) -> Result<Vec<String>> {
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

fn slugify(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}
