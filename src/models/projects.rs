use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};

pub use super::_entities::projects::{self, ActiveModel, Entity, Model};

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateProjectParams {
    pub name: String,
    pub slug: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateProjectParams {
    pub name: Option<String>,
    pub description: Option<String>,
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
            .filter(projects::Column::Slug.eq(slug))
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)?)
    }

    pub async fn all(db: &DatabaseConnection) -> Result<Vec<Model>> {
        let list = Entity::find()
            .order_by_asc(projects::Column::Name)
            .all(db)
            .await?;
        Ok(list)
    }

    pub async fn all_for_organizations(
        db: &DatabaseConnection,
        organization_ids: &[i64],
    ) -> Result<Vec<Model>> {
        if organization_ids.is_empty() {
            return Ok(Vec::new());
        }
        Ok(Entity::find()
            .filter(projects::Column::OrganizationId.is_in(organization_ids.iter().copied()))
            .order_by_asc(projects::Column::Name)
            .all(db)
            .await?)
    }

    pub async fn create_project(
        db: &DatabaseConnection,
        params: &CreateProjectParams,
    ) -> Result<Model> {
        Self::create_project_for_organization(db, params, None).await
    }

    pub async fn create_project_for_organization(
        db: &DatabaseConnection,
        params: &CreateProjectParams,
        organization_id: Option<i64>,
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
            organization_id: Set(organization_id),
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
