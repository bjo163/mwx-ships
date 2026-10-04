use crate::{
    models::{
        _entities::servers::{ActiveModel, Entity},
        audit_events::{AuditEventInput, Model as AuditEventModel},
        operational_events, server_health_checks,
        servers::{CreateServerParams, Model as ServerModel, UpdateServerParams},
    },
    services::{
        access_control::{Permission, Principal},
        crypto::CryptoService,
        notification::NotificationService,
        ssh::SshService,
    },
};
use axum::http::HeaderMap;
use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{ActiveValue::Set, EntityTrait};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct OrganizationQuery {
    pub organization_id: Option<i64>,
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/servers")
        .add("/", get(list))
        .add("/", post(create))
        .add("{id}", get(get_one))
        .add("{id}", put(update))
        .add("{id}", delete(remove))
        .add("{id}/test-connection", post(test_conn))
        .add("{id}/preflight", post(preflight))
}

#[debug_handler]
pub async fn list(headers: HeaderMap, State(ctx): State<AppContext>) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    let organization_ids = principal.organization_ids(&ctx.db).await?;
    let servers = ServerModel::all_for_organizations(&ctx.db, &organization_ids).await?;
    // Mask private keys
    let safe_servers: Vec<serde_json::Value> = servers
        .into_iter()
        .map(|s| {
            serde_json::json!({
                "id": s.id,
                "name": s.name,
                "host": s.host,
                "port": s.port,
                "username": s.username,
                "authentication_type": s.authentication_type,
                "known_host_fingerprint": s.known_host_fingerprint,
                "status": s.status,
                "last_seen_at": s.last_seen_at,
                "created_at": s.created_at,
                "updated_at": s.updated_at,
            })
        })
        .collect();

    format::json(serde_json::json!({
        "data": safe_servers,
        "message": "ok"
    }))
}

