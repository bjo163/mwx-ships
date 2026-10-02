use crate::models::{
    applications, deployment_logs, deployments, domains, environment_variables, servers,
};
use crate::services::{
    crypto::CryptoService,
    docker::{ContainerConfig, DockerService},
    git::GitService,
    proxy::ProxyService,
};
use loco_rs::prelude::*;
use std::{path::PathBuf, time::Duration};
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
}

pub struct DeploymentService;

impl DeploymentService {
    /// Verifies concurrency and enqueues a new deployment
    pub async fn trigger_deploy(
        db: &DatabaseConnection,
        app_id: i64,
        commit_hash: Option<String>,
        commit_message: Option<String>,
    ) -> Result<deployments::Model, DeploymentError> {
        let app = applications::Model::find_by_id(db, app_id)
            .await
            .map_err(|_| DeploymentError::AppNotFound(app_id))?;

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
            app.server_id,
            commit_hash,
            commit_message,
        )
        .await
        .map_err(|e| DeploymentError::StepFailed {
            step: "create_deployment".to_string(),
            message: e.to_string(),
        })?;

        // Update application state to queued
        let _ = applications::Model::update_status(db, app.id, "queued").await;

        let _ = deployment_logs::Model::append(
            db,
            dep.id,
            "system",
            &format!(
                "Deployment #{} queued for application '{}'",
                dep.id, app.name
            ),
        )
        .await;

