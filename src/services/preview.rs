use crate::{
    models::{
        _entities::{applications, domains, environment_variables, environments, git_integrations},
        applications::{CreateApplicationParams, Model as ApplicationModel},
        deployments::Model as DeploymentModel,
        domains::{CreateDomainParams, Model as DomainModel},
        environment_variables::Model as EnvironmentVariableModel,
        environments::{CreateEnvironmentParams, Model as EnvironmentModel},
        git_integrations::Model as GitIntegrationModel,
        preview_deployments::{Model as PreviewModel, UpsertPreviewInput},
        servers::Model as ServerModel,
    },
    services::{
        deployment::{DeploymentError, DeploymentService},
        proxy::ProxyService,
        remote::RemoteRuntime,
    },
};
use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};

#[derive(Debug, thiserror::Error)]
pub enum PreviewError {
    #[error("preview base domain is not configured")]
    BaseDomainMissing,
    #[error("preview application has an active deployment")]
    Busy,
    #[error("preview setup failed: {0}")]
    Setup(String),
    #[error("preview cleanup failed: {0}")]
    Cleanup(String),
    #[error(transparent)]
    Deployment(#[from] DeploymentError),
}

pub struct PreviewService;

impl PreviewService {
    pub async fn prepare_and_queue(
        db: &DatabaseConnection,
        base_application_id: i64,
        provider: &str,
        external_request_id: &str,
        source_ref: &str,
        commit_sha: &str,
    ) -> Result<(PreviewModel, ApplicationModel, DeploymentModel), PreviewError> {
        let base_app = ApplicationModel::find_by_id(db, base_application_id)
            .await
            .map_err(|err| PreviewError::Setup(err.to_string()))?;

        let mut preview = PreviewModel::upsert(
            db,
            &UpsertPreviewInput {
                application_id: base_application_id,
                provider: provider.to_string(),
                external_request_id: external_request_id.to_string(),
                source_ref: source_ref.to_string(),
                commit_sha: commit_sha.to_string(),
            },
        )
        .await
        .map_err(|err| PreviewError::Setup(err.to_string()))?;

        let preview_base_domain = std::env::var("MOONSHIPS_PREVIEW_BASE_DOMAIN")
            .ok()
            .map(|value| value.trim().trim_matches('.').to_ascii_lowercase())
            .filter(|value| !value.is_empty())
            .ok_or(PreviewError::BaseDomainMissing)?;

        ProxyService::validate_hostname(&preview_base_domain)
            .map_err(|err| PreviewError::Setup(err.to_string()))?;

        let preview_hostname = format!("{}.{}", preview.preview_slug, preview_base_domain);
        ProxyService::validate_hostname(&preview_hostname)
            .map_err(|err| PreviewError::Setup(err.to_string()))?;

        let preview_environment =
            if let Some(preview_environment_id) = preview.preview_environment_id {
                EnvironmentModel::find_by_id(db, preview_environment_id)
                    .await
                    .map_err(|err| PreviewError::Setup(err.to_string()))?
            } else {
                EnvironmentModel::create_environment(
                    db,
                    &CreateEnvironmentParams {
                        project_id: base_app.project_id,
                        name: format!("Preview #{}", external_request_id),
                        slug: Some(preview.preview_slug.clone()),
                        description: Some(format!(
                            "{} {} preview environment",
                            provider, external_request_id
                        )),
                    },
                )
                .await
                .map_err(|err| PreviewError::Setup(err.to_string()))?
            };

        let preview_app = if let Some(preview_application_id) = preview.preview_application_id {
            if DeploymentModel::has_active_deployment(db, preview_application_id)
                .await
                .map_err(|err| PreviewError::Setup(err.to_string()))?
            {
                return Err(PreviewError::Busy);
            }
            let existing = ApplicationModel::find_by_id(db, preview_application_id)
                .await
                .map_err(|err| PreviewError::Setup(err.to_string()))?;
            Self::sync_preview_application(
                db,
                existing,
                &base_app,
                preview_environment.id,
                source_ref,
            )
            .await?
        } else {
            ApplicationModel::create_application(
                db,
                &CreateApplicationParams {
                    project_id: base_app.project_id,
                    environment_id: preview_environment.id,
                    server_id: base_app.server_id,
                    name: format!("{} Preview #{}", base_app.name, external_request_id),
                    slug: Some(preview.preview_slug.clone()),
                    git_repository: base_app.git_repository.clone(),
                    git_branch: Some(source_ref.to_string()),
                    build_type: Some(base_app.build_type.clone()),
                    dockerfile_path: Some(base_app.dockerfile_path.clone()),
                    docker_context: Some(base_app.docker_context.clone()),
                    docker_image: base_app.docker_image.clone(),
                    container_name: Some(format!("moonships-{}", preview.preview_slug)),
                    container_port: Some(base_app.container_port),
                    published_port: None,
                    startup_command: base_app.startup_command.clone(),
                    healthcheck_path: base_app.healthcheck_path.clone(),
                    healthcheck_port: base_app.healthcheck_port,
                    auto_deploy: Some(false),
                },
            )
            .await
            .map_err(|err| PreviewError::Setup(err.to_string()))?
        };

        Self::sync_git_integration(db, base_app.id, preview_app.id, provider).await?;
        Self::sync_environment(db, base_app.id, preview_app.id).await?;
        Self::sync_domain(db, preview_app.id, &preview_hostname).await?;

        let deployment = DeploymentService::trigger_deploy_with_provenance(
            db,
            preview_app.id,
            Some(commit_sha.to_string()),
            Some(format!(
                "{} preview #{} from {}",
                provider, external_request_id, source_ref
            )),
            "preview",
            None,
        )
        .await?;

        preview = PreviewModel::attach_runtime(
            db,
            preview.id,
            preview_app.id,
            preview_environment.id,
            deployment.id,
            Some(preview_hostname),
        )
        .await
        .map_err(|err| PreviewError::Setup(err.to_string()))?;

        Ok((preview, preview_app, deployment))
    }

