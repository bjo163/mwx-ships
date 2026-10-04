use crate::models::{
    applications, deployment_logs, deployment_revisions, deployments, servers,
};
use crate::services::{
    crypto::CryptoService,
    docker::ContainerConfig,
    proxy::ProxyService,
    remote::{redact_secrets, RemoteRuntime},
};
use loco_rs::prelude::*;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, thiserror::Error)]
pub enum DeploymentError {
    #[error("Deployment is already running for this application (409 Conflict)")]
    Conflict,
    #[error("Application not found: {0}")]
    AppNotFound(i64),
    #[error("Server not found: {0}")]
    ServerNotFound(i64),
    #[error("Deployment step failed ({step}): {message}")]
    StepFailed { step: String, message: String },
    #[error("Healthcheck failed after retries: {0}")]
    HealthcheckFailed(String),
    #[error("Deployment was cancelled")]
    Cancelled,
    #[error("Deployment cannot be cancelled from status '{status}'")]
    CancelNotAllowed { status: String },
    #[error("Deployment cannot be retried from status '{status}'")]
    RetryNotAllowed { status: String },
}

pub struct DeploymentService;

impl DeploymentService {
    pub async fn trigger_deploy(
        db: &DatabaseConnection,
        app_id: i64,
        commit_hash: Option<String>,
        commit_message: Option<String>,
    ) -> Result<deployments::Model, DeploymentError> {
        let app = applications::Model::find_by_id(db, app_id)
            .await
            .map_err(|_| DeploymentError::AppNotFound(app_id))?;
        let server = servers::Model::find_by_id(db, app.server_id)
            .await
            .map_err(|_| DeploymentError::ServerNotFound(app.server_id))?;

        if deployments::Model::has_active_deployment(db, app_id)
            .await
            .map_err(|e| DeploymentError::StepFailed {
                step: "lock_check".to_string(),
                message: e.to_string(),
            })?
        {
            return Err(DeploymentError::Conflict);
        }

        let dep = deployments::Model::create_deployment(
            db,
            app.id,
            server.id,
            commit_hash,
            commit_message,
        )
        .await
        .map_err(|e| DeploymentError::StepFailed {
            step: "create_deployment".to_string(),
            message: e.to_string(),
        })?;

        let _ = applications::Model::update_status(db, app.id, "queued").await;
        let _ = deployment_logs::Model::append(
            db,
            dep.id,
            "system",
            &format!(
                "Deployment #{} queued for application '{}' on target server '{}' ({}@{}:{})",
                dep.id, app.name, server.name, server.username, server.host, server.port
            ),
        )
        .await;

        Ok(dep)
    }

    pub async fn cancel_deployment(
        db: &DatabaseConnection,
        deployment_id: i64,
    ) -> Result<deployments::Model, DeploymentError> {
        let deployment = deployments::Model::find_by_id(db, deployment_id)
            .await
            .map_err(|err| DeploymentError::StepFailed {
                step: "load_deployment".to_string(),
                message: err.to_string(),
            })?;

        match deployments::Model::cancel_if_safe(db, deployment_id)
            .await
            .map_err(|err| DeploymentError::StepFailed {
                step: "cancel_deployment".to_string(),
                message: err.to_string(),
            })?
        {
            Some(cancelled) => {
                let _ = deployment_logs::Model::append(
                    db,
                    deployment_id,
                    "system",
                    "Deployment cancelled before destructive replacement began",
                )
                .await;

                if let Ok(app) = applications::Model::find_by_id(db, deployment.application_id).await
                {
                    let restored_status = if app.current_revision_id.is_some() {
                        "running"
                    } else {
                        "stopped"
                    };
                    let _ =
                        applications::Model::update_status(db, app.id, restored_status).await;
                }

                Ok(cancelled)
            }
            None => {
                let latest = deployments::Model::find_by_id(db, deployment_id)
                    .await
                    .map_err(|err| DeploymentError::StepFailed {
                        step: "reload_deployment".to_string(),
                        message: err.to_string(),
                    })?;
                Err(DeploymentError::CancelNotAllowed {
                    status: latest.status,
                })
            }
        }
    }

