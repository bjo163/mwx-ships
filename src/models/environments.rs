use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};

pub use super::_entities::environments::{self, ActiveModel, Entity, Model};

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateEnvironmentParams {
    pub project_id: i64,
    pub name: String,
    pub slug: Option<String>,
    pub description: Option<String>,
}

impl Model {
    pub async fn find_by_id(db: &DatabaseConnection, id: i64) -> Result<Model> {
        Ok(Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)?)
    }

    pub async fn by_project(db: &DatabaseConnection, project_id: i64) -> Result<Vec<Model>> {
        let list = Entity::find()
            .filter(environments::Column::ProjectId.eq(project_id))
            .order_by_asc(environments::Column::Name)
            .all(db)
            .await?;
        Ok(list)
    }

    pub async fn create_environment(
        db: &DatabaseConnection,
        params: &CreateEnvironmentParams,
    ) -> Result<Model> {
        let slug = params.slug.clone().unwrap_or_else(|| {
            params
                .name
                .to_lowercase()
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '-' })
                .collect()
        });

        let now = Utc::now();
        let active = ActiveModel {
            project_id: Set(params.project_id),
            name: Set(params.name.clone()),
            slug: Set(slug),
            description: Set(params.description.clone()),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
            ..Default::default()
        };

        let model = active.insert(db).await?;
        Ok(model)
    }
}