    pub async fn close(
        db: &DatabaseConnection,
        base_application_id: i64,
        provider: &str,
        external_request_id: &str,
    ) -> Result<Option<PreviewModel>, PreviewError> {
        let Some(preview) =
            PreviewModel::find_by_request(db, base_application_id, provider, external_request_id)
                .await
                .map_err(|err| PreviewError::Cleanup(err.to_string()))?
        else {
            return Ok(None);
        };

        if preview.status == "closed" {
            return Ok(Some(preview));
        }

        let preview = PreviewModel::mark_closing(db, preview.id)
            .await
            .map_err(|err| PreviewError::Cleanup(err.to_string()))?;

        if let Some(preview_application_id) = preview.preview_application_id {
            if DeploymentModel::has_active_deployment(db, preview_application_id)
                .await
                .map_err(|err| PreviewError::Cleanup(err.to_string()))?
            {
                if let Some(active) =
                    DeploymentModel::latest_for_application(db, preview_application_id)
                        .await
                        .map_err(|err| PreviewError::Cleanup(err.to_string()))?
                {
                    match DeploymentService::cancel_deployment(db, active.id).await {
                        Ok(_) | Err(DeploymentError::CancelNotAllowed { .. }) => {}
                        Err(DeploymentError::ExecutionClaimUnavailable { .. })
                        | Err(DeploymentError::ExecutionClaimLost) => {}
                        Err(error) => {
                            return Err(PreviewError::Cleanup(error.to_string()));
                        }
                    }
                }

                return Ok(Some(preview));
            }
        }

        Self::cleanup_preview(db, preview).await.map(Some)
    }

    pub async fn finalize_pending_close_for_deployment(
        db: &DatabaseConnection,
        deployment_id: i64,
    ) -> Result<(), PreviewError> {
        let Some(preview) = PreviewModel::find_by_deployment(db, deployment_id)
            .await
            .map_err(|err| PreviewError::Cleanup(err.to_string()))?
        else {
            return Ok(());
        };

        if preview.status != "closing" {
            return Ok(());
        }

        Self::cleanup_preview(db, preview).await?;
        Ok(())
    }

    async fn cleanup_preview(
        db: &DatabaseConnection,
        preview: PreviewModel,
    ) -> Result<PreviewModel, PreviewError> {
        if let Some(preview_application_id) = preview.preview_application_id {
            let app = ApplicationModel::find_by_id(db, preview_application_id)
                .await
                .map_err(|err| PreviewError::Cleanup(err.to_string()))?;

            if DeploymentModel::has_active_deployment(db, app.id)
                .await
                .map_err(|err| PreviewError::Cleanup(err.to_string()))?
            {
                return Err(PreviewError::Busy);
            }

            let server = ServerModel::find_by_id(db, app.server_id)
                .await
                .map_err(|err| PreviewError::Cleanup(err.to_string()))?;
            let runtime = RemoteRuntime::connect(&server)
                .await
                .map_err(|err| PreviewError::Cleanup(err.to_string()))?;

            let active_runtime = app.resolved_runtime_name();
            let _ = runtime.stop_and_remove_container(&active_runtime).await;

            if let Some(candidate) = app.candidate_runtime_name.as_deref() {
                if candidate != active_runtime {
                    let _ = runtime.stop_and_remove_container(candidate).await;
                }
            }

            runtime
                .remove_managed_route(app.id)
                .await
                .map_err(|err| PreviewError::Cleanup(err.to_string()))?;

            applications::Entity::delete_by_id(app.id)
                .exec(db)
                .await
                .map_err(|err| PreviewError::Cleanup(err.to_string()))?;
        }

        if let Some(preview_environment_id) = preview.preview_environment_id {
            environments::Entity::delete_by_id(preview_environment_id)
                .exec(db)
                .await
                .map_err(|err| PreviewError::Cleanup(err.to_string()))?;
        }

        PreviewModel::close(
            db,
            preview.application_id,
            &preview.provider,
            &preview.external_request_id,
        )
        .await
        .map_err(|err| PreviewError::Cleanup(err.to_string()))?
        .ok_or_else(|| PreviewError::Cleanup("preview record disappeared during cleanup".to_string()))
    }

