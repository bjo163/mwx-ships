use crate::{
    models::{backup_runs, operational_events, server_health_checks},
    services::operations::OperationsService,
    workers::{
        backup::{BackupWorker, BackupWorkerArgs},
        operations_monitor::{OperationsMonitorWorker, OperationsMonitorWorkerArgs},
    },
};
use axum::http::StatusCode;
use loco_rs::prelude::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct LimitQuery {
    pub limit: Option<u64>,
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/operations")
        .add("metrics", get(metrics))
        .add("health", get(health))
        .add("events", get(events))
        .add("target-health", get(target_health))
        .add("backups", get(backups))
        .add("backups", post(queue_backup))
        .add("poll", post(queue_poll))
}

#[debug_handler]
pub async fn metrics(
    _auth: auth::JWT,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let data = OperationsService::metrics(&ctx.db).await?;
    format::json(serde_json::json!({"data":data,"message":"ok"}))
}

#[debug_handler]
pub async fn health(
    _auth: auth::JWT,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let data = OperationsService::health(&ctx.db).await?;
    let status = if data.status == "healthy" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    Ok((status, format::json(serde_json::json!({"data":data,"message":"ok"}))?).into_response())
}

#[debug_handler]
pub async fn events(
    _auth: auth::JWT,
    State(ctx): State<AppContext>,
    Query(query): Query<LimitQuery>,
) -> Result<Response> {
    let data = operational_events::Model::recent(&ctx.db, query.limit.unwrap_or(50)).await?;
    format::json(serde_json::json!({"data":data,"message":"ok"}))
}

#[debug_handler]
pub async fn target_health(
    _auth: auth::JWT,
    State(ctx): State<AppContext>,
    Query(query): Query<LimitQuery>,
) -> Result<Response> {
    let data = server_health_checks::Model::recent(&ctx.db, query.limit.unwrap_or(50)).await?;
    format::json(serde_json::json!({"data":data,"message":"ok"}))
}

#[debug_handler]
pub async fn backups(
    _auth: auth::JWT,
    State(ctx): State<AppContext>,
    Query(query): Query<LimitQuery>,
) -> Result<Response> {
    let data = backup_runs::Model::recent(&ctx.db, query.limit.unwrap_or(20)).await?;
    format::json(serde_json::json!({"data":data,"message":"ok"}))
}

#[debug_handler]
pub async fn queue_backup(
    _auth: auth::JWT,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    BackupWorker::perform_later(
        &ctx,
        BackupWorkerArgs {
            reason: "manual_api".to_string(),
        },
    )
    .await?;

    Ok((
        StatusCode::ACCEPTED,
        format::json(serde_json::json!({
            "data":{"queued":true},
            "message":"Backup queued"
        }))?,
    )
        .into_response())
}

#[debug_handler]
pub async fn queue_poll(
    _auth: auth::JWT,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    OperationsMonitorWorker::perform_later(
        &ctx,
        OperationsMonitorWorkerArgs {
            reason: "manual_api".to_string(),
        },
    )
    .await?;

    Ok((
        StatusCode::ACCEPTED,
        format::json(serde_json::json!({
            "data":{"queued":true},
            "message":"Operations poll queued"
        }))?,
    )
        .into_response())
}
