use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{
    applications, domains, environment_variables, registry_credentials, servers,
};

pub use super::_entities::deployment_revisions::{self, ActiveModel, Entity, Model};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RevisionEnvironmentVariable {
    pub key: String,
    pub is_secret: bool,
    pub value: Option<String>,
    #[serde(default)]
    pub encrypted_value: Option<String>,
    pub value_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RevisionDomain {
    pub hostname: String,
    pub port: i32,
    pub https_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RevisionRegistryCredential {
    pub registry: String,
    pub username: String,
    pub encrypted_password: String,
    pub password_fingerprint: String,
}

fn default_workload_type() -> String {
    "single".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeSnapshot {
    pub schema_version: u8,
    pub server_id: i64,
    pub git_repository: String,
    pub git_branch: String,
    pub build_type: String,
    #[serde(default = "default_workload_type")]
    pub workload_type: String,
    #[serde(default)]
    pub compose_file_path: Option<String>,
    #[serde(default)]
    pub compose_project_name: Option<String>,
    #[serde(default)]
    pub registry_credential: Option<RevisionRegistryCredential>,
    pub dockerfile_path: String,
    pub docker_context: String,
    pub docker_image: Option<String>,
    pub container_name: String,
    pub container_port: i32,
    pub published_port: Option<i32>,
    pub startup_command: Option<String>,
    pub healthcheck_path: Option<String>,
    pub healthcheck_port: Option<i32>,
    pub environment: Vec<RevisionEnvironmentVariable>,
    pub domains: Vec<RevisionDomain>,
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
        Ok(Entity::find()
            .filter(deployment_revisions::Column::ApplicationId.eq(application_id))
            .order_by_desc(deployment_revisions::Column::CreatedAt)
            .all(db)
            .await?)
    }

    pub async fn create_or_get(
        db: &DatabaseConnection,
        app: &applications::Model,
        server: &servers::Model,
        source_commit_hash: &str,
        source_commit_message: Option<String>,
    ) -> Result<Model> {
        let snapshot = Self::capture_snapshot(db, app, server).await?;
        let runtime_snapshot = serde_json::to_string(&snapshot).map_err(|err| {
            Error::BadRequest(format!("revision snapshot serialization failed: {err}"))
        })?;

        let identity = format!(
            "moonships-revision-v1\n{}\n{}\n{}\n{}",
            app.id, server.id, source_commit_hash, runtime_snapshot
        );
        let revision_hash = sha256_hex(&identity);

        if let Some(existing) = Entity::find()
            .filter(deployment_revisions::Column::ApplicationId.eq(app.id))
            .filter(deployment_revisions::Column::RevisionHash.eq(&revision_hash))
            .one(db)
            .await?
        {
            return Ok(existing);
        }

        let image_reference = if app.build_type == "prebuilt_image" {
            app.docker_image
                .clone()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| {
                    Error::BadRequest("prebuilt_image revision requires docker_image".to_string())
                })?
        } else {
            format!(
                "moonships/{}:rev-{}",
                app.slug,
                &revision_hash[..12.min(revision_hash.len())]
            )
        };

        let now = Utc::now();
        let active = ActiveModel {
            application_id: Set(app.id),
            server_id: Set(server.id),
            source_commit_hash: Set(source_commit_hash.to_string()),
            source_commit_message: Set(source_commit_message),
            image_reference: Set(image_reference),
            revision_hash: Set(revision_hash),
            runtime_snapshot: Set(runtime_snapshot),
            status: Set("prepared".to_string()),
            healthy_at: Set(None),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
            ..Default::default()
        };

        Ok(active.insert(db).await?)
    }

    pub async fn mark_healthy(db: &DatabaseConnection, id: i64) -> Result<Model> {
        let revision = Self::find_by_id(db, id).await?;
        let mut active: ActiveModel = revision.into();
        let now = Utc::now();
        active.status = Set("healthy".to_string());
        active.healthy_at = Set(Some(now.into()));
        active.updated_at = Set(now.into());
        Ok(active.update(db).await?)
    }

    pub async fn mark_failed(db: &DatabaseConnection, id: i64) -> Result<Model> {
        let revision = Self::find_by_id(db, id).await?;
        if revision.status == "healthy" {
            return Ok(revision);
        }
        let mut active: ActiveModel = revision.into();
        active.status = Set("failed".to_string());
        active.updated_at = Set(Utc::now().into());
        Ok(active.update(db).await?)
    }

    pub fn snapshot(&self) -> Result<RuntimeSnapshot> {
        serde_json::from_str(&self.runtime_snapshot)
            .map_err(|err| Error::BadRequest(format!("invalid revision snapshot: {err}")))
    }

    async fn capture_snapshot(
        db: &DatabaseConnection,
        app: &applications::Model,
        server: &servers::Model,
    ) -> Result<RuntimeSnapshot> {
        let environment = environment_variables::Model::by_application(db, app.id)
            .await?
            .into_iter()
            .map(|var| RevisionEnvironmentVariable {
                key: var.key,
                is_secret: var.is_secret,
                value: if var.is_secret {
                    None
                } else {
                    Some(var.encrypted_value.clone())
                },
                encrypted_value: if var.is_secret {
                    Some(var.encrypted_value.clone())
                } else {
                    None
                },
                value_fingerprint: sha256_hex(&var.encrypted_value),
            })
            .collect();

        let domains = domains::Model::by_application(db, app.id)
            .await?
            .into_iter()
            .map(|domain| RevisionDomain {
                hostname: domain.hostname,
                port: domain.port,
                https_enabled: domain.https_enabled,
            })
            .collect();

        let registry_credential = match app.registry_credential_id {
            Some(id) => {
                let credential = registry_credentials::Model::find_by_id(db, id).await?;
                Some(RevisionRegistryCredential {
                    registry: credential.registry,
                    username: credential.username,
                    password_fingerprint: sha256_hex(&credential.encrypted_password),
                    encrypted_password: credential.encrypted_password,
                })
            }
            None => None,
        };

        Ok(RuntimeSnapshot {
            schema_version: 2,
            server_id: server.id,
            git_repository: app.git_repository.clone(),
            git_branch: app.git_branch.clone(),
            build_type: app.build_type.clone(),
            workload_type: app.workload_type.clone(),
            compose_file_path: app.compose_file_path.clone(),
            compose_project_name: app.compose_project_name.clone(),
            registry_credential,
            dockerfile_path: app.dockerfile_path.clone(),
            docker_context: app.docker_context.clone(),
            docker_image: app.docker_image.clone(),
            container_name: app.container_name.clone(),
            container_port: app.container_port,
            published_port: app.published_port,
            startup_command: app.startup_command.clone(),
            healthcheck_path: app.healthcheck_path.clone(),
            healthcheck_port: app.healthcheck_port,
            environment,
            domains,
        })
    }
}

pub fn sha256_hex(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::sha256_hex;

    #[test]
    fn sha256_is_stable() {
        assert_eq!(
            sha256_hex("moonships"),
            "86ad120a7f38e8741c0ff3ebe500f72d1d66345d945396a351042c887bef7fc2"
        );
    }
}