    async fn sync_preview_application(
        db: &DatabaseConnection,
        preview: ApplicationModel,
        base: &ApplicationModel,
        preview_environment_id: i64,
        source_ref: &str,
    ) -> Result<ApplicationModel, PreviewError> {
        let mut active: applications::ActiveModel = preview.into();
        active.server_id = Set(base.server_id);
        active.environment_id = Set(preview_environment_id);
        active.git_repository = Set(base.git_repository.clone());
        active.git_branch = Set(source_ref.to_string());
        active.build_type = Set(base.build_type.clone());
        active.dockerfile_path = Set(base.dockerfile_path.clone());
        active.docker_context = Set(base.docker_context.clone());
        active.docker_image = Set(base.docker_image.clone());
        active.container_port = Set(base.container_port);
        active.published_port = Set(None);
        active.startup_command = Set(base.startup_command.clone());
        active.healthcheck_path = Set(base.healthcheck_path.clone());
        active.healthcheck_port = Set(base.healthcheck_port);
        active.auto_deploy = Set(false);
        active.updated_at = Set(Utc::now().into());
        active
            .update(db)
            .await
            .map_err(|err| PreviewError::Setup(err.to_string()))
    }

    async fn sync_git_integration(
        db: &DatabaseConnection,
        source_application_id: i64,
        preview_application_id: i64,
        provider: &str,
    ) -> Result<(), PreviewError> {
        let source =
            GitIntegrationModel::find_for_application_provider(db, source_application_id, provider)
                .await
                .map_err(|err| PreviewError::Setup(err.to_string()))?;

        let existing = git_integrations::Entity::find()
            .filter(git_integrations::Column::ApplicationId.eq(preview_application_id))
            .filter(git_integrations::Column::Provider.eq(&source.provider))
            .one(db)
            .await
            .map_err(|err| PreviewError::Setup(err.to_string()))?;

        let now = Utc::now();
        match existing {
            Some(model) => {
                let mut active: git_integrations::ActiveModel = model.into();
                active.repository_ref = Set(source.repository_ref);
                active.api_base_url = Set(source.api_base_url);
                active.git_username = Set(source.git_username);
                active.encrypted_token = Set(source.encrypted_token);
                active.encrypted_webhook_secret = Set(source.encrypted_webhook_secret);
                active.enabled = Set(false);
                active.updated_at = Set(now.into());
                active
                    .update(db)
                    .await
                    .map_err(|err| PreviewError::Setup(err.to_string()))?;
            }
            None => {
                git_integrations::ActiveModel {
                    application_id: Set(preview_application_id),
                    provider: Set(source.provider),
                    repository_ref: Set(source.repository_ref),
                    api_base_url: Set(source.api_base_url),
                    git_username: Set(source.git_username),
                    encrypted_token: Set(source.encrypted_token),
                    encrypted_webhook_secret: Set(source.encrypted_webhook_secret),
                    enabled: Set(false),
                    created_at: Set(now.into()),
                    updated_at: Set(now.into()),
                    ..Default::default()
                }
                .insert(db)
                .await
                .map_err(|err| PreviewError::Setup(err.to_string()))?;
            }
        }

        Ok(())
    }

    async fn sync_environment(
        db: &DatabaseConnection,
        source_application_id: i64,
        preview_application_id: i64,
    ) -> Result<(), PreviewError> {
        let vars = EnvironmentVariableModel::by_application(db, source_application_id)
            .await
            .map_err(|err| PreviewError::Setup(err.to_string()))?;

        environment_variables::Entity::delete_many()
            .filter(environment_variables::Column::ApplicationId.eq(preview_application_id))
            .exec(db)
            .await
            .map_err(|err| PreviewError::Setup(err.to_string()))?;

        let now = Utc::now();
        for var in vars {
            environment_variables::ActiveModel {
                application_id: Set(preview_application_id),
                key: Set(var.key),
                encrypted_value: Set(var.encrypted_value),
                is_secret: Set(var.is_secret),
                created_at: Set(now.into()),
                updated_at: Set(now.into()),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(|err| PreviewError::Setup(err.to_string()))?;
        }

        Ok(())
    }

    async fn sync_domain(
        db: &DatabaseConnection,
        preview_application_id: i64,
        hostname: &str,
    ) -> Result<(), PreviewError> {
        domains::Entity::delete_many()
            .filter(domains::Column::ApplicationId.eq(preview_application_id))
            .exec(db)
            .await
            .map_err(|err| PreviewError::Setup(err.to_string()))?;

        let https_enabled = std::env::var("MOONSHIPS_PREVIEW_HTTPS")
            .ok()
            .map(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
            .unwrap_or(false);

        DomainModel::create_domain(
            db,
            preview_application_id,
            &CreateDomainParams {
                hostname: hostname.to_string(),
                port: None,
                https_enabled: Some(https_enabled),
            },
        )
        .await
        .map_err(|err| PreviewError::Setup(err.to_string()))?;

        Ok(())
    }
}
