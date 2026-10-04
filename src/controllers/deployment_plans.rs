use crate::{
    models::{
        audit_events::{AuditEventInput, Model as AuditEventModel},
        deployment_revisions,
    },
    services::{
        access_control::{Permission, Principal},
        deployment::{DeploymentError, DeploymentService},
        deployment_plan::{DeploymentPlanService, PrepareDeploymentPlanRequest},
    },
    workers::deployment::{DeploymentWorker, DeploymentWorkerArgs},
};
use axum::http::{HeaderMap, StatusCode};
use loco_rs::prelude::*;

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/deployment-plans")
        .add("/", post(prepare))
        .add("{revision_id}/deploy", post(deploy))
}

#[debug_handler]
pub async fn prepare(
    headers: HeaderMap,
    State(ctx): State<AppContext>,
    Json(params): Json<PrepareDeploymentPlanRequest>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    let organization_id = principal
        .application_organization(&ctx.db, params.application_id, Permission::Deploy)
        .await?;

    let plan = DeploymentPlanService::prepare(
        &ctx.db,
        params.application_id,
        &params.commit_hash,
        params.commit_message,
    )
    .await?;

    audit_plan(
        &ctx,
        &principal,
        organization_id,
        "deployment.plan.prepare",
        plan.application_id,
        Some(serde_json::json!({
            "revision_id": plan.revision_id,
            "plan_fingerprint": plan.plan_fingerprint,
            "commit_hash": plan.source.commit_hash,
        })),
    )
    .await;

    format::json(serde_json::json!({
        "data": plan,
        "message": "Deployment plan prepared"
    }))
}

#[debug_handler]
pub async fn deploy(
    headers: HeaderMap,
    Path(revision_id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let revision = deployment_revisions::Model::find_by_id(&ctx.db, revision_id).await?;
    let principal = Principal::authenticate(&ctx, &headers).await?;
    let organization_id = principal
        .application_organization(&ctx.db, revision.application_id, Permission::Deploy)
        .await?;

    match DeploymentService::trigger_planned_deploy(&ctx.db, revision.application_id, revision_id)
        .await
    {
        Ok(deployment) => {
            audit_plan(
                &ctx,
                &principal,
                organization_id,
                "deployment.plan.deploy",
                revision.application_id,
                Some(serde_json::json!({
                    "revision_id": revision_id,
                    "plan_fingerprint": revision.revision_hash,
                    "deployment_id": deployment.id,
                })),
            )
            .await;

            DeploymentWorker::perform_later(
                &ctx,
                DeploymentWorkerArgs {
                    deployment_id: deployment.id,
                },
            )
            .await?;

            Ok((
                StatusCode::ACCEPTED,
                format::json(serde_json::json!({
                    "data": {
                        "deployment_id": deployment.id,
                        "application_id": deployment.application_id,
                        "revision_id": revision_id,
                        "plan_fingerprint": revision.revision_hash,
                        "status": "queued",
                    },
                    "message": "Planned deployment accepted and queued"
                }))?,
            )
                .into_response())
        }
        Err(DeploymentError::Conflict) => Ok((
            StatusCode::CONFLICT,
            format::json(serde_json::json!({
                "error": {
                    "code": "DEPLOYMENT_CONFLICT",
                    "message": "A deployment is already actively running for this application"
                }
            }))?,
        )
            .into_response()),
        Err(error) => Err(Error::BadRequest(error.to_string())),
    }
}

async fn audit_plan(
    ctx: &AppContext,
    principal: &Principal,
    organization_id: i64,
    action: &str,
    application_id: i64,
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
            resource_type: Some("application".to_string()),
            resource_id: Some(application_id.to_string()),
            outcome: "success".to_string(),
            request_id: Some(principal.request_id.clone()),
            metadata: principal.audit_metadata(metadata),
        },
    )
    .await;
}
