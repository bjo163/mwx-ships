use crate::{
    models::{
        applications::Model as ApplicationModel,
        audit_events::{AuditEventInput, Model as AuditEventModel},
        deployment_logs::Model as DeploymentLogModel,
        deployments::Model as DeploymentModel,
        projects::Model as ProjectModel,
    },
    services::{
        access_control::{Permission, Principal},
        deployment::{DeploymentError, DeploymentService},
    },
    workers::deployment::{DeploymentWorker, DeploymentWorkerArgs},
};
use axum::http::{HeaderMap, StatusCode};
use loco_rs::prelude::*;

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/deployments")
        .add("/", get(list))
        .add("{id}", get(get_one))
        .add("{id}/logs", get(get_logs))
        .add("{id}/cancel", post(cancel))
        .add("{id}/retry", post(retry))
}

#[debug_handler]
pub async fn list(headers: HeaderMap, State(ctx): State<AppContext>) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    let organization_ids = principal.organization_ids(&ctx.db).await?;
    let projects = ProjectModel::all_for_organizations(&ctx.db, &organization_ids).await?;
    let project_ids = projects.into_iter().map(|project| project.id).collect::<Vec<_>>();
    let applications = ApplicationModel::all_for_projects(&ctx.db, &project_ids).await?;
    let application_ids = applications.into_iter().map(|app| app.id).collect::<Vec<_>>();
    let deployments = DeploymentModel::all_for_applications(&ctx.db, &application_ids).await?;

    format::json(serde_json::json!({
        "data": deployments,
        "message": "ok"
    }))
}

#[debug_handler]
pub async fn get_one(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    principal
        .deployment_organization(&ctx.db, id, Permission::View)
        .await?;
    let deployment = DeploymentModel::find_by_id(&ctx.db, id).await?;
    format::json(serde_json::json!({
        "data": deployment,
        "message": "ok"
    }))
}

#[debug_handler]
pub async fn get_logs(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    principal
        .deployment_organization(&ctx.db, id, Permission::View)
        .await?;
    let logs = DeploymentLogModel::by_deployment(&ctx.db, id).await?;
    format::json(serde_json::json!({
        "data": logs,
        "message": "ok"
    }))
}

#[debug_handler]
pub async fn cancel(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    let organization_id = principal
        .deployment_organization(&ctx.db, id, Permission::Deploy)
        .await?;

    match DeploymentService::cancel_deployment(&ctx.db, id).await {
        Ok(deployment) => {
            audit_deployment(
                &ctx,
                &principal,
                organization_id,
                "deployment.cancel",
                deployment.id,
                None,
            )
            .await;
            format::json(serde_json::json!({
                "data": deployment,
                "message": "Deployment cancelled"
            }))
        }
        Err(DeploymentError::CancelNotAllowed { status }) => {
            let response = (
                StatusCode::CONFLICT,
                format::json(serde_json::json!({
                    "error": {
                        "code": "DEPLOYMENT_CANCEL_NOT_ALLOWED",
                        "message": format!("Deployment cannot be cancelled from status '{status}'")
                    }
                }))?,
            );
            Ok(response.into_response())
        }
        Err(error) => Err(Error::BadRequest(error.to_string())),
    }
}

#[debug_handler]
pub async fn retry(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    let organization_id = principal
        .deployment_organization(&ctx.db, id, Permission::Deploy)
        .await?;

    match DeploymentService::retry_deployment(&ctx.db, id).await {
        Ok(deployment) => {
            audit_deployment(
                &ctx,
                &principal,
                organization_id,
                "deployment.retry",
                deployment.id,
                Some(serde_json::json!({"source_deployment_id": id})),
            )
            .await;

            DeploymentWorker::perform_later(
                &ctx,
                DeploymentWorkerArgs {
                    deployment_id: deployment.id,
                },
            )
            .await?;

            let response = (
                StatusCode::ACCEPTED,
                format::json(serde_json::json!({
                    "data": {
                        "deployment_id": deployment.id,
                        "application_id": deployment.application_id,
                        "status": deployment.status,
                    },
                    "message": format!("Retry queued from deployment #{id}")
                }))?,
            );
            Ok(response.into_response())
        }
        Err(DeploymentError::Conflict) => {
            let response = (
                StatusCode::CONFLICT,
                format::json(serde_json::json!({
                    "error": {
                        "code": "DEPLOYMENT_CONFLICT",
                        "message": "Another deployment is already active for this application"
                    }
                }))?,
            );
            Ok(response.into_response())
        }
        Err(DeploymentError::RetryNotAllowed { status }) => {
            let response = (
                StatusCode::CONFLICT,
                format::json(serde_json::json!({
                    "error": {
                        "code": "DEPLOYMENT_RETRY_NOT_ALLOWED",
                        "message": format!("Deployment cannot be retried from status '{status}'")
                    }
                }))?,
            );
            Ok(response.into_response())
        }
        Err(error) => Err(Error::BadRequest(error.to_string())),
    }
}

async fn audit_deployment(
    ctx: &AppContext,
    principal: &Principal,
    organization_id: i64,
    action: &str,
    deployment_id: i64,
    metadata: Option<serde_json::Value>,
) {
    let (actor_kind, actor_id) = principal.audit_actor();
    let _ = AuditEventModel::append(
        &ctx.db,
        AuditEventInput {
            organization_id: Some(organization_id),
            actor_kind: actor_kind.to_string(),
            actor_id,
            action: action.to_string(),
            resource_type: Some("deployment".to_string()),
            resource_id: Some(deployment_id.to_string()),
            outcome: "success".to_string(),
            request_id: None,
            metadata,
        },
    )
    .await;
}