    pub async fn retry_deployment(
        db: &DatabaseConnection,
        deployment_id: i64,
    ) -> Result<deployments::Model, DeploymentError> {
        let original = deployments::Model::find_by_id(db, deployment_id)
            .await
            .map_err(|err| DeploymentError::StepFailed {
                step: "load_deployment".to_string(),
                message: err.to_string(),
            })?;

        if !original.is_retryable() {
            return Err(DeploymentError::RetryNotAllowed {
                status: original.status,
            });
        }

        if deployments::Model::has_active_deployment(db, original.application_id)
            .await
            .map_err(|err| DeploymentError::StepFailed {
                step: "retry_lock_check".to_string(),
                message: err.to_string(),
            })?
        {
            return Err(DeploymentError::Conflict);
        }

        let retry = deployments::Model::create_deployment_attempt(
            db,
            original.application_id,
            original.server_id,
            original.commit_hash.clone(),
            original.commit_message.clone(),
            "retry",
            Some(original.id),
            original.revision_id,
        )
        .await
        .map_err(|err| DeploymentError::StepFailed {
            step: "create_retry".to_string(),
            message: err.to_string(),
        })?;

        let _ = applications::Model::update_status(db, original.application_id, "queued").await;
        let _ = deployment_logs::Model::append(
            db,
            retry.id,
            "system",
            &format!(
                "Retry queued from deployment #{}{}",
                original.id,
                original
                    .revision_id
                    .map(|revision_id| format!(" using immutable revision #{revision_id}"))
                    .unwrap_or_default()
            ),
        )
        .await;

        Ok(retry)
    }

