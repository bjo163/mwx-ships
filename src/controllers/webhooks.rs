use crate::{
    models::{
        applications::Model as ApplicationModel,
        git_integrations::Model as GitIntegrationModel,
        webhook_deliveries::{Model as WebhookDeliveryModel, WebhookDeliveryInput},
    },
    services::{
        deployment::{DeploymentError, DeploymentService},
        git_provider::{CommitStatus, GitProviderError, GitProviderService},
        preview::{PreviewError, PreviewService},
    },
    workers::deployment::{DeploymentWorker, DeploymentWorkerArgs},
};
use axum::{
    body::Bytes,
    http::{HeaderMap, StatusCode},
};
use loco_rs::prelude::*;

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/webhooks")
        .add("{provider}/{application_id}", post(receive))
}

#[debug_handler]
pub async fn receive(
    Path((provider, application_id)): Path<(String, i64)>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response> {
    let integration =
        GitIntegrationModel::find_for_application_provider(&ctx.db, application_id, &provider)
            .await
            .map_err(|_| Error::BadRequest("Git integration not configured".to_string()))?;

    if !integration.enabled {
        return response(
            StatusCode::GONE,
            serde_json::json!({"error":{"code":"GIT_INTEGRATION_DISABLED"}}),
        );
    }

    let secret = integration.webhook_secret()?;
    let event =
        match GitProviderService::verify_and_parse(&integration.provider, &headers, &body, &secret)
        {
            Ok(event) => event,
            Err(GitProviderError::InvalidSignature | GitProviderError::MissingHeader(_)) => {
                return response(
                    StatusCode::UNAUTHORIZED,
                    serde_json::json!({"error":{"code":"INVALID_WEBHOOK_SIGNATURE"}}),
                );
            }
            Err(error) => return Err(Error::BadRequest(error.to_string())),
        };

    let created = WebhookDeliveryModel::create_or_get(
        &ctx.db,
        &WebhookDeliveryInput {
            application_id,
            provider: event.provider.clone(),
            delivery_id: event.delivery_id.clone(),
            event_kind: event.event_kind.clone(),
            source_ref: event.source_ref.clone(),
            commit_sha: event.commit_sha.clone(),
            external_request_id: event.external_request_id.clone(),
            action: event.action.clone(),
        },
    )
    .await?;

    if !created.inserted && created.delivery.status != "failed" {
        return response(
            StatusCode::ACCEPTED,
            serde_json::json!({
                "data":{
                    "delivery_id": created.delivery.delivery_id,
                    "duplicate": true,
                    "status": created.delivery.status,
                    "deployment_id": created.delivery.deployment_id,
                },
                "message":"Duplicate webhook delivery ignored"
            }),
        );
    }

    match event.event_kind.as_str() {
        "push" => handle_push(&ctx, &integration, &created.delivery, &event).await,
        "pull_request" => handle_pull_request(&ctx, &integration, &created.delivery, &event).await,
        _ => {
            let delivery = WebhookDeliveryModel::mark_ignored(&ctx.db, created.delivery.id).await?;
            response(
                StatusCode::ACCEPTED,
                serde_json::json!({
                    "data":{"delivery_id":delivery.delivery_id,"status":"ignored"},
                    "message":"Webhook event not used by Moonships"
                }),
            )
        }
    }
}

async fn handle_push(
    ctx: &AppContext,
    integration: &GitIntegrationModel,
    delivery: &WebhookDeliveryModel,
    event: &crate::services::git_provider::GitWebhookEvent,
) -> Result<Response> {
    let app = ApplicationModel::find_by_id(&ctx.db, delivery.application_id).await?;
    let source_ref = event.source_ref.as_deref().unwrap_or_default();

    if !app.auto_deploy || source_ref != app.git_branch {
        let delivery = WebhookDeliveryModel::mark_ignored(&ctx.db, delivery.id).await?;
        return response(
            StatusCode::ACCEPTED,
            serde_json::json!({
                "data":{
                    "delivery_id":delivery.delivery_id,
                    "status":"ignored",
                    "expected_branch":app.git_branch,
                },
                "message":"Push does not match enabled auto-deploy branch"
            }),
        );
    }

    let commit_sha = event
        .commit_sha
        .clone()
        .ok_or_else(|| Error::BadRequest("push webhook has no commit SHA".to_string()))?;

    match DeploymentService::trigger_deploy_with_provenance(
        &ctx.db,
        app.id,
        Some(commit_sha.clone()),
        Some(format!(
            "{} webhook {}",
            integration.provider, delivery.delivery_id
        )),
        "webhook",
        None,
    )
    .await
    {
        Ok(deployment) => {
            let delivery =
                WebhookDeliveryModel::mark_queued(&ctx.db, delivery.id, deployment.id).await?;

            let _ = GitProviderService::set_commit_status(
                integration,
                &commit_sha,
                CommitStatus::Pending,
                "Moonships deployment queued",
                None,
            )
            .await;

            DeploymentWorker::perform_later(
                ctx,
                DeploymentWorkerArgs {
                    deployment_id: deployment.id,
                },
            )
            .await?;

            response(
                StatusCode::ACCEPTED,
                serde_json::json!({
                    "data":{
                        "delivery_id":delivery.delivery_id,
                        "deployment_id":deployment.id,
                        "status":"queued",
                    },
                    "message":"Webhook deployment queued"
                }),
            )
        }
        Err(DeploymentError::Conflict) => {
            let delivery = WebhookDeliveryModel::mark_failed(
                &ctx.db,
                delivery.id,
                "another deployment is already active",
            )
            .await?;
            let _ = GitProviderService::set_commit_status(
                integration,
                &commit_sha,
                CommitStatus::Failure,
                "Moonships deployment blocked by an active deployment",
                None,
            )
            .await;
            response(
                StatusCode::CONFLICT,
                serde_json::json!({
                    "error":{
                        "code":"DEPLOYMENT_CONFLICT",
                        "delivery_id":delivery.delivery_id
                    }
                }),
            )
        }
        Err(error) => {
            let _ =
                WebhookDeliveryModel::mark_failed(&ctx.db, delivery.id, error.to_string()).await;
            Err(Error::BadRequest(error.to_string()))
        }
    }
}

async fn handle_pull_request(
    ctx: &AppContext,
    integration: &GitIntegrationModel,
    delivery: &WebhookDeliveryModel,
    event: &crate::services::git_provider::GitWebhookEvent,
) -> Result<Response> {
    let external_request_id = event
        .external_request_id
        .clone()
        .ok_or_else(|| Error::BadRequest("pull request webhook has no request id".to_string()))?;

    if event.closed {
        return match PreviewService::close(
            &ctx.db,
            delivery.application_id,
            &delivery.provider,
            &external_request_id,
        )
        .await
        {
            Ok(preview) => {
                let delivery = WebhookDeliveryModel::mark_completed(&ctx.db, delivery.id).await?;
                response(
                    StatusCode::ACCEPTED,
                    serde_json::json!({
                        "data":{
                            "delivery_id":delivery.delivery_id,
                            "preview":preview,
                            "status":"closed"
                        },
                        "message":"Preview runtime removed"
                    }),
                )
            }
            Err(error) => {
                let _ = WebhookDeliveryModel::mark_failed(&ctx.db, delivery.id, error.to_string())
                    .await;
                let status = if matches!(&error, PreviewError::Busy) {
                    StatusCode::CONFLICT
                } else {
                    StatusCode::BAD_REQUEST
                };
                response(
                    status,
                    serde_json::json!({
                        "error":{
                            "code":"PREVIEW_CLOSE_FAILED",
                            "message":error.to_string()
                        }
                    }),
                )
            }
        };
    }

    let source_ref = event
        .source_ref
        .clone()
        .ok_or_else(|| Error::BadRequest("pull request webhook has no source ref".to_string()))?;
    let commit_sha = event
        .commit_sha
        .clone()
        .ok_or_else(|| Error::BadRequest("pull request webhook has no commit SHA".to_string()))?;

    match PreviewService::prepare_and_queue(
        &ctx.db,
        delivery.application_id,
        &delivery.provider,
        &external_request_id,
        &source_ref,
        &commit_sha,
    )
    .await
    {
        Ok((preview, preview_app, deployment)) => {
            let delivery =
                WebhookDeliveryModel::mark_queued(&ctx.db, delivery.id, deployment.id).await?;

            let preview_url = preview.preview_hostname.as_deref().map(|host| {
                let https_enabled = std::env::var("MOONSHIPS_PREVIEW_HTTPS")
                    .ok()
                    .map(|value| {
                        matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes")
                    })
                    .unwrap_or(false);
                let scheme = if https_enabled { "https" } else { "http" };
                format!("{scheme}://{host}")
            });

            let _ = GitProviderService::set_commit_status(
                integration,
                &commit_sha,
                CommitStatus::Pending,
                "Moonships preview deployment queued",
                preview_url.as_deref(),
            )
            .await;

            DeploymentWorker::perform_later(
                ctx,
                DeploymentWorkerArgs {
                    deployment_id: deployment.id,
                },
            )
            .await?;

            response(
                StatusCode::ACCEPTED,
                serde_json::json!({
                    "data":{
                        "delivery_id":delivery.delivery_id,
                        "deployment_id":deployment.id,
                        "preview_application_id":preview_app.id,
                        "preview_hostname":preview.preview_hostname,
                        "status":"queued"
                    },
                    "message":"Preview deployment queued"
                }),
            )
        }
        Err(error) => {
            let _ =
                WebhookDeliveryModel::mark_failed(&ctx.db, delivery.id, error.to_string()).await;
            let status = if matches!(
                &error,
                PreviewError::Busy | PreviewError::Deployment(DeploymentError::Conflict)
            ) {
                StatusCode::CONFLICT
            } else {
                StatusCode::BAD_REQUEST
            };
            response(
                status,
                serde_json::json!({
                    "error":{
                        "code":"PREVIEW_DEPLOY_FAILED",
                        "message":error.to_string()
                    }
                }),
            )
        }
    }
}

fn response(status: StatusCode, body: serde_json::Value) -> Result<Response> {
    Ok((status, format::json(body)?).into_response())
}
