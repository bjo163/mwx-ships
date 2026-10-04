use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder,
};
use serde::{Deserialize, Serialize};

use super::_entities::volume_attachments as volume_attachments_entity;

pub use super::_entities::persistent_volumes::{self, ActiveModel, Entity, Model};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreatePersistentVolumeParams {
    pub server_id: i64,
    pub name: String,
    pub deletion_protected: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdateVolumeProtectionParams {
    pub deletion_protected: bool,
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
                persistent_volumes::Column::OrganizationId
                    .is_in(organization_ids.iter().copied()),
            )
            .order_by_asc(persistent_volumes::Column::Name)
            .all(db)
            .await?)
    }

    pub async fn create_declared(
        db: &DatabaseConnection,
        organization_id: i64,
        server_id: i64,
        name: &str,
        deletion_protected: bool,
    ) -> Result<Model> {
        let name = normalize_display_name(name)?;
        let duplicate = Entity::find()
            .filter(persistent_volumes::Column::OrganizationId.eq(organization_id))
            .filter(persistent_volumes::Column::ServerId.eq(server_id))
            .filter(persistent_volumes::Column::Name.eq(name.clone()))
            .one(db)
            .await?;
        if duplicate.is_some() {
            return Err(Error::BadRequest(
                "a volume with this name already exists on the selected server".to_string(),
            ));
        }

        let now = Utc::now();
        let docker_volume_name = format!("moonships-v-{}", uuid::Uuid::new_v4().simple());
        let active = ActiveModel {
            organization_id: Set(organization_id),
            server_id: Set(server_id),
            name: Set(name),
            docker_volume_name: Set(docker_volume_name),
            deletion_protected: Set(deletion_protected),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
            ..Default::default()
        };
        Ok(active.insert(db).await?)
    }

    pub async fn set_deletion_protection(
        db: &DatabaseConnection,
        id: i64,
        deletion_protected: bool,
    ) -> Result<Model> {
        let volume = Self::find_by_id(db, id).await?;
        let mut active: ActiveModel = volume.into();
        active.deletion_protected = Set(deletion_protected);
        active.updated_at = Set(Utc::now().into());
        Ok(active.update(db).await?)
    }

    pub async fn attachment_count(&self, db: &DatabaseConnection) -> Result<u64> {
        Ok(volume_attachments_entity::Entity::find()
            .filter(volume_attachments_entity::Column::VolumeId.eq(self.id))
            .count(db)
            .await?)
    }

    pub async fn delete_record(self, db: &DatabaseConnection) -> Result<()> {
        if self.attachment_count(db).await? > 0 {
            return Err(Error::BadRequest(
                "attached volume cannot be deleted; detach it first".to_string(),
            ));
        }
        if self.deletion_protected {
            return Err(Error::BadRequest(
                "volume deletion protection is enabled".to_string(),
            ));
        }
        self.delete(db).await?;
        Ok(())
    }
}

pub fn normalize_display_name(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 64
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ' ' | '-' | '_' | '.'))
    {
        return Err(Error::BadRequest(
            "volume name must be 1-64 characters using letters, numbers, spaces, '.', '_' or '-'"
                .to_string(),
        ));
    }
    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_display_name_is_bounded_and_safe() {
        assert_eq!(
            normalize_display_name(" Database Data ").unwrap(),
            "Database Data"
        );
        assert!(normalize_display_name("").is_err());
        assert!(normalize_display_name("data; rm -rf /").is_err());
    }
}
