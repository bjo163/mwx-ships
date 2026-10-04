use crate::{
    models::{backup_runs, operational_events, server_health_checks},
    services::{
        access_control::Principal,
        operations::OperationsService,
    },
    workers::{
        backup::{BackupWorker, BackupWorkerArgs},
        operations_monitor::{OperationsMonitorWorker, OperationsMonitorWorkerArgs},
    },
};
use axum::http::{HeaderMap, StatusCode};
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
pub async fn metrics(headers: HeaderMap, State(ctx): State<AppContext>) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    principal.require_platform_owner(&ctx.db).await?;
    let data = OperationsService::metrics(&ctx.db).await?;
    format::json(serde_json::json!({"data":data,"message":"ok"}))
}

#[debug_handler]
pub async fn health(headers: HeaderMap, State(ctx): State<AppContext>) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    principal.require_platform_owner(&ctx.db).await?;
    let data = OperationsService::health(&ctx.db).await?;
    let status = if data.status == "healthy" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    Ok((
        status,
        format::json(serde_json::json!({"data":data,"message":"ok"}))?,
    )
        .into_response())
}

#[debug_handler]
pub async fn events(
    headers: HeaderMap,
    State(ctx): State<AppContext>,
    Query(query): Query<LimitQuery>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    principal.require_platform_owner(&ctx.db).await?;
    let data = operational_events::Model::recent(&ctx.db, query.limit.unwrap_or(50)).await?;
    format::json(serde_json::json!({"data":data,"message":"ok"}))
}

#[debug_handler]
pub async fn target_health(
    headers: HeaderMap,
    State(ctx): State<AppContext>,
    Query(query): Query<LimitQuery>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    principal.require_platform_owner(&ctx.db).await?;
    let data = server_health_checks::Model::recent(&ctx.db, query.limit.unwrap_or(50)).await?;
    format::json(serde_json::json!({"data":data,"message":"ok"}))
}

#[debug_handler]
pub async fn backups(
    headers: HeaderMap,
    State(ctx): State<AppContext>,
    Query(query): Query<LimitQuery>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    principal.require_platform_owner(&ctx.db).await?;
    let data = backup_runs::Model::recent(&ctx.db, query.limit.unwrap_or(20)).await?;
    format::json(serde_json::json!({"data":data,"message":"ok"}))
}

#[debug_handler]
pub async fn queue_backup(headers: HeaderMap, State(ctx): State<AppContext>) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    principal.require_platform_owner(&ctx.db).await?;
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
pub async fn queue_poll(headers: HeaderMap, State(ctx): State<AppContext>) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    principal.require_platform_owner(&ctx.db).await?;
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
