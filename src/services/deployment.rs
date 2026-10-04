use crate::models::{
    applications, deployment_logs, deployment_revisions, deployments, domains, git_integrations,
    servers, webhook_deliveries,
};
use crate::services::{
    crypto::CryptoService,
    docker::ContainerConfig,
    git_provider::{CommitStatus, GitProviderService},
    proxy::ProxyService,
    remote::{redact_secrets, RemoteRuntime},
    retention::RetentionService,
};
use loco_rs::prelude::*;
use std::time::Duration;
use tokio::time::{sleep, timeout};

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
    #[error("No previous known-good revision is available for application {application_id}")]
    RollbackNotAvailable { application_id: i64 },
    #[error("Deployment execution is already claimed in status '{status}'")]
    ExecutionClaimUnavailable { status: String },
    #[error("Deployment execution lease ownership was lost")]
    ExecutionClaimLost,
}

pub struct DeploymentService;

impl DeploymentService {
    pub async fn trigger_deploy(
        db: &DatabaseConnection,
        app_id: i64,
        commit_hash: Option<String>,
        commit_message: Option<String>,
    ) -> Result<deployments::Model, DeploymentError> {
        Self::trigger_deploy_with_provenance(
            db,
            app_id,
            commit_hash,
            commit_message,
            "manual",
            None,
        )
        .await
    }

