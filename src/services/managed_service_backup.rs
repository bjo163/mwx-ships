use crate::{
    models::{
        managed_service_backups::Model as ManagedServiceBackupModel,
        managed_services::{ManagedServiceCredentials, Model as ManagedServiceModel},
    },
    services::{
        docker::{ContainerConfig, DockerService},
        managed_service::ManagedServiceTemplateService,
        remote::RemoteVolumeSnapshot,
    },
};
use loco_rs::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedServiceBackupPlan {
    pub helper_image: String,
    pub artifact_path: String,
    pub verification_volume_name: String,
    pub verification_container_name: String,
}

pub struct ManagedServiceBackupService;

impl ManagedServiceBackupService {
    pub const DEFAULT_HELPER_IMAGE: &'static str = "alpine:3.22.2";

    pub fn helper_image() -> Result<String> {
        let image = std::env::var("MOONSHIPS_BACKUP_HELPER_IMAGE")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| Self::DEFAULT_HELPER_IMAGE.to_string());
        validate_pinned_image(&image)?;
        Ok(image)
    }

    pub fn plan(service_id: i64, backup_id: i64, helper_image: String) -> Result<ManagedServiceBackupPlan> {
        if service_id <= 0 || backup_id <= 0 {
            return Err(Error::BadRequest(
                "service and backup identifiers must be positive".to_string(),
            ));
        }
        validate_pinned_image(&helper_image)?;
        Ok(ManagedServiceBackupPlan {
            helper_image,
            artifact_path: format!(
                ".moonships/backups/services/{service_id}/backup-{backup_id}.tar.gz"
            ),
            verification_volume_name: format!("moonships-backup-verify-{backup_id}"),
            verification_container_name: format!(
                "moonships-backup-verify-{service_id}-{backup_id}"
            ),
        })
    }

    pub fn verification_runtime_config(
        service: &ManagedServiceModel,
        credentials: &ManagedServiceCredentials,
        plan: &ManagedServiceBackupPlan,
    ) -> Result<ContainerConfig> {
        ManagedServiceTemplateService::runtime_config_with_name(
            service,
            &plan.verification_volume_name,
            credentials,
            &plan.verification_container_name,
        )
    }

    pub fn verify_metadata(
        backup: &ManagedServiceBackupModel,
        observed: &RemoteVolumeSnapshot,
    ) -> Result<()> {
        let expected_size = backup.size_bytes.ok_or_else(|| {
            Error::BadRequest("backup has no recorded size".to_string())
        })?;
        let expected_sha = backup.sha256.as_deref().ok_or_else(|| {
            Error::BadRequest("backup has no recorded checksum".to_string())
        })?;

        if observed.artifact_path != backup.artifact_path {
            return Err(Error::BadRequest(
                "backup artifact path does not match recorded metadata".to_string(),
            ));
        }
        if observed.size_bytes != expected_size {
            return Err(Error::BadRequest(
                "backup artifact size does not match recorded metadata".to_string(),
            ));
        }
        if observed.sha256 != expected_sha {
            return Err(Error::BadRequest(
                "backup artifact checksum does not match recorded metadata".to_string(),
            ));
        }
        Ok(())
    }
}

fn validate_pinned_image(image: &str) -> Result<()> {
    DockerService::validate_image_name(image)
        .map_err(|error| Error::BadRequest(error.to_string()))?;
    if image.contains("@sha256:") {
        return Ok(());
    }
    let last = image.rsplit('/').next().unwrap_or(image);
    let Some((_, tag)) = last.rsplit_once(':') else {
        return Err(Error::BadRequest(
            "backup helper image must include an explicit version tag or digest".to_string(),
        ));
    };
    if tag.is_empty() || tag.eq_ignore_ascii_case("latest") {
        return Err(Error::BadRequest(
            "backup helper image cannot use an empty or latest tag".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_helper_is_explicitly_pinned() {
        assert!(validate_pinned_image(ManagedServiceBackupService::DEFAULT_HELPER_IMAGE).is_ok());
        assert!(validate_pinned_image("alpine").is_err());
        assert!(validate_pinned_image("alpine:latest").is_err());
    }

    #[test]
    fn backup_plan_uses_stable_scoped_names() {
        let plan = ManagedServiceBackupService::plan(
            7,
            11,
            ManagedServiceBackupService::DEFAULT_HELPER_IMAGE.to_string(),
        )
        .unwrap();
        assert_eq!(
            plan.artifact_path,
            ".moonships/backups/services/7/backup-11.tar.gz"
        );
        assert_eq!(plan.verification_volume_name, "moonships-backup-verify-11");
        assert_eq!(
            plan.verification_container_name,
            "moonships-backup-verify-7-11"
        );
    }
}
