use crate::{
    models::{
        deployment_logs::Model as DeploymentLogModel, deployments::Model as DeploymentModel,
    },
    services::deployment::{DeploymentError, DeploymentService},
    workers::deployment::{DeploymentWorker, DeploymentWorkerArgs},
};
use axum::http::StatusCode;
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
pub async fn list(_auth: auth::JWT, State(ctx): State<AppContext>) -> Result<Response> {
    let deps = DeploymentModel::all(&ctx.db).await?;
    format::json(serde_json::json!({
        "data": deps,
        "message": "ok"
    }))
}

#[debug_handler]
pub async fn get_one(
    _auth: auth::JWT,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let dep = DeploymentModel::find_by_id(&ctx.db, id).await?;
    format::json(serde_json::json!({
        "data": dep,
        "message": "ok"
    }))
}

#[debug_handler]
pub async fn get_logs(
    _auth: auth::JWT,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let logs = DeploymentLogModel::by_deployment(&ctx.db, id).await?;
    format::json(serde_json::json!({
        "data": logs,
        "message": "ok"
    }))
}


#[debug_handler]
pub async fn cancel(
    _auth: auth::JWT,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    match DeploymentService::cancel_deployment(&ctx.db, id).await {
        Ok(deployment) => format::json(serde_json::json!({
            "data": deployment,
            "message": "Deployment cancelled"
        })),
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
    _auth: auth::JWT,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    match DeploymentService::retry_deployment(&ctx.db, id).await {
        Ok(deployment) => {
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
