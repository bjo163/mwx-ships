use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

pub use super::_entities::organization_memberships::{self, ActiveModel, Entity, Model};

pub const OWNER: &str = "owner";
pub const ADMIN: &str = "admin";
pub const DEPLOYER: &str = "deployer";
pub const VIEWER: &str = "viewer";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SetMembershipParams {
    pub user_id: i64,
    pub role: String,
    pub is_active: Option<bool>,
}

impl Model {
    pub async fn find_for_user(
        db: &DatabaseConnection,
        organization_id: i64,
        user_id: i64,
    ) -> Result<Option<Model>> {
        Ok(Entity::find()
            .filter(organization_memberships::Column::OrganizationId.eq(organization_id))
            .filter(organization_memberships::Column::UserId.eq(user_id))
            .one(db)
            .await?)
    }

    pub async fn list_for_user(
        db: &DatabaseConnection,
        user_id: i64,
    ) -> Result<Vec<Model>> {
        Ok(Entity::find()
            .filter(organization_memberships::Column::UserId.eq(user_id))
            .filter(organization_memberships::Column::IsActive.eq(true))
            .all(db)
            .await?)
    }

    pub async fn list_for_organization(
        db: &DatabaseConnection,
        organization_id: i64,
    ) -> Result<Vec<Model>> {
        Ok(Entity::find()
            .filter(organization_memberships::Column::OrganizationId.eq(organization_id))
            .all(db)
            .await?)
    }

    pub async fn upsert(
        db: &DatabaseConnection,
        organization_id: i64,
        params: &SetMembershipParams,
    ) -> Result<Model> {
        let role = normalize_role(&params.role)?;
        let now = Utc::now();

        if let Some(existing) = Self::find_for_user(db, organization_id, params.user_id).await? {
            let mut active: ActiveModel = existing.into();
            active.role = Set(role);
            active.is_active = Set(params.is_active.unwrap_or(true));
            active.updated_at = Set(now.into());
            return Ok(active.update(db).await?);
        }

        Ok(ActiveModel {
            organization_id: Set(organization_id),
            user_id: Set(params.user_id),
            role: Set(role),
            is_active: Set(params.is_active.unwrap_or(true)),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
            ..Default::default()
        }
        .insert(db)
        .await?)
    }

    pub fn can_view(&self) -> bool {
        self.is_active && matches!(self.role.as_str(), OWNER | ADMIN | DEPLOYER | VIEWER)
    }

    pub fn can_deploy(&self) -> bool {
        self.is_active && matches!(self.role.as_str(), OWNER | ADMIN | DEPLOYER)
    }

    pub fn can_admin(&self) -> bool {
        self.is_active && matches!(self.role.as_str(), OWNER | ADMIN)
    }

    pub fn is_owner(&self) -> bool {
        self.is_active && self.role == OWNER
    }
}

pub fn normalize_role(role: &str) -> Result<String> {
    let role = role.trim().to_ascii_lowercase();
    match role.as_str() {
        OWNER | ADMIN | DEPLOYER | VIEWER => Ok(role),
        _ => Err(Error::BadRequest(format!(
            "unsupported organization role '{role}'"
        ))),
    }
}