#[debug_handler]
pub async fn create(
    headers: HeaderMap,
    Query(query): Query<OrganizationQuery>,
    State(ctx): State<AppContext>,
    Json(params): Json<CreateServerParams>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    let organization_id = principal
        .resolve_organization(&ctx.db, query.organization_id)
        .await?;
    principal
        .require(&ctx.db, organization_id, Permission::ManageServers)
        .await?;

    let now = Utc::now();
    let tags = ServerModel::normalized_tags(params.tags.as_deref().unwrap_or(&[]))?;
    let capacity_units = params.capacity_units.unwrap_or(100);
    if capacity_units < 1 {
        return Err(Error::BadRequest(
            "capacity_units must be at least 1".to_string(),
        ));
    }
    let encrypted_key = if let Some(key) = params.private_key {
        Some(CryptoService::encrypt(&key).map_err(|e| Error::BadRequest(e.to_string()))?)
    } else {
        None
    };

    let active = ActiveModel {
        organization_id: Set(Some(organization_id)),
        name: Set(params.name),
        host: Set(params.host),
        port: Set(params.port.unwrap_or(22)),
        username: Set(params.username),
        authentication_type: Set(params
            .authentication_type
            .unwrap_or_else(|| "ssh_key".to_string())),
        encrypted_private_key: Set(encrypted_key),
        known_host_fingerprint: Set(params.known_host_fingerprint),
        status: Set("unknown".to_string()),
        tags_json: Set(serde_json::to_string(&tags)?),
        capacity_units: Set(capacity_units),
        created_at: Set(now.into()),
        updated_at: Set(now.into()),
        ..Default::default()
    };

    let model = active.insert(&ctx.db).await?;
    audit_server(
        &ctx,
        &principal,
        organization_id,
        "server.create",
        model.id,
        None,
    )
    .await;

    format::json(serde_json::json!({
        "data": {
            "id": model.id,
            "name": model.name,
            "host": model.host,
            "port": model.port,
            "username": model.username,
            "known_host_fingerprint": model.known_host_fingerprint,
            "status": model.status,
            "tags": tags,
            "capacity_units": model.capacity_units,
            "created_at": model.created_at,
        },
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
        .server_organization(&ctx.db, id, Permission::View)
        .await?;
    let server = ServerModel::find_by_id(&ctx.db, id).await?;
    format::json(serde_json::json!({
        "data": {
            "id": server.id,
            "name": server.name,
            "host": server.host,
            "port": server.port,
            "username": server.username,
            "authentication_type": server.authentication_type,
            "known_host_fingerprint": server.known_host_fingerprint,
            "status": server.status,
            "tags": server.tags().unwrap_or_default(),
            "capacity_units": server.capacity_units,
            "last_seen_at": server.last_seen_at,
            "created_at": server.created_at,
            "updated_at": server.updated_at,
        },
        "message": "ok"
    }))
}

#[debug_handler]
pub async fn update(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
    Json(params): Json<UpdateServerParams>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    let organization_id = principal
        .server_organization(&ctx.db, id, Permission::ManageServers)
        .await?;
    let server = ServerModel::find_by_id(&ctx.db, id).await?;
    let mut active: ActiveModel = server.into();

    if let Some(name) = params.name {
        active.name = Set(name);
    }
    if let Some(host) = params.host {
        active.host = Set(host);
    }
    if let Some(port) = params.port {
        active.port = Set(port);
    }
    if let Some(username) = params.username {
        active.username = Set(username);
    }
    if let Some(auth_type) = params.authentication_type {
        active.authentication_type = Set(auth_type);
    }
    if let Some(key) = params.private_key {
        let enc = CryptoService::encrypt(&key).map_err(|e| Error::BadRequest(e.to_string()))?;
        active.encrypted_private_key = Set(Some(enc));
    }
    if let Some(fingerprint) = params.known_host_fingerprint {
        active.known_host_fingerprint = Set(Some(fingerprint));
    }
    if let Some(tags) = params.tags {
        active.tags_json = Set(serde_json::to_string(&ServerModel::normalized_tags(
            &tags,
        )?)?);
    }
    if let Some(capacity_units) = params.capacity_units {
        if capacity_units < 1 {
            return Err(Error::BadRequest(
                "capacity_units must be at least 1".to_string(),
            ));
        }
        active.capacity_units = Set(capacity_units);
    }

    active.updated_at = Set(Utc::now().into());
    let updated = active.update(&ctx.db).await?;

    audit_server(&ctx, &principal, organization_id, "server.update", id, None).await;
    format::json(serde_json::json!({
        "data": {
            "id": updated.id,
            "name": updated.name,
            "host": updated.host,
            "port": updated.port,
            "username": updated.username,
            "known_host_fingerprint": updated.known_host_fingerprint,
            "status": updated.status,
            "tags": updated.tags().unwrap_or_default(),
            "capacity_units": updated.capacity_units,
        },
        "message": "ok"
    }))
}

#[debug_handler]
pub async fn remove(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    let organization_id = principal
        .server_organization(&ctx.db, id, Permission::ManageServers)
        .await?;
    let server = ServerModel::find_by_id(&ctx.db, id).await?;
    Entity::delete_by_id(server.id).exec(&ctx.db).await?;

    audit_server(&ctx, &principal, organization_id, "server.delete", id, None).await;
    format::json(serde_json::json!({
        "data": null,
        "message": "ok"
    }))
}

#[debug_handler]
pub async fn test_conn(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    let organization_id = principal
        .server_organization(&ctx.db, id, Permission::ManageServers)
        .await?;
    let server = ServerModel::find_by_id(&ctx.db, id).await?;
    let is_connected = SshService::connect(&server).await.is_ok();

    let new_status = if is_connected { "online" } else { "offline" };
    let _ = ServerModel::update_status(&ctx.db, server.id, new_status).await;

    audit_server(
        &ctx,
        &principal,
        organization_id,
        "server.test_connection",
        id,
        None,
    )
    .await;
    format::json(serde_json::json!({
        "data": {
            "server_id": server.id,
            "connected": is_connected,
            "status": new_status,
        },
        "message": "ok"
    }))
}
#[debug_handler]
pub async fn preflight(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    let organization_id = principal
        .server_organization(&ctx.db, id, Permission::ManageServers)
        .await?;
    let server = ServerModel::find_by_id(&ctx.db, id).await?;
    let report = SshService::run_preflight(&server)
        .await
        .map_err(|_| Error::InternalServerError)?;

    let status = if report.healthy {
        "online"
    } else if report.ssh_connected {
        "error"
    } else {
        "offline"
    };

    let _ = ServerModel::update_status(&ctx.db, server.id, status).await;
    let _ = server_health_checks::Model::record(&ctx.db, &report).await;
    let _ = operational_events::Model::record(
        &ctx.db,
        "target_preflight",
        if report.healthy { "info" } else { "warning" },
        Some("server"),
        Some(server.id),
        &format!("Manual preflight for '{}' returned {}", server.name, status),
        Some(&serde_json::json!({
            "ssh_connected": report.ssh_connected,
            "docker_running": report.docker_running,
            "disk_available_gb": report.disk_available_gb,
            "issues": report.issues,
        })),
    )
    .await;

    if !report.healthy {
        let _ = NotificationService::notify(
            &ctx,
            "target_unhealthy",
            "warning",
            &format!("Target {} unhealthy", server.name),
            &format!(
                "Manual preflight status={status}; {}",
                report.issues.join("; ")
            ),
        )
        .await;
    }

    audit_server(
        &ctx,
        &principal,
        organization_id,
        "server.preflight",
        id,
        None,
    )
    .await;
    format::json(serde_json::json!({
        "data": report,
        "message": "ok"
    }))
}

async fn audit_server(
    ctx: &AppContext,
    principal: &Principal,
    organization_id: i64,
    action: &str,
    server_id: i64,
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
            resource_type: Some("server".to_string()),
            resource_id: Some(server_id.to_string()),
            outcome: "success".to_string(),
            request_id: Some(principal.request_id.clone()),
            metadata: principal.audit_metadata(metadata),
        },
    )
    .await;
}
