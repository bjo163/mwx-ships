use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
};
use serde::{Deserialize, Serialize};

use super::persistent_volumes;

pub use super::_entities::volume_attachments::{self, ActiveModel, Entity, Model};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AttachVolumeParams {
    pub application_id: i64,
    pub mount_path: String,
    pub read_only: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct ResolvedVolumeAttachment {
    pub attachment: Model,
    pub volume: persistent_volumes::Model,
}

impl Model {
    pub async fn find_by_id(db: &DatabaseConnection, id: i64) -> Result<Model> {
        Ok(Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)?)
    }

    pub async fn by_volume(db: &DatabaseConnection, volume_id: i64) -> Result<Vec<Model>> {
        Ok(Entity::find()
            .filter(volume_attachments::Column::VolumeId.eq(volume_id))
            .order_by_asc(volume_attachments::Column::CreatedAt)
            .all(db)
            .await?)
    }

    pub async fn by_application(
        db: &DatabaseConnection,
        application_id: i64,
    ) -> Result<Vec<Model>> {
        Ok(Entity::find()
            .filter(volume_attachments::Column::ApplicationId.eq(application_id))
            .order_by_asc(volume_attachments::Column::MountPath)
            .all(db)
            .await?)
    }

    pub async fn resolved_for_application(
        db: &DatabaseConnection,
        application_id: i64,
    ) -> Result<Vec<ResolvedVolumeAttachment>> {
        let attachments = Self::by_application(db, application_id).await?;
        let mut resolved = Vec::with_capacity(attachments.len());
        for attachment in attachments {
            let volume = persistent_volumes::Model::find_by_id(db, attachment.volume_id).await?;
            resolved.push(ResolvedVolumeAttachment { attachment, volume });
        }
        Ok(resolved)
    }

    pub async fn attach(
        db: &DatabaseConnection,
        volume_id: i64,
        application_id: i64,
        mount_path: &str,
        read_only: bool,
    ) -> Result<Model> {
        let mount_path = normalize_mount_path(mount_path)?;

        if Entity::find()
            .filter(volume_attachments::Column::VolumeId.eq(volume_id))
            .one(db)
            .await?
            .is_some()
        {
            return Err(Error::BadRequest(
                "volume is already attached; detach it before assigning a new owner".to_string(),
            ));
        }

        if Entity::find()
            .filter(volume_attachments::Column::ApplicationId.eq(application_id))
            .filter(volume_attachments::Column::MountPath.eq(mount_path.clone()))
            .one(db)
            .await?
            .is_some()
        {
            return Err(Error::BadRequest(
                "application already has a volume at this mount path".to_string(),
            ));
        }

        let active = ActiveModel {
            volume_id: Set(volume_id),
            application_id: Set(application_id),
            mount_path: Set(mount_path),
            read_only: Set(read_only),
            created_at: Set(Utc::now().into()),
            ..Default::default()
        };
        Ok(active.insert(db).await?)
    }

    pub async fn detach(self, db: &DatabaseConnection) -> Result<()> {
        self.delete(db).await?;
        Ok(())
    }
}

pub fn normalize_mount_path(value: &str) -> Result<String> {
    let value = value.trim();
    if !value.starts_with('/') || value == "/" || value.len() > 255 {
        return Err(Error::BadRequest(
            "volume mount path must be an absolute non-root Linux path".to_string(),
        ));
    }

    let safe = value.split('/').skip(1).all(|segment| {
        !segment.is_empty()
            && !matches!(segment, "." | "..")
            && segment
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    });
    if !safe || value.contains(['\n', '\r', '\0']) {
        return Err(Error::BadRequest(
            "volume mount path contains unsupported characters or traversal".to_string(),
        ));
    }

    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_mount_path_is_absolute_and_non_traversing() {
        assert_eq!(
            normalize_mount_path("/var/lib/app-data").unwrap(),
            "/var/lib/app-data"
        );
        assert!(normalize_mount_path("var/lib/data").is_err());
        assert!(normalize_mount_path("/").is_err());
        assert!(normalize_mount_path("/var/../etc").is_err());
        assert!(normalize_mount_path("/var//data").is_err());
    }
}