    pub async fn trigger_deploy_with_provenance(
        db: &DatabaseConnection,
        app_id: i64,
        commit_hash: Option<String>,
        commit_message: Option<String>,
        trigger_kind: &str,
        source_deployment_id: Option<i64>,
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

        let dep = deployments::Model::create_deployment_attempt(
            db,
            app.id,
            server.id,
            commit_hash,
            commit_message,
            trigger_kind,
            source_deployment_id,
            None,
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
                "Deployment #{} queued for application '{}' on target server '{}' ({}@{}:{}) via {}",
                dep.id,
                app.name,
                server.name,
                server.username,
                server.host,
                server.port,
                trigger_kind
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
            })? {
            Some(cancelled) => {
                let _ = deployment_logs::Model::append(
                    db,
                    deployment_id,
                    "system",
                    "Deployment cancelled before destructive replacement began",
                )
                .await;

                if let Ok(app) =
                    applications::Model::find_by_id(db, deployment.application_id).await
                {
                    let restored_status = if app.current_revision_id.is_some() {
                        "running"
                    } else {
                        "stopped"
                    };
                    let _ = applications::Model::update_status(db, app.id, restored_status).await;
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

    pub async fn rollback_application(
        db: &DatabaseConnection,
        application_id: i64,
    ) -> Result<deployments::Model, DeploymentError> {
        let app = applications::Model::find_by_id(db, application_id)
            .await
            .map_err(|_| DeploymentError::AppNotFound(application_id))?;

        if deployments::Model::has_active_deployment(db, application_id)
            .await
            .map_err(|err| DeploymentError::StepFailed {
                step: "rollback_lock_check".to_string(),
                message: err.to_string(),
            })?
        {
            return Err(DeploymentError::Conflict);
        }

        let revision_id = app
            .previous_revision_id
            .ok_or(DeploymentError::RollbackNotAvailable { application_id })?;
        let revision = deployment_revisions::Model::find_by_id(db, revision_id)
            .await
            .map_err(|err| DeploymentError::StepFailed {
                step: "load_rollback_revision".to_string(),
                message: err.to_string(),
            })?;

        if revision.application_id != application_id {
            return Err(DeploymentError::StepFailed {
                step: "validate_rollback_revision".to_string(),
                message: "previous revision belongs to a different application".to_string(),
            });
        }

        let source_deployment_id =
            deployments::Model::latest_success_for_application(db, application_id)
                .await
                .map_err(|err| DeploymentError::StepFailed {
                    step: "rollback_source_lookup".to_string(),
                    message: err.to_string(),
                })?
                .map(|deployment| deployment.id);

        let deployment = deployments::Model::create_deployment_attempt(
            db,
            application_id,
            revision.server_id,
            Some(revision.source_commit_hash.clone()),
            revision.source_commit_message.clone(),
            "rollback",
            source_deployment_id,
            Some(revision.id),
        )
        .await
        .map_err(|err| DeploymentError::StepFailed {
            step: "create_rollback".to_string(),
            message: err.to_string(),
        })?;

        let _ = applications::Model::update_status(db, application_id, "queued").await;
        let _ = deployment_logs::Model::append(
            db,
            deployment.id,
            "system",
            &format!(
                "Rollback queued to immutable revision {} (commit {})",
                &revision.revision_hash[..12.min(revision.revision_hash.len())],
                &revision.source_commit_hash[..7.min(revision.source_commit_hash.len())]
            ),
        )
        .await;

        Ok(deployment)
    }

    pub async fn execute_deployment(
        db: &DatabaseConnection,
        deployment_id: i64,
    ) -> Result<(), DeploymentError> {
        let lease_seconds = Self::execution_lease_seconds();
        let claim = deployments::Model::claim_for_execution(db, deployment_id, lease_seconds)
            .await
            .map_err(|err| DeploymentError::StepFailed {
                step: "claim_execution".to_string(),
                message: err.to_string(),
            })?;

        let claim = match claim {
            Some(claim) => claim,
            None => {
                let current = deployments::Model::find_by_id(db, deployment_id)
                    .await
                    .map_err(|err| DeploymentError::StepFailed {
                        step: "reload_claim".to_string(),
                        message: err.to_string(),
                    })?;

                return match current.status.as_str() {
                    "success" | "failed" => Ok(()),
                    "cancelled" => Err(DeploymentError::Cancelled),
                    _ => Err(DeploymentError::ExecutionClaimUnavailable {
                        status: current.status,
                    }),
                };
            }
        };

        let execution_token = claim.token.clone();
        let dep = claim.deployment;

        if let Some(recovered_from) = claim.recovered_from.as_deref() {
            let _ = deployment_logs::Model::append(
                db,
                dep.id,
                "system",
                &format!(
                    "Recovered stale deployment execution from phase '{recovered_from}' (attempt {})",
                    dep.attempt_count
                ),
            )
            .await;
        }

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
                    &execution_token,
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
                &execution_token,
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

        Self::transition_phase(db, dep.id, &execution_token, &["connecting"], "cloning").await?;
        let _ = applications::Model::update_status(db, app.id, "cloning").await;
        let _ = deployment_logs::Model::append(
            db,
            dep.id,
            "system",
            &format!("Synchronizing branch '{}' on target server", app.git_branch),
        )
        .await;

        let requested_snapshot = match requested_revision.as_ref() {
            Some(revision) => {
                Some(
                    revision
                        .snapshot()
                        .map_err(|err| DeploymentError::StepFailed {
                            step: "decode_requested_revision".to_string(),
                            message: err.to_string(),
                        })?,
                )
            }
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

        let git_integration =
            git_integrations::Model::find_for_repository(db, app.id, git_repository)
                .await
                .map_err(|err| DeploymentError::StepFailed {
                    step: "git_integration_lookup".to_string(),
                    message: err.to_string(),
                })?;

        let (git_username, git_token) = match git_integration.as_ref() {
            Some(integration)
                if git_repository.starts_with("https://")
                    || git_repository.starts_with("http://") =>
            {
                let username = integration.resolved_git_username().map_err(|err| {
                    DeploymentError::StepFailed {
                        step: "git_credential_username".to_string(),
                        message: err.to_string(),
                    }
                })?;
                let token = integration
                    .token()
                    .map_err(|err| DeploymentError::StepFailed {
                        step: "git_credential_decrypt".to_string(),
                        message: err.to_string(),
                    })?;
                (username, token)
            }
            _ => (None, None),
        };

        let (commit_sha, commit_message) = match runtime
            .sync_repository_config_with_credentials(
                app.id,
                git_repository,
                git_branch,
                requested_commit,
                git_username.as_deref(),
                git_token.as_deref(),
            )
            .await
        {
            Ok(revision) => revision,
            Err(err) => {
                let exit_code = err.exit_code();
                let message = redact_secrets(
                    &err.to_string(),
                    &git_token.iter().cloned().collect::<Vec<_>>(),
                );
                return Err(Self::record_failure(
                    db,
                    &execution_token,
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
                    &execution_token,
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
                        &execution_token,
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
                &execution_token,
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

        Self::transition_phase(db, dep.id, &execution_token, &["cloning"], "building").await?;
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
                db,
                &execution_token,
                dep.id,
                app.id,
                error_code,
                "building",
                &message,
                exit_code,
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

        let mut decrypted_envs = Vec::new();
        let mut secret_values = git_token.iter().cloned().collect::<Vec<_>>();

        for env in &revision_snapshot.environment {
            let value = if env.is_secret {
                let encrypted =
                    env.encrypted_value
                        .as_deref()
                        .ok_or_else(|| DeploymentError::StepFailed {
                            step: "revision_secret_resolve".to_string(),
                            message: format!(
                                "revision secret '{}' has no encrypted value",
                                env.key
                            ),
                        })?;

                if deployment_revisions::sha256_hex(encrypted) != env.value_fingerprint {
                    return Err(Self::record_failure(
                        db,
                        &execution_token,
                        dep.id,
                        app.id,
                        "REVISION_SECRET_INTEGRITY_FAILED",
                        "starting_runtime",
                        &format!(
                            "encrypted revision secret '{}' failed integrity check",
                            env.key
                        ),
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
                            &execution_token,
                            dep.id,
                            app.id,
                            "SECRET_DECRYPT_FAILED",
                            "starting_runtime",
                            &err.to_string(),
                            None,
                        )
                        .await);
                    }
                }
            } else {
                let value = env
                    .value
                    .clone()
                    .ok_or_else(|| DeploymentError::StepFailed {
                        step: "revision_env_resolve".to_string(),
                        message: format!("revision environment '{}' has no value", env.key),
                    })?;
                if deployment_revisions::sha256_hex(&value) != env.value_fingerprint {
                    return Err(Self::record_failure(
                        db,
                        &execution_token,
                        dep.id,
                        app.id,
                        "REVISION_ENV_INTEGRITY_FAILED",
                        "starting_runtime",
                        &format!("revision environment '{}' failed integrity check", env.key),
                        None,
                    )
                    .await);
                }
                value
            };
            decrypted_envs.push((env.key.clone(), value));
        }

        let managed_ingress = ProxyService::uses_managed_ingress(
            revision_snapshot.domains.len(),
            revision_snapshot.published_port,
        );

        if managed_ingress {
            let healthcheck_path = match revision_snapshot.healthcheck_path.as_deref() {
                Some(path) if !path.trim().is_empty() => path,
                _ => {
                    return Err(Self::record_failure(
                        db,
                        &execution_token,
                        dep.id,
                        app.id,
                        "MANAGED_INGRESS_HEALTHCHECK_REQUIRED",
                        "building",
                        "managed ingress requires an application healthcheck path before traffic can switch",
                        None,
                    )
                    .await);
                }
            };

            let https_domains = revision_snapshot
                .domains
                .iter()
                .filter(|domain| domain.https_enabled)
                .map(|domain| domain.hostname.clone())
                .collect::<Vec<_>>();
            let http_domains = revision_snapshot
                .domains
                .iter()
                .filter(|domain| !domain.https_enabled)
                .map(|domain| domain.hostname.clone())
                .collect::<Vec<_>>();

            let acme_email = std::env::var("MOONSHIPS_ACME_EMAIL")
                .ok()
                .filter(|value| !value.trim().is_empty());
            if !https_domains.is_empty() && acme_email.is_none() {
                return Err(Self::record_failure(
                    db,
                    &execution_token,
                    dep.id,
                    app.id,
                    "MANAGED_TLS_EMAIL_REQUIRED",
                    "building",
                    "HTTPS domains require MOONSHIPS_ACME_EMAIL for Let's Encrypt",
                    None,
                )
                .await);
            }

            for domain_snapshot in &revision_snapshot.domains {
                let verified = match runtime
                    .verify_domain_target(&domain_snapshot.hostname, &server.host)
                    .await
                {
                    Ok(verified) => verified,
                    Err(err) => {
                        return Err(Self::record_failure(
                            db,
                            &execution_token,
                            dep.id,
                            app.id,
                            "DOMAIN_VERIFICATION_FAILED",
                            "building",
                            &err.to_string(),
                            err.exit_code(),
                        )
                        .await);
                    }
                };

                if let Ok(Some(current_domain)) = domains::Model::find_by_application_hostname(
                    db,
                    app.id,
                    &domain_snapshot.hostname,
                )
                .await
                {
                    let _ = domains::Model::update_verification(
                        db,
                        current_domain.id,
                        verified,
                        (!verified).then(|| {
                            format!(
                                "DNS for '{}' does not resolve to target '{}'",
                                domain_snapshot.hostname, server.host
                            )
                        }),
                    )
                    .await;
                }

                if !verified {
                    return Err(Self::record_failure(
                        db,
                        &execution_token,
                        dep.id,
                        app.id,
                        "DOMAIN_VERIFICATION_FAILED",
                        "building",
                        &format!(
                            "DNS for '{}' does not resolve to target '{}'",
                            domain_snapshot.hostname, server.host
                        ),
                        None,
                    )
                    .await);
                }
            }

            let traefik_image = std::env::var("MOONSHIPS_TRAEFIK_IMAGE")
                .unwrap_or_else(|_| "traefik:v3.1".to_string());
            if let Err(err) = runtime
                .ensure_managed_ingress(
                    ProxyService::MANAGED_NETWORK,
                    ProxyService::MANAGED_PROXY_CONTAINER,
                    &traefik_image,
                    acme_email.as_deref(),
                )
                .await
            {
                return Err(Self::record_failure(
                    db,
                    &execution_token,
                    dep.id,
                    app.id,
                    "MANAGED_INGRESS_UNAVAILABLE",
                    "building",
                    &err.to_string(),
                    err.exit_code(),
                )
                .await);
            }

            let candidate_name =
                ProxyService::managed_runtime_name(&app.slug, &revision.revision_hash);
            let old_runtime_name = app.resolved_runtime_name();
            let _ = applications::Model::set_candidate_runtime(
                db,
                app.id,
                Some(candidate_name.clone()),
            )
            .await;

            Self::transition_phase(
                db,
                dep.id,
                &execution_token,
                &["building"],
                "starting_candidate",
            )
            .await?;
            let _ = applications::Model::update_status(db, app.id, "starting_candidate").await;

            let container_config = ContainerConfig {
                name: candidate_name.clone(),
                image: revision.image_reference.clone(),
                container_port: revision_snapshot.container_port,
                published_port: None,
                env_vars: decrypted_envs.clone(),
                labels: Vec::new(),
                restart_policy: "unless-stopped".to_string(),
                network: Some(ProxyService::MANAGED_NETWORK.to_string()),
            };

            let existing_candidate_status = runtime
                .container_status(&candidate_name)
                .await
                .unwrap_or_else(|_| "stopped".to_string());

            if existing_candidate_status == "running" {
                let _ = deployment_logs::Model::append(
                    db,
                    dep.id,
                    "system",
                    &format!(
                        "Reusing already-running candidate runtime '{}' after execution recovery",
                        candidate_name
                    ),
                )
                .await;
            } else {
                let _ = runtime.stop_and_remove_container(&candidate_name).await;
                match runtime.run_container(app.id, &container_config).await {
                    Ok(container_id) => {
                        let _ = deployment_logs::Model::append(
                            db,
                            dep.id,
                            "stdout",
                            &format!(
                                "Candidate runtime '{}' started with container ID {}",
                                candidate_name,
                                &container_id[..12.min(container_id.len())]
                            ),
                        )
                        .await;
                    }
                    Err(err) => {
                        let _ = applications::Model::set_candidate_runtime(db, app.id, None).await;
                        let message = redact_secrets(&err.to_string(), &secret_values);
                        return Err(Self::record_failure(
                            db,
                            &execution_token,
                            dep.id,
                            app.id,
                            "REMOTE_DOCKER_CANDIDATE_FAILED",
                            "starting_candidate",
                            &message,
                            err.exit_code(),
                        )
                        .await);
                    }
                }
            }

            Self::transition_phase(
                db,
                dep.id,
                &execution_token,
                &["starting_candidate"],
                "healthchecking",
            )
            .await?;
            let _ = applications::Model::update_status(db, app.id, "healthchecking").await;

            let retries = 5;
            let mut last_error = None;
            for attempt in 1..=retries {
                sleep(Duration::from_secs(2)).await;
                match runtime
                    .healthcheck_once(
                        &candidate_name,
                        None,
                        revision_snapshot.container_port,
                        healthcheck_path,
                    )
                    .await
                {
                    Ok(()) => {
                        let _ = deployment_logs::Model::append(
                            db,
                            dep.id,
                            "stdout",
                            &format!("Candidate healthcheck passed on attempt {attempt}/{retries}"),
                        )
                        .await;
                        last_error = None;
                        break;
                    }
                    Err(err) => last_error = Some(err),
                }
            }

            if let Some(err) = last_error {
                let message = redact_secrets(&err.to_string(), &secret_values);
                let _ = runtime.stop_and_remove_container(&candidate_name).await;
                let _ = applications::Model::set_candidate_runtime(db, app.id, None).await;
                if revision.status != "healthy"
                    && revision.image_reference.starts_with("moonships/")
                {
                    let _ = runtime.remove_image(&revision.image_reference).await;
                }
                let _ = Self::record_failure(
                    db,
                    &execution_token,
                    dep.id,
                    app.id,
                    "REMOTE_HEALTHCHECK_FAILED",
                    "healthchecking",
                    &message,
                    err.exit_code(),
                )
                .await;
                return Err(DeploymentError::HealthcheckFailed(message));
            }

            Self::transition_phase(
                db,
                dep.id,
                &execution_token,
                &["healthchecking"],
                "switching_traffic",
            )
            .await?;
            let route = ProxyService::managed_route_config(
                &app.slug,
                &candidate_name,
                revision_snapshot.container_port,
                &https_domains,
                &http_domains,
            );
            let route_json = serde_json::to_string_pretty(&route).map_err(|err| {
                DeploymentError::StepFailed {
                    step: "managed_route_serialize".to_string(),
                    message: err.to_string(),
                }
            })?;

            if let Err(err) = runtime.write_managed_route(app.id, &route_json).await {
                let _ = runtime.stop_and_remove_container(&candidate_name).await;
                let _ = applications::Model::set_candidate_runtime(db, app.id, None).await;
                return Err(Self::record_failure(
                    db,
                    &execution_token,
                    dep.id,
                    app.id,
                    "MANAGED_ROUTE_SWITCH_FAILED",
                    "switching_traffic",
                    &err.to_string(),
                    err.exit_code(),
                )
                .await);
            }

            sleep(Duration::from_secs(2)).await;

            if let Err(err) = deployment_revisions::Model::mark_healthy(db, revision.id).await {
                return Err(Self::record_failure(
                    db,
                    &execution_token,
                    dep.id,
                    app.id,
                    "REVISION_FINALIZE_FAILED",
                    "switching_traffic",
                    &err.to_string(),
                    None,
                )
                .await);
            }

            if let Err(err) = applications::Model::promote_revision_with_runtime(
                db,
                app.id,
                revision.id,
                Some(candidate_name.clone()),
            )
            .await
            {
                return Err(Self::record_failure(
                    db,
                    &execution_token,
                    dep.id,
                    app.id,
                    "REVISION_PROMOTE_FAILED",
                    "switching_traffic",
                    &err.to_string(),
                    None,
                )
                .await);
            }

            Self::transition_phase(
                db,
                dep.id,
                &execution_token,
                &["switching_traffic"],
                "draining_old",
            )
            .await?;

            if old_runtime_name != candidate_name {
                if let Err(err) = runtime.stop_and_remove_container(&old_runtime_name).await {
                    let _ = deployment_logs::Model::append(
                        db,
                        dep.id,
                        "stderr",
                        &format!(
                            "Traffic switched successfully but old runtime '{}' cleanup failed: {}",
                            old_runtime_name, err
                        ),
                    )
                    .await;
                }
            }

            for hostname in &https_domains {
                if let Ok(Some(current_domain)) =
                    domains::Model::find_by_application_hostname(db, app.id, hostname).await
                {
                    let tls_ready = runtime.verify_tls(hostname).await.unwrap_or(false);
                    let _ = domains::Model::update_tls_status(
                        db,
                        current_domain.id,
                        if tls_ready { "active" } else { "pending" },
                        None,
                    )
                    .await;
                }
            }

            Self::transition_phase(db, dep.id, &execution_token, &["draining_old"], "success")
                .await?;

            let _ = deployment_logs::Model::append(
                db,
                dep.id,
                "system",
                &format!(
                    "Deployment #{} switched managed traffic to revision {} on runtime '{}'",
                    dep.id,
                    &revision.revision_hash[..12.min(revision.revision_hash.len())],
                    candidate_name
                ),
            )
            .await;
        } else {
            Self::transition_phase(db, dep.id, &execution_token, &["building"], "stopping_old")
                .await?;

            let old_runtime_name = app.resolved_runtime_name();
            if let Err(err) = runtime.stop_and_remove_container(&old_runtime_name).await {
                return Err(Self::record_failure(
                    db,
                    &execution_token,
                    dep.id,
                    app.id,
                    "REMOTE_DOCKER_REPLACE_FAILED",
                    "stopping_old",
                    &err.to_string(),
                    err.exit_code(),
                )
                .await);
            }

            Self::transition_phase(
                db,
                dep.id,
                &execution_token,
                &["stopping_old"],
                "starting_new",
            )
            .await?;
            let _ = applications::Model::update_status(db, app.id, "starting").await;

            let domain_names = revision_snapshot
                .domains
                .iter()
                .map(|domain| domain.hostname.clone())
                .collect::<Vec<_>>();
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

            if let Err(err) = runtime.run_container(app.id, &container_config).await {
                let message = redact_secrets(&err.to_string(), &secret_values);
                return Err(Self::record_failure(
                    db,
                    &execution_token,
                    dep.id,
                    app.id,
                    "REMOTE_DOCKER_RUN_FAILED",
                    "starting_new",
                    &message,
                    err.exit_code(),
                )
                .await);
            }

            if let Some(path) = &revision_snapshot.healthcheck_path {
                Self::transition_phase(
                    db,
                    dep.id,
                    &execution_token,
                    &["starting_new"],
                    "healthchecking",
                )
                .await?;
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
                        Err(err) => last_error = Some(err),
                    }
                }

                if let Some(err) = last_error {
                    let message = redact_secrets(&err.to_string(), &secret_values);
                    let _ = runtime
                        .stop_and_remove_container(&revision_snapshot.container_name)
                        .await;
                    return Err(Self::record_failure(
                        db,
                        &execution_token,
                        dep.id,
                        app.id,
                        "REMOTE_HEALTHCHECK_FAILED",
                        "healthchecking",
                        &message,
                        err.exit_code(),
                    )
                    .await);
                }
            }

            if let Err(err) = deployment_revisions::Model::mark_healthy(db, revision.id).await {
                return Err(Self::record_failure(
                    db,
                    &execution_token,
                    dep.id,
                    app.id,
                    "REVISION_FINALIZE_FAILED",
                    "healthchecking",
                    &err.to_string(),
                    None,
                )
                .await);
            }

            if let Err(err) = applications::Model::promote_revision_with_runtime(
                db,
                app.id,
                revision.id,
                Some(revision_snapshot.container_name.clone()),
            )
            .await
            {
                return Err(Self::record_failure(
                    db,
                    &execution_token,
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
            Self::transition_phase(db, dep.id, &execution_token, success_from, "success").await?;
        }

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

        Self::sync_webhook_delivery(db, dep.id, true, None).await;

        match RetentionService::cleanup_after_success(db, app.id).await {
            Ok(report) => {
                let _ = deployment_logs::Model::append(
                    db,
                    dep.id,
                    "system",
                    &format!(
                        "Retention cleanup: {} revision artifacts considered, {} images pruned, {} old log rows pruned{}",
                        report.revision_artifacts_considered,
                        report.images_pruned,
                        report.logs_pruned,
                        report
                            .disk_available_gb
                            .map(|gb| format!(", target disk {gb:.1} GB free"))
                            .unwrap_or_default()
                    ),
                )
                .await;
                for warning in report.warnings {
                    let _ = deployment_logs::Model::append(db, dep.id, "stderr", &warning).await;
                }
            }
            Err(err) => {
                let _ = deployment_logs::Model::append(
                    db,
                    dep.id,
                    "stderr",
                    &format!("Retention cleanup skipped: {err}"),
                )
                .await;
            }
        }

        Ok(())
    }

    fn execution_lease_seconds() -> i64 {
        std::env::var("MOONSHIPS_DEPLOYMENT_LEASE_SECS")
            .ok()
            .and_then(|value| value.parse::<i64>().ok())
            .filter(|value| *value >= 60)
            .unwrap_or(7200)
    }

    async fn transition_phase(
        db: &DatabaseConnection,
        deployment_id: i64,
        execution_token: &str,
        allowed_from: &[&str],
        next_status: &str,
    ) -> Result<deployments::Model, DeploymentError> {
        let transition = deployments::Model::transition_status_if_owned(
            db,
            deployment_id,
            allowed_from,
            next_status,
            Some(execution_token),
        )
        .await
        .map_err(|err| DeploymentError::StepFailed {
            step: "transition_status".to_string(),
            message: err.to_string(),
        })?;

        match transition {
            Some(deployment) => {
                if !matches!(next_status, "success" | "failed" | "cancelled") {
                    let renewed = deployments::Model::renew_execution_lease(
                        db,
                        deployment_id,
                        execution_token,
                        Self::execution_lease_seconds(),
                    )
                    .await
                    .map_err(|err| DeploymentError::StepFailed {
                        step: "renew_execution_lease".to_string(),
                        message: err.to_string(),
                    })?;
                    if !renewed {
                        return Err(DeploymentError::ExecutionClaimLost);
                    }
                }
                Ok(deployment)
            }
            None => {
                let current = deployments::Model::find_by_id(db, deployment_id)
                    .await
                    .map_err(|err| DeploymentError::StepFailed {
                        step: "reload_transition".to_string(),
                        message: err.to_string(),
                    })?;
                if current.is_cancelled() {
                    Err(DeploymentError::Cancelled)
                } else if current.execution_token.as_deref() != Some(execution_token) {
                    Err(DeploymentError::ExecutionClaimLost)
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

    #[allow(clippy::too_many_arguments)]
    async fn sync_webhook_delivery(
        db: &DatabaseConnection,
        deployment_id: i64,
        success: bool,
        error_message: Option<&str>,
    ) {
        let Ok(Some(delivery)) = webhook_deliveries::Model::by_deployment(db, deployment_id).await
        else {
            return;
        };

        let integration = git_integrations::Model::find_for_application_provider(
            db,
            delivery.application_id,
            &delivery.provider,
        )
        .await;

        if success {
            let _ = webhook_deliveries::Model::mark_completed(db, delivery.id).await;
        } else {
            let _ = webhook_deliveries::Model::mark_failed(
                db,
                delivery.id,
                error_message.unwrap_or("deployment failed"),
            )
            .await;
        }

        let (Ok(integration), Some(commit_sha)) = (integration, delivery.commit_sha.as_deref())
        else {
            return;
        };

        if integration.encrypted_token.is_none() {
            return;
        }

        let status = if success {
            CommitStatus::Success
        } else {
            CommitStatus::Failure
        };
        let description = if success {
            "Moonships deployment succeeded"
        } else {
            "Moonships deployment failed"
        };

        let callback = GitProviderService::set_commit_status(
            &integration,
            commit_sha,
            status,
            description,
            None,
        );
        let _ = timeout(Duration::from_secs(5), callback).await;
    }

    #[allow(clippy::too_many_arguments)]
    async fn record_failure(
        db: &DatabaseConnection,
        execution_token: &str,
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
            if current.execution_token.as_deref() != Some(execution_token) {
                return DeploymentError::ExecutionClaimLost;
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

        Self::sync_webhook_delivery(db, deployment_id, false, Some(message)).await;

        if let Ok(app) = applications::Model::find_by_id(db, application_id).await {
            let fallback_status = if app.current_revision_id.is_some() {
                "running"
            } else {
                "failed"
            };
            let _ = applications::Model::update_status(db, application_id, fallback_status).await;
            if fallback_status == "running" {
                let _ = applications::Model::set_candidate_runtime(db, application_id, None).await;
            }
        }

        DeploymentError::StepFailed {
            step: step.to_string(),
            message: message.to_string(),
        }
    }
}