    pub async fn execute_deployment(
        db: &DatabaseConnection,
        deployment_id: i64,
    ) -> Result<(), DeploymentError> {
        let dep = deployments::Model::find_by_id(db, deployment_id)
            .await
            .map_err(|e| DeploymentError::StepFailed {
                step: "load_deployment".to_string(),
                message: e.to_string(),
            })?;

        let app = applications::Model::find_by_id(db, dep.application_id)
            .await
            .map_err(|_| DeploymentError::AppNotFound(dep.application_id))?;
        let server = servers::Model::find_by_id(db, dep.server_id)
            .await
            .map_err(|_| DeploymentError::ServerNotFound(dep.server_id))?;

        let requested_revision = match dep.revision_id {
            Some(revision_id) => {
                let revision = deployment_revisions::Model::find_by_id(db, revision_id)
                    .await
                    .map_err(|err| DeploymentError::StepFailed {
                        step: "load_requested_revision".to_string(),
                        message: err.to_string(),
                    })?;
                if revision.application_id != app.id || revision.server_id != server.id {
                    return Err(DeploymentError::StepFailed {
                        step: "validate_requested_revision".to_string(),
                        message: "requested revision does not belong to this application/server"
                            .to_string(),
                    });
                }
                Some(revision)
            }
            None => None,
        };

        Self::transition_phase(db, dep.id, &["queued"], "connecting").await?;
        let _ = applications::Model::update_status(db, app.id, "connecting").await;
        let _ = deployment_logs::Model::append(
            db,
            dep.id,
            "system",
            &format!(
                "Connecting to selected target server '{}' ({}@{}:{})",
                server.name, server.username, server.host, server.port
            ),
        )
        .await;

        let runtime = match RemoteRuntime::connect(&server).await {
            Ok(runtime) => runtime,
            Err(err) => {
                let message = err.to_string();
                return Err(Self::record_failure(
                    db,
                    dep.id,
                    app.id,
                    "SSH_CONNECT_FAILED",
                    "connecting",
                    &message,
                    None,
                )
                .await);
            }
        };

        if let Err(err) = runtime.ensure_docker().await {
            let exit_code = err.exit_code();
            let message = err.to_string();
            return Err(Self::record_failure(
                db,
                dep.id,
                app.id,
                "REMOTE_DOCKER_UNAVAILABLE",
                "connecting",
                &message,
                exit_code,
            )
            .await);
        }

        let _ = deployment_logs::Model::append(
            db,
            dep.id,
            "stdout",
            &format!(
                "Connected to {}; remote Docker is available",
                runtime.target()
            ),
        )
        .await;

        Self::transition_phase(db, dep.id, &["connecting"], "cloning").await?;
        let _ = applications::Model::update_status(db, app.id, "cloning").await;
        let _ = deployment_logs::Model::append(
            db,
            dep.id,
            "system",
            &format!("Synchronizing branch '{}' on target server", app.git_branch),
        )
        .await;

        let requested_snapshot = match requested_revision.as_ref() {
            Some(revision) => Some(revision.snapshot().map_err(|err| DeploymentError::StepFailed {
                step: "decode_requested_revision".to_string(),
                message: err.to_string(),
            })?),
            None => None,
        };

        let requested_commit = requested_revision
            .as_ref()
            .map(|revision| revision.source_commit_hash.as_str())
            .or(dep.commit_hash.as_deref());

        let (git_repository, git_branch) = requested_snapshot
            .as_ref()
            .map(|snapshot| {
                (
                    snapshot.git_repository.as_str(),
                    snapshot.git_branch.as_str(),
                )
            })
            .unwrap_or((&app.git_repository, &app.git_branch));

        let (commit_sha, commit_message) = match runtime
            .sync_repository_config(app.id, git_repository, git_branch, requested_commit)
            .await
        {
            Ok(revision) => revision,
            Err(err) => {
                let exit_code = err.exit_code();
                let message = err.to_string();
                return Err(Self::record_failure(
                    db,
                    dep.id,
                    app.id,
                    "REMOTE_GIT_FAILED",
                    "cloning",
                    &message,
                    exit_code,
                )
                .await);
            }
        };

        let _ = deployment_logs::Model::append(
            db,
            dep.id,
            "stdout",
            &format!(
                "Target checked out commit {} - {}",
                &commit_sha[..7.min(commit_sha.len())],
                commit_message
            ),
        )
        .await;

        let revision = if let Some(revision) = requested_revision {
            if revision.source_commit_hash != commit_sha {
                return Err(Self::record_failure(
                    db,
                    dep.id,
                    app.id,
                    "REVISION_SOURCE_MISMATCH",
                    "cloning",
                    "requested revision commit does not match checked-out commit",
                    None,
                )
                .await);
            }
            revision
        } else {
            match deployment_revisions::Model::create_or_get(
                db,
                &app,
                &server,
                &commit_sha,
                Some(commit_message.clone()),
            )
            .await
            {
                Ok(revision) => revision,
                Err(err) => {
                    return Err(Self::record_failure(
                        db,
                        dep.id,
                        app.id,
                        "REVISION_SNAPSHOT_FAILED",
                        "cloning",
                        &err.to_string(),
                        None,
                    )
                    .await);
                }
            }
        };

        let revision_snapshot = revision
            .snapshot()
            .map_err(|err| DeploymentError::StepFailed {
                step: "decode_revision_snapshot".to_string(),
                message: err.to_string(),
            })?;

        if let Err(err) = deployments::Model::attach_revision(
            db,
            dep.id,
            revision.id,
            &commit_sha,
            Some(commit_message.clone()),
        )
        .await
        {
            return Err(Self::record_failure(
                db,
                dep.id,
                app.id,
                "REVISION_BIND_FAILED",
                "cloning",
                &err.to_string(),
                None,
            )
            .await);
        }

        let _ = deployment_logs::Model::append(
            db,
            dep.id,
            "system",
            &format!(
                "Prepared immutable revision {} using image '{}'",
                &revision.revision_hash[..12.min(revision.revision_hash.len())],
                revision.image_reference
            ),
        )
        .await;

        Self::transition_phase(db, dep.id, &["cloning"], "building").await?;
        let _ = applications::Model::update_status(db, app.id, "building").await;

        let build_result = if revision_snapshot.build_type == "prebuilt_image" {
            runtime.pull_image(&revision.image_reference).await
        } else {
            runtime
                .build_image_config(
                    app.id,
                    &revision_snapshot.docker_context,
                    &revision_snapshot.dockerfile_path,
                    &revision.image_reference,
                )
                .await
        };

        if let Err(err) = build_result {
            let exit_code = err.exit_code();
            let message = err.to_string();
            let error_code = if revision_snapshot.build_type == "prebuilt_image" {
                "REMOTE_DOCKER_PULL_FAILED"
            } else {
                "REMOTE_DOCKER_BUILD_FAILED"
            };
            return Err(Self::record_failure(
                db, dep.id, app.id, error_code, "building", &message, exit_code,
            )
            .await);
        }

        let _ = deployment_logs::Model::append(
            db,
            dep.id,
            "stdout",
            &format!("Image prepared successfully on {}", runtime.target()),
        )
        .await;

        Self::transition_phase(db, dep.id, &["building"], "stopping_old").await?;
        let _ = deployment_logs::Model::append(
            db,
            dep.id,
            "system",
            &format!(
                "Replacing existing container '{}' on target server",
                app.container_name
            ),
        )
        .await;

        if let Err(err) = runtime.stop_and_remove_container(&app.container_name).await {
            let exit_code = err.exit_code();
            let message = err.to_string();
            return Err(Self::record_failure(
                db,
                dep.id,
                app.id,
                "REMOTE_DOCKER_REPLACE_FAILED",
                "stopping_old",
                &message,
                exit_code,
            )
            .await);
        }

        Self::transition_phase(db, dep.id, &["stopping_old"], "starting_new").await?;
        let _ = applications::Model::update_status(db, app.id, "starting").await;

        let mut decrypted_envs = Vec::new();
        let mut secret_values = Vec::new();

        for env in &revision_snapshot.environment {
            let value = if env.is_secret {
                let encrypted = env.encrypted_value.as_deref().ok_or_else(|| {
                    DeploymentError::StepFailed {
                        step: "revision_secret_resolve".to_string(),
                        message: format!(
                            "revision secret '{}' has no encrypted value",
                            env.key
                        ),
                    }
                })?;

                if deployment_revisions::sha256_hex(encrypted) != env.value_fingerprint {
                    return Err(Self::record_failure(
                        db,
                        dep.id,
                        app.id,
                        "REVISION_SECRET_INTEGRITY_FAILED",
                        "starting_new",
                        &format!("encrypted revision secret '{}' failed integrity check", env.key),
                        None,
                    )
                    .await);
                }

                match CryptoService::decrypt(encrypted) {
                    Ok(value) => {
                        secret_values.push(value.clone());
                        value
                    }
                    Err(err) => {
                        return Err(Self::record_failure(
                            db,
                            dep.id,
                            app.id,
                            "SECRET_DECRYPT_FAILED",
                            "starting_new",
                            &err.to_string(),
                            None,
                        )
                        .await);
                    }
                }
            } else {
                let value = env.value.clone().ok_or_else(|| DeploymentError::StepFailed {
                    step: "revision_env_resolve".to_string(),
                    message: format!("revision environment '{}' has no value", env.key),
                })?;
                if deployment_revisions::sha256_hex(&value) != env.value_fingerprint {
                    return Err(Self::record_failure(
                        db,
                        dep.id,
                        app.id,
                        "REVISION_ENV_INTEGRITY_FAILED",
                        "starting_new",
                        &format!("revision environment '{}' failed integrity check", env.key),
                        None,
                    )
                    .await);
                }
                value
            };
            decrypted_envs.push((env.key.clone(), value));
        }

        let domain_names: Vec<String> = revision_snapshot
            .domains
            .iter()
            .map(|domain| domain.hostname.clone())
            .collect();
        let has_https = revision_snapshot
            .domains
            .iter()
            .any(|domain| domain.https_enabled);
        let labels = ProxyService::generate_traefik_labels(
            &app.slug,
            &domain_names,
            revision_snapshot.container_port,
            has_https,
        );

        let container_config = ContainerConfig {
            name: revision_snapshot.container_name.clone(),
            image: revision.image_reference.clone(),
            container_port: revision_snapshot.container_port,
            published_port: revision_snapshot.published_port,
            env_vars: decrypted_envs,
            labels,
            restart_policy: "unless-stopped".to_string(),
            network: None,
        };

        match runtime.run_container(app.id, &container_config).await {
            Ok(container_id) => {
                let _ = deployment_logs::Model::append(
                    db,
                    dep.id,
                    "stdout",
                    &format!(
                        "Remote container started with ID {} on {}",
                        &container_id[..12.min(container_id.len())],
                        runtime.target()
                    ),
                )
                .await;
            }
            Err(err) => {
                let exit_code = err.exit_code();
                let message = redact_secrets(&err.to_string(), &secret_values);
                return Err(Self::record_failure(
                    db,
                    dep.id,
                    app.id,
                    "REMOTE_DOCKER_RUN_FAILED",
                    "starting_new",
                    &message,
                    exit_code,
                )
                .await);
            }
        }

        if let Some(path) = &revision_snapshot.healthcheck_path {
            Self::transition_phase(db, dep.id, &["starting_new"], "healthchecking").await?;
            let _ = applications::Model::update_status(db, app.id, "healthchecking").await;
            let _ = deployment_logs::Model::append(
                db,
                dep.id,
                "system",
                &format!("Running healthcheck on target server at path '{path}'"),
            )
            .await;

            let retries = 5;
            let mut last_error = None;
            let host_port = revision_snapshot
                .healthcheck_port
                .or(revision_snapshot.published_port);

            for attempt in 1..=retries {
                sleep(Duration::from_secs(2)).await;
                match runtime
                    .healthcheck_once(
                        &revision_snapshot.container_name,
                        host_port,
                        revision_snapshot.container_port,
                        path,
                    )
                    .await
                {
                    Ok(()) => {
                        let _ = deployment_logs::Model::append(
                            db,
                            dep.id,
                            "stdout",
                            &format!("Healthcheck passed on attempt {attempt}/{retries}"),
                        )
                        .await;
                        last_error = None;
                        break;
                    }
                    Err(err) => {
                        last_error = Some(err);
                    }
                }
            }

            if let Some(err) = last_error {
                let exit_code = err.exit_code();
                let message = redact_secrets(&err.to_string(), &secret_values);
                let _ = Self::record_failure(
                    db,
                    dep.id,
                    app.id,
                    "REMOTE_HEALTHCHECK_FAILED",
                    "healthchecking",
                    &message,
                    exit_code,
                )
                .await;
                return Err(DeploymentError::HealthcheckFailed(message));
            }
        }

        if let Err(err) = deployment_revisions::Model::mark_healthy(db, revision.id).await {
            return Err(Self::record_failure(
                db,
                dep.id,
                app.id,
                "REVISION_FINALIZE_FAILED",
                "healthchecking",
                &err.to_string(),
                None,
            )
            .await);
        }

        if let Err(err) = applications::Model::promote_revision(db, app.id, revision.id).await {
            return Err(Self::record_failure(
                db,
                dep.id,
                app.id,
                "REVISION_PROMOTE_FAILED",
                "healthchecking",
                &err.to_string(),
                None,
            )
            .await);
        }

        let success_from = if revision_snapshot.healthcheck_path.is_some() {
            &["healthchecking"][..]
        } else {
            &["starting_new"][..]
        };
        Self::transition_phase(db, dep.id, success_from, "success").await?;
        let _ = deployment_logs::Model::append(
            db,
            dep.id,
            "system",
            &format!(
                "Deployment #{} completed successfully on target server '{}' with revision {}",
                dep.id,
                server.name,
                &revision.revision_hash[..12.min(revision.revision_hash.len())]
            ),
        )
        .await;

        Ok(())
    }

