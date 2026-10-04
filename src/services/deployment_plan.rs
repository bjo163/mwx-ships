use crate::{
    models::{
        _entities::{
            applications::{Column as ApplicationColumn, Entity as ApplicationEntity},
            domains::{Column as DomainColumn, Entity as DomainEntity},
        },
        applications,
        deployment_revisions,
        servers,
    },
    services::{docker::DockerService, git::GitService, proxy::ProxyService},
};
use loco_rs::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path};

#[derive(Debug, Clone, Deserialize)]
pub struct PrepareDeploymentPlanRequest {
    pub application_id: i64,
    pub commit_hash: String,
    pub commit_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DeploymentPlanSource {
    pub repository: String,
    pub branch: String,
    pub commit_hash: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DeploymentPlanTarget {
    pub server_id: i64,
    pub server_name: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DeploymentPlanEnvironmentVariable {
    pub key: String,
    pub is_secret: bool,
    pub value_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DeploymentPlanDomain {
    pub hostname: String,
    pub port: i32,
    pub https_enabled: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DeploymentPlan {
    pub plan_version: u32,
    pub plan_fingerprint: String,
    pub revision_id: i64,
    pub application_id: i64,
    pub source: DeploymentPlanSource,
    pub target: DeploymentPlanTarget,
    pub build_type: String,
    pub workload_type: String,
    pub compose_file_path: Option<String>,
    pub dockerfile_path: String,
    pub docker_context: String,
    pub image_reference: String,
    pub container_port: i32,
    pub published_port: Option<i32>,
    pub healthcheck_path: Option<String>,
    pub healthcheck_port: Option<i32>,
    pub environment: Vec<DeploymentPlanEnvironmentVariable>,
    pub domains: Vec<DeploymentPlanDomain>,
}

pub struct DeploymentPlanService;

impl DeploymentPlanService {
    pub async fn prepare(
        db: &DatabaseConnection,
        application_id: i64,
        commit_hash: &str,
        commit_message: Option<String>,
    ) -> Result<DeploymentPlan> {
        validate_commit_hash(commit_hash)?;

        let app = applications::Model::find_by_id(db, application_id).await?;
        let server = servers::Model::find_by_id(db, app.server_id).await?;

        Self::validate_application(db, &app).await?;

        let revision = deployment_revisions::Model::create_or_get(
            db,
            &app,
            &server,
            commit_hash,
            commit_message,
        )
        .await?;
        let snapshot = revision.snapshot()?;

        Ok(DeploymentPlan {
            plan_version: 1,
            plan_fingerprint: revision.revision_hash.clone(),
            revision_id: revision.id,
            application_id: app.id,
            source: DeploymentPlanSource {
                repository: snapshot.git_repository.clone(),
                branch: snapshot.git_branch.clone(),
                commit_hash: revision.source_commit_hash.clone(),
            },
            target: DeploymentPlanTarget {
                server_id: server.id,
                server_name: server.name,
            },
            build_type: snapshot.build_type.clone(),
            workload_type: snapshot.workload_type.clone(),
            compose_file_path: snapshot.compose_file_path.clone(),
            dockerfile_path: snapshot.dockerfile_path.clone(),
            docker_context: snapshot.docker_context.clone(),
            image_reference: revision.image_reference,
            container_port: snapshot.container_port,
            published_port: snapshot.published_port,
            healthcheck_path: snapshot.healthcheck_path.clone(),
            healthcheck_port: snapshot.healthcheck_port,
            environment: snapshot
                .environment
                .iter()
                .map(|variable| DeploymentPlanEnvironmentVariable {
                    key: variable.key.clone(),
                    is_secret: variable.is_secret,
                    value_fingerprint: variable.value_fingerprint.clone(),
                })
                .collect(),
            domains: snapshot
                .domains
                .iter()
                .map(|domain| DeploymentPlanDomain {
                    hostname: domain.hostname.clone(),
                    port: domain.port,
                    https_enabled: domain.https_enabled,
                })
                .collect(),
        })
    }

    async fn validate_application(
        db: &DatabaseConnection,
        app: &applications::Model,
    ) -> Result<()> {
        GitService::validate_url(&app.git_repository)
            .map_err(|error| Error::BadRequest(error.to_string()))?;
        GitService::validate_branch(&app.git_branch)
            .map_err(|error| Error::BadRequest(error.to_string()))?;
        DockerService::validate_container_name(&app.container_name)
            .map_err(|error| Error::BadRequest(error.to_string()))?;

        validate_port(app.container_port, "container_port")?;
        if let Some(port) = app.published_port {
            validate_port(port, "published_port")?;
        }
        if let Some(port) = app.healthcheck_port {
            validate_port(port, "healthcheck_port")?;
        }
        if let Some(path) = app.healthcheck_path.as_deref() {
            validate_healthcheck_path(path)?;
        }

        if !matches!(app.workload_type.as_str(), "single" | "compose") {
            return Err(Error::BadRequest(
                "workload_type must be 'single' or 'compose'".to_string(),
            ));
        }

        if app.workload_type == "compose" {
            let compose_file = app
                .compose_file_path
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    Error::BadRequest(
                        "compose_file_path is required for compose workloads".to_string(),
                    )
                })?;
            validate_relative_path(compose_file, "compose_file_path")?;
        } else {
            match app.build_type.as_str() {
                "dockerfile" => {
                    validate_relative_path(&app.dockerfile_path, "dockerfile_path")?;
                    validate_relative_path(&app.docker_context, "docker_context")?;
                }
                "prebuilt_image" => {
                    let image = app
                        .docker_image
                        .as_deref()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .ok_or_else(|| {
                            Error::BadRequest(
                                "prebuilt_image workload requires docker_image".to_string(),
                            )
                        })?;
                    DockerService::validate_image_name(image)
                        .map_err(|error| Error::BadRequest(error.to_string()))?;
                }
                other => {
                    return Err(Error::BadRequest(format!(
                        "unsupported build_type '{other}'; expected 'dockerfile' or 'prebuilt_image'"
                    )));
                }
            }
        }

        let domains = DomainEntity::find()
            .filter(DomainColumn::ApplicationId.eq(app.id))
            .all(db)
            .await?;
        for domain in &domains {
            ProxyService::validate_hostname(&domain.hostname)
                .map_err(|error| Error::BadRequest(error.to_string()))?;
            validate_port(domain.port, "domain port")?;

            let conflict = DomainEntity::find()
                .filter(DomainColumn::Hostname.eq(domain.hostname.clone()))
                .filter(DomainColumn::ApplicationId.ne(app.id))
                .one(db)
                .await?;
            if conflict.is_some() {
                return Err(Error::BadRequest(format!(
                    "domain '{}' is already assigned to another application",
                    domain.hostname
                )));
            }
        }

        if app.workload_type == "compose"
            && ProxyService::uses_managed_ingress(domains.len(), app.published_port)
        {
            return Err(Error::BadRequest(
                "managed ingress for Compose workloads is not supported; configure an explicit published port"
                    .to_string(),
            ));
        }

        if let Some(port) = app.published_port {
            let conflict = ApplicationEntity::find()
                .filter(ApplicationColumn::ServerId.eq(app.server_id))
                .filter(ApplicationColumn::PublishedPort.eq(port))
                .filter(ApplicationColumn::Id.ne(app.id))
                .one(db)
                .await?;
            if let Some(other) = conflict {
                return Err(Error::BadRequest(format!(
                    "published_port {port} conflicts with application '{}'",
                    other.name
                )));
            }
        }

        Ok(())
    }
}

fn validate_commit_hash(value: &str) -> Result<()> {
    let value = value.trim();
    if matches!(value.len(), 40 | 64) && value.chars().all(|ch| ch.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(Error::BadRequest(
            "commit_hash must be a full 40- or 64-character hexadecimal Git object ID".to_string(),
        ))
    }
}

fn validate_port(value: i32, field: &str) -> Result<()> {
    if (1..=65_535).contains(&value) {
        Ok(())
    } else {
        Err(Error::BadRequest(format!(
            "{field} must be between 1 and 65535"
        )))
    }
}

fn validate_healthcheck_path(value: &str) -> Result<()> {
    let value = value.trim();
    if value.starts_with('/')
        && value.len() <= 512
        && !value.contains(['\n', '\r', '\0'])
    {
        Ok(())
    } else {
        Err(Error::BadRequest(
            "healthcheck_path must be an absolute HTTP path".to_string(),
        ))
    }
}

fn validate_relative_path(value: &str, field: &str) -> Result<()> {
    let value = value.trim();
    let path = Path::new(value);
    if value.is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        Err(Error::BadRequest(format!(
            "{field} must be a safe relative path"
        )))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commit_hash_requires_full_git_object_id() {
        assert!(validate_commit_hash("0123456789abcdef0123456789abcdef01234567").is_ok());
        assert!(validate_commit_hash(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        )
        .is_ok());
        assert!(validate_commit_hash("deadbeef").is_err());
        assert!(validate_commit_hash("z123456789abcdef0123456789abcdef01234567").is_err());
    }

    #[test]
    fn paths_and_ports_fail_before_worker_execution() {
        assert!(validate_relative_path("Dockerfile", "dockerfile_path").is_ok());
        assert!(validate_relative_path("../Dockerfile", "dockerfile_path").is_err());
        assert!(validate_relative_path("/etc/passwd", "dockerfile_path").is_err());
        assert!(validate_port(1, "port").is_ok());
        assert!(validate_port(65_535, "port").is_ok());
        assert!(validate_port(0, "port").is_err());
        assert!(validate_port(65_536, "port").is_err());
    }

    #[test]
    fn healthcheck_path_is_http_path_only() {
        assert!(validate_healthcheck_path("/health").is_ok());
        assert!(validate_healthcheck_path("health").is_err());
        assert!(validate_healthcheck_path("/health\nHost: evil").is_err());
    }
}
