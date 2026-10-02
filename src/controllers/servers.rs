use crate::{
    models::{
        _entities::servers::{ActiveModel, Entity},
        servers::{CreateServerParams, Model as ServerModel, UpdateServerParams},
    },
    services::{crypto::CryptoService, ssh::SshService},
};
use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{ActiveValue::Set, EntityTrait};

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
pub async fn list(State(ctx): State<AppContext>) -> Result<Response> {
    let servers = ServerModel::all(&ctx.db).await?;
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
    State(ctx): State<AppContext>,
    Json(params): Json<CreateServerParams>,
) -> Result<Response> {
    let now = Utc::now();
    let encrypted_key = if let Some(key) = params.private_key {
        Some(CryptoService::encrypt(&key).map_err(|e| Error::BadRequest(e.to_string()))?)
    } else {
        None
    };

    let active = ActiveModel {
        name: Set(params.name),
        host: Set(params.host),
        port: Set(params.port.unwrap_or(22)),
        username: Set(params.username),
        authentication_type: Set(params
            .authentication_type
            .unwrap_or_else(|| "ssh_key".to_string())),
        encrypted_private_key: Set(encrypted_key),
        status: Set("unknown".to_string()),
        created_at: Set(now.into()),
        updated_at: Set(now.into()),
        ..Default::default()
    };

    let model = active.insert(&ctx.db).await?;

    format::json(serde_json::json!({
        "data": {
            "id": model.id,
            "name": model.name,
            "host": model.host,
            "port": model.port,
            "username": model.username,
            "status": model.status,
            "created_at": model.created_at,
        },
        "message": "ok"
    }))
}

#[debug_handler]
pub async fn get_one(Path(id): Path<i64>, State(ctx): State<AppContext>) -> Result<Response> {
    let server = ServerModel::find_by_id(&ctx.db, id).await?;
    format::json(serde_json::json!({
        "data": {
            "id": server.id,
            "name": server.name,
            "host": server.host,
            "port": server.port,
            "username": server.username,
            "authentication_type": server.authentication_type,
            "status": server.status,
            "last_seen_at": server.last_seen_at,
            "created_at": server.created_at,
            "updated_at": server.updated_at,
        },
        "message": "ok"
    }))
}

#[debug_handler]
pub async fn update(
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
    Json(params): Json<UpdateServerParams>,
) -> Result<Response> {
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

    active.updated_at = Set(Utc::now().into());
    let updated = active.update(&ctx.db).await?;

    format::json(serde_json::json!({
        "data": {
            "id": updated.id,
            "name": updated.name,
            "host": updated.host,
            "port": updated.port,
            "username": updated.username,
            "status": updated.status,
        },
        "message": "ok"
    }))
}

#[debug_handler]
pub async fn remove(Path(id): Path<i64>, State(ctx): State<AppContext>) -> Result<Response> {
    let server = ServerModel::find_by_id(&ctx.db, id).await?;
    Entity::delete_by_id(server.id).exec(&ctx.db).await?;

    format::json(serde_json::json!({
        "data": null,
        "message": "ok"
    }))
}

#[debug_handler]
pub async fn test_conn(Path(id): Path<i64>, State(ctx): State<AppContext>) -> Result<Response> {
    let server = ServerModel::find_by_id(&ctx.db, id).await?;
    let is_connected =
        SshService::test_connection(&server.host, server.port, &server.username, None)
            .await
            .unwrap_or(false);

    let new_status = if is_connected { "online" } else { "offline" };
    let _ = ServerModel::update_status(&ctx.db, server.id, new_status).await;

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
pub async fn preflight(Path(id): Path<i64>, State(ctx): State<AppContext>) -> Result<Response> {
    let server = ServerModel::find_by_id(&ctx.db, id).await?;
    let report =
        SshService::run_preflight(server.id, &server.host, server.port, &server.username, None)
            .await
            .map_err(|_e| Error::InternalServerError)?;

    let status = if report.healthy {
        "online"
    } else if report.ssh_connected {
        "error"
    } else {
        "offline"
    };

    let _ = ServerModel::update_status(&ctx.db, server.id, status).await;

    format::json(serde_json::json!({
        "data": report,
        "message": "ok"
    }))
}