    async fn transition_phase(
        db: &DatabaseConnection,
        deployment_id: i64,
        allowed_from: &[&str],
        next_status: &str,
    ) -> Result<deployments::Model, DeploymentError> {
        match deployments::Model::transition_status_if(
            db,
            deployment_id,
            allowed_from,
            next_status,
        )
        .await
        .map_err(|err| DeploymentError::StepFailed {
            step: "transition_status".to_string(),
            message: err.to_string(),
        })? {
            Some(deployment) => Ok(deployment),
            None => {
                let current = deployments::Model::find_by_id(db, deployment_id)
                    .await
                    .map_err(|err| DeploymentError::StepFailed {
                        step: "reload_transition".to_string(),
                        message: err.to_string(),
                    })?;
                if current.is_cancelled() {
                    Err(DeploymentError::Cancelled)
                } else {
                    Err(DeploymentError::StepFailed {
                        step: "transition_status".to_string(),
                        message: format!(
                            "cannot transition deployment #{} from '{}' to '{}'",
                            deployment_id, current.status, next_status
                        ),
                    })
                }
            }
        }
    }

    async fn record_failure(
        db: &DatabaseConnection,
        deployment_id: i64,
        application_id: i64,
        error_code: &str,
        step: &str,
        message: &str,
        exit_code: Option<i32>,
    ) -> DeploymentError {
        if let Ok(current) = deployments::Model::find_by_id(db, deployment_id).await {
            if current.is_cancelled() {
                return DeploymentError::Cancelled;
            }
        }

        let _ = deployment_logs::Model::append(db, deployment_id, "stderr", message).await;
        let _ =
            deployments::Model::record_failure(db, deployment_id, error_code, message, exit_code)
                .await;

        if let Ok(deployment) = deployments::Model::find_by_id(db, deployment_id).await {
            if let Some(revision_id) = deployment.revision_id {
                let _ = deployment_revisions::Model::mark_failed(db, revision_id).await;
            }
        }

        let _ = applications::Model::update_status(db, application_id, "failed").await;

        DeploymentError::StepFailed {
            step: step.to_string(),
            message: message.to_string(),
        }
    }
}