        Ok(dep)
    }

    /// Full asynchronous deployment pipeline orchestrated by DeploymentWorker
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

        let _server = servers::Model::find_by_id(db, app.server_id)
            .await
            .map_err(|_| DeploymentError::ServerNotFound(app.server_id))?;

        let workspace_dir = PathBuf::from("data")
            .join("workspaces")
            .join(format!("app-{}", app.id));
        let _ = tokio::fs::create_dir_all(&workspace_dir).await;

        // ==========================================
        // 1. CLONING
        // ==========================================
        let _ = deployments::Model::update_status(db, dep.id, "cloning").await;
        let _ = applications::Model::update_status(db, app.id, "cloning").await;
        let _ = deployment_logs::Model::append(
            db,
            dep.id,
            "system",
            &format!(
                "Cloning repository '{}' (branch: '{}')...",
                app.git_repository, app.git_branch
            ),
        )
        .await;

        let git_result = if workspace_dir.join(".git").exists() {
            let _ = GitService::fetch_repository(&workspace_dir).await;
            GitService::checkout_branch(&workspace_dir, &app.git_branch).await
        } else {
            GitService::clone_repository(&app.git_repository, &app.git_branch, &workspace_dir).await
        };

        if let Err(e) = git_result {
            let msg = format!("Git operation failed: {}", e);
            let _ = deployment_logs::Model::append(db, dep.id, "stderr", &msg).await;
            let _ =
                deployments::Model::record_failure(db, dep.id, "GIT_CLONE_FAILED", &msg, Some(1))
                    .await;
            let _ = applications::Model::update_status(db, app.id, "failed").await;
            return Err(DeploymentError::StepFailed {
                step: "cloning".to_string(),
                message: msg,
            });
        }

        // Record actual commit hash & message
        let (commit_sha, commit_msg) = GitService::get_current_commit(&workspace_dir)
            .await
            .unwrap_or_else(|_| ("HEAD".to_string(), "Deployment".to_string()));

        let _ = deployment_logs::Model::append(
            db,
            dep.id,
            "stdout",
            &format!(
                "Checked out commit {} - {}",
                &commit_sha[..7.min(commit_sha.len())],
                commit_msg
            ),
        )
        .await;

        // ==========================================
        // 2. BUILDING
        // ==========================================
        let _ = deployments::Model::update_status(db, dep.id, "building").await;
        let _ = applications::Model::update_status(db, app.id, "building").await;

        let image_tag = format!(
            "moonships/{}:{}",
            app.slug,
            &commit_sha[..7.min(commit_sha.len())]
        );

        let build_result = if app.build_type == "prebuilt_image" {
            let img = app.docker_image.as_deref().unwrap_or("nginx:alpine");
            let _ = deployment_logs::Model::append(
                db,
                dep.id,
                "system",
                &format!("Pulling prebuilt image '{}'...", img),
            )
            .await;
            DockerService::pull_image(img).await
        } else {
            let context = workspace_dir.join(&app.docker_context);
            let dockerfile = context.join(&app.dockerfile_path);
            let _ = deployment_logs::Model::append(
                db,
                dep.id,
                "system",
                &format!(
                    "Building Docker image '{}' with context '{}'...",
                    image_tag,
                    context.display()
                ),
            )
            .await;
            DockerService::build_image(&context, &dockerfile, &image_tag).await
        };

        match build_result {
            Ok(output) => {
                let _ = deployment_logs::Model::append(db, dep.id, "stdout", &output).await;
            }
            Err(e) => {
                let msg = format!("Docker build failed: {}", e);
                let _ = deployment_logs::Model::append(db, dep.id, "stderr", &msg).await;
                let _ = deployments::Model::record_failure(
                    db,
                    dep.id,
                    "DOCKER_BUILD_FAILED",
                    &msg,
                    Some(2),
                )
                .await;
                let _ = applications::Model::update_status(db, app.id, "failed").await;
                return Err(DeploymentError::StepFailed {
                    step: "building".to_string(),
                    message: msg,
                });
            }
        }

        // ==========================================
        // 3. STOPPING OLD CONTAINER
        // ==========================================
        let _ = deployments::Model::update_status(db, dep.id, "stopping_old").await;
        let _ = deployment_logs::Model::append(
            db,
            dep.id,
            "system",
            &format!(
                "Stopping existing container '{}' if running...",
                app.container_name
            ),
        )
        .await;

        let _ = DockerService::stop_container(&app.container_name).await;
        let _ = DockerService::remove_container(&app.container_name).await;

        // ==========================================
        // 4. STARTING NEW CONTAINER
        // ==========================================
        let _ = deployments::Model::update_status(db, dep.id, "starting_new").await;
        let _ = applications::Model::update_status(db, app.id, "starting").await;

        // Decrypt environment variables safely
        let raw_env_vars = environment_variables::Model::by_application(db, app.id)
            .await
            .unwrap_or_default();
        let mut decrypted_envs = Vec::new();
        for ev in raw_env_vars {
            let val = if ev.is_secret {
                CryptoService::decrypt(&ev.encrypted_value).unwrap_or(ev.encrypted_value)
            } else {
                ev.encrypted_value
            };
            decrypted_envs.push((ev.key, val));
        }

        // Fetch domains for Traefik label generation
        let app_domains = domains::Model::by_application(db, app.id)
            .await
            .unwrap_or_default();
        let domain_names: Vec<String> = app_domains.iter().map(|d| d.hostname.clone()).collect();
        let has_https = app_domains.iter().any(|d| d.https_enabled);

        let labels = ProxyService::generate_traefik_labels(
            &app.slug,
            &domain_names,
            app.container_port,
            has_https,
        );

        let container_config = ContainerConfig {
            name: app.container_name.clone(),
            image: if app.build_type == "prebuilt_image" {
                app.docker_image
                    .unwrap_or_else(|| "nginx:alpine".to_string())
            } else {
                image_tag
            },
            container_port: app.container_port,
            published_port: app.published_port,
            env_vars: decrypted_envs,
            labels,
            restart_policy: "unless-stopped".to_string(),
            network: None,
        };

        let run_result = DockerService::run_container(&container_config).await;
        match run_result {
            Ok(cid) => {
                let _ = deployment_logs::Model::append(
                    db,
                    dep.id,
                    "stdout",
                    &format!(
                        "Container started successfully with ID: {}",
                        &cid[..12.min(cid.len())]
                    ),
                )
                .await;
            }
            Err(e) => {
                let msg = format!("Failed to run container: {}", e);
                let _ = deployment_logs::Model::append(db, dep.id, "stderr", &msg).await;
                let _ = deployments::Model::record_failure(
                    db,
                    dep.id,
                    "CONTAINER_RUN_FAILED",
                    &msg,
                    Some(3),
                )
                .await;
                let _ = applications::Model::update_status(db, app.id, "failed").await;
                return Err(DeploymentError::StepFailed {
                    step: "starting_new".to_string(),
                    message: msg,
                });
            }
        }

        // ==========================================
        // 5. HEALTHCHECKING
        // ==========================================
        if let Some(path) = &app.healthcheck_path {
            let _ = deployments::Model::update_status(db, dep.id, "healthchecking").await;
            let port = app
                .healthcheck_port
                .or(app.published_port)
                .unwrap_or(app.container_port);
            let _ = deployment_logs::Model::append(
                db,
                dep.id,
                "system",
                &format!(
                    "Running HTTP healthcheck on port {} with path '{}'...",
                    port, path
                ),
            )
            .await;

            let mut passed = false;
            let retries = 5;
            let target_url = format!("http://localhost:{}{}", port, path);

            for attempt in 1..=retries {
                sleep(Duration::from_secs(2)).await;
                let res = tokio::process::Command::new("curl")
                    .arg("-f")
                    .arg("-s")
                    .arg("-m")
                    .arg("3")
                    .arg(&target_url)
                    .output()
                    .await;

                if let Ok(out) = res {
                    if out.status.success() {
                        passed = true;
                        let _ = deployment_logs::Model::append(
                            db,
                            dep.id,
                            "stdout",
                            &format!("Healthcheck passed on attempt {}/{}", attempt, retries),
                        )
                        .await;
                        break;
                    }
                }
            }

            if !passed {
                let msg = format!(
                    "Healthcheck failed on {} after {} attempts",
                    target_url, retries
                );
                let _ = deployment_logs::Model::append(db, dep.id, "stderr", &msg).await;
                let _ = deployments::Model::record_failure(
                    db,
                    dep.id,
                    "HEALTHCHECK_FAILED",
                    &msg,
                    Some(4),
                )
                .await;
                let _ = applications::Model::update_status(db, app.id, "failed").await;
                return Err(DeploymentError::HealthcheckFailed(msg));
            }
        }

        // ==========================================
        // 6. SUCCESS
        // ==========================================
        let _ = deployments::Model::update_status(db, dep.id, "success").await;
        let _ = applications::Model::update_status(db, app.id, "running").await;
        let _ = deployment_logs::Model::append(
            db,
            dep.id,
            "system",
            &format!(
                "Deployment #{} completed successfully! Application '{}' is running.",
                dep.id, app.name
            ),
        )
        .await;

        Ok(())
    }
}
