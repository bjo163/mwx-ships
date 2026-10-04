use crate::{
    models::{
        _entities::{
            applications::{ActiveModel, Entity},
            domains, environment_variables,
        },
        applications::{
            CreateApplicationParams, Model as ApplicationModel, UpdateApplicationParams,
        },
        deployments::{Model as DeploymentModel, TriggerDeployParams},
        domains::{CreateDomainParams, Model as DomainModel},
        environment_variables::{Model as EnvVarModel, SetEnvVarParams},
        servers::Model as ServerModel,
    },
    services::{
        crypto::CryptoService,
        deployment::{DeploymentError, DeploymentService},
        proxy::ProxyService,
        remote::{redact_secrets, RemoteRuntime},
    },
    workers::deployment::{DeploymentWorker, DeploymentWorkerArgs},
};
use axum::http::StatusCode;
use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/applications")
        .add("/", get(list))
        .add("/", post(create))
        .add("{id}", get(get_one))
        .add("{id}", put(update))
        .add("{id}", delete(remove))
        .add("{id}/deploy", post(deploy))
        .add("{id}/rollback", post(rollback))
        .add("{id}/start", post(start))
        .add("{id}/stop", post(stop))
        .add("{id}/restart", post(restart))
        .add("{id}/status", get(status))
        .add("{id}/logs", get(logs))
        .add("{id}/environment", get(get_env))
        .add("{id}/environment", post(set_env))
        .add("{id}/environment/{key}", delete(remove_env))
        .add("{id}/domains", get(list_domains))
        .add("{id}/domains", post(add_domain))
        .add("{id}/domains/{domain_id}", delete(remove_domain))
}

async fn remote_runtime(
    db: &DatabaseConnection,
    app: &ApplicationModel,
) -> Result<(ServerModel, RemoteRuntime)> {
    let server = ServerModel::find_by_id(db, app.server_id).await?;
    let runtime = RemoteRuntime::connect(&server)
        .await
        .map_err(|err| Error::BadRequest(err.to_string()))?;
    Ok((server, runtime))
}

#[debug_handler]
pub async fn list(_auth: auth::JWT, State(ctx): State<AppContext>) -> Result<Response> {
    let apps = ApplicationModel::all(&ctx.db).await?;
    format::json(serde_json::json!({
        "data": apps,
        "message": "ok"
    }))
}

#[debug_handler]
pub async fn create(
    _auth: auth::JWT,
    State(ctx): State<AppContext>,
    Json(params): Json<CreateApplicationParams>,
) -> Result<Response> {
    let app = ApplicationModel::create_application(&ctx.db, &params).await?;
    format::json(serde_json::json!({
        "data": app,
        "message": "ok"
    }))
}

#[debug_handler]
pub async fn get_one(
    _auth: auth::JWT,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let app = ApplicationModel::find_by_id(&ctx.db, id).await?;
    let latest_deployment = DeploymentModel::latest_for_application(&ctx.db, id).await?;
    let env_count = EnvVarModel::by_application(&ctx.db, id).await?.len();
    let domains = DomainModel::by_application(&ctx.db, id).await?;

    format::json(serde_json::json!({
        "data": {
            "application": app,
            "latest_deployment": latest_deployment,
            "environment_variables_count": env_count,
            "domains": domains,
        },
        "message": "ok"
    }))
}

#[debug_handler]
pub async fn update(
    _auth: auth::JWT,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
    Json(params): Json<UpdateApplicationParams>,
) -> Result<Response> {
    let app = ApplicationModel::find_by_id(&ctx.db, id).await?;
    let mut active: ActiveModel = app.into();

    if let Some(name) = params.name {
        active.name = Set(name);
    }
    if let Some(repo) = params.git_repository {
        active.git_repository = Set(repo);
    }
    if let Some(branch) = params.git_branch {
        active.git_branch = Set(branch);
    }
    if let Some(btype) = params.build_type {
        active.build_type = Set(btype);
    }
    if let Some(df_path) = params.dockerfile_path {
        active.dockerfile_path = Set(df_path);
    }
    if let Some(ctx_path) = params.docker_context {
        active.docker_context = Set(ctx_path);
    }
    if let Some(img) = params.docker_image {
        active.docker_image = Set(Some(img));
    }
    if let Some(cport) = params.container_port {
        active.container_port = Set(cport);
    }
    if let Some(pub_port) = params.published_port {
        active.published_port = Set(Some(pub_port));
    }
    if let Some(cmd) = params.startup_command {
        active.startup_command = Set(Some(cmd));
    }
    if let Some(hpath) = params.healthcheck_path {
        active.healthcheck_path = Set(Some(hpath));
    }
    if let Some(hport) = params.healthcheck_port {
        active.healthcheck_port = Set(Some(hport));
    }
    if let Some(auto) = params.auto_deploy {
        active.auto_deploy = Set(auto);
    }
    active.updated_at = Set(Utc::now().into());

    let updated = active.update(&ctx.db).await?;
    format::json(serde_json::json!({
        "data": updated,
        "message": "ok"
    }))
}

#[debug_handler]
pub async fn remove(
    _auth: auth::JWT,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let app = ApplicationModel::find_by_id(&ctx.db, id).await?;
    let (_, runtime) = remote_runtime(&ctx.db, &app).await?;
    runtime
        .stop_and_remove_container(&app.container_name)
        .await
        .map_err(|err| Error::BadRequest(err.to_string()))?;
    Entity::delete_by_id(app.id).exec(&ctx.db).await?;

    format::json(serde_json::json!({
        "data": null,
        "message": "ok"
    }))
}
#[debug_handler]
pub async fn deploy(
    _auth: auth::JWT,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
    Json(params): Json<TriggerDeployParams>,
) -> Result<Response> {
    match DeploymentService::trigger_deploy(&ctx.db, id, params.commit_hash, params.commit_message)
        .await
    {
        Ok(dep) => {
            // Enqueue asynchronous DeploymentWorker via Loco SQLite background queue
            DeploymentWorker::perform_later(
                &ctx,
                DeploymentWorkerArgs {
                    deployment_id: dep.id,
                },
            )
            .await?;

            // Asynchronous 202 Accepted response
            let res = (
                StatusCode::ACCEPTED,
                format::json(serde_json::json!({
                    "data": {
                        "deployment_id": dep.id,
                        "application_id": dep.application_id,
                        "status": "queued",
                    },
                    "message": "Deployment accepted and queued"
                }))?,
            );
            Ok(res.into_response())
        }
        Err(DeploymentError::Conflict) => {
            let res = (
                StatusCode::CONFLICT,
                format::json(serde_json::json!({
                    "error": {
                        "code": "DEPLOYMENT_CONFLICT",
                        "message": "A deployment is already actively running for this application"
                    }
                }))?,
            );
            Ok(res.into_response())
        }
        Err(e) => Err(Error::BadRequest(e.to_string())),
    }
}

#[debug_handler]
pub async fn rollback(
    _auth: auth::JWT,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    match DeploymentService::rollback_application(&ctx.db, id).await {
        Ok(dep) => {
            DeploymentWorker::perform_later(
                &ctx,
                DeploymentWorkerArgs {
                    deployment_id: dep.id,
                },
            )
            .await?;

            let res = (
                StatusCode::ACCEPTED,
                format::json(serde_json::json!({
                    "data": {
                        "deployment_id": dep.id,
                        "application_id": dep.application_id,
                        "revision_id": dep.revision_id,
                        "status": dep.status,
                        "trigger_kind": dep.trigger_kind,
                    },
                    "message": "Rollback accepted and queued"
                }))?,
            );
            Ok(res.into_response())
        }
        Err(DeploymentError::Conflict) => {
            let res = (
                StatusCode::CONFLICT,
                format::json(serde_json::json!({
                    "error": {
                        "code": "DEPLOYMENT_CONFLICT",
                        "message": "A deployment is already actively running for this application"
                    }
                }))?,
            );
            Ok(res.into_response())
        }
        Err(DeploymentError::RollbackNotAvailable { .. }) => {
            let res = (
                StatusCode::CONFLICT,
                format::json(serde_json::json!({
                    "error": {
                        "code": "ROLLBACK_NOT_AVAILABLE",
                        "message": "No previous known-good revision is available"
                    }
                }))?,
            );
            Ok(res.into_response())
        }
        Err(error) => Err(Error::BadRequest(error.to_string())),
    }
}

#[debug_handler]
pub async fn start(
    _auth: auth::JWT,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let app = ApplicationModel::find_by_id(&ctx.db, id).await?;
    let (_, runtime) = remote_runtime(&ctx.db, &app).await?;
    runtime
        .start_container(&app.container_name)
        .await
        .map_err(|err| Error::BadRequest(err.to_string()))?;
    let updated = ApplicationModel::update_status(&ctx.db, app.id, "running").await?;
    format::json(serde_json::json!({
        "data": updated,
        "message": "ok"
    }))
}
#[debug_handler]
pub async fn stop(
    _auth: auth::JWT,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let app = ApplicationModel::find_by_id(&ctx.db, id).await?;
    let (_, runtime) = remote_runtime(&ctx.db, &app).await?;
    runtime
        .stop_container(&app.container_name)
        .await
        .map_err(|err| Error::BadRequest(err.to_string()))?;
    let updated = ApplicationModel::update_status(&ctx.db, app.id, "stopped").await?;
    format::json(serde_json::json!({
        "data": updated,
        "message": "ok"
    }))
}
#[debug_handler]
pub async fn restart(
    _auth: auth::JWT,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let app = ApplicationModel::find_by_id(&ctx.db, id).await?;
    let (_, runtime) = remote_runtime(&ctx.db, &app).await?;
    runtime
        .restart_container(&app.container_name)
        .await
        .map_err(|err| Error::BadRequest(err.to_string()))?;
    let updated = ApplicationModel::update_status(&ctx.db, app.id, "running").await?;
    format::json(serde_json::json!({
        "data": updated,
        "message": "ok"
    }))
}
#[debug_handler]
pub async fn status(
    _auth: auth::JWT,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let app = ApplicationModel::find_by_id(&ctx.db, id).await?;
    let (server, runtime) = remote_runtime(&ctx.db, &app).await?;
    let container_status = runtime
        .container_status(&app.container_name)
        .await
        .unwrap_or_else(|_| "unknown".to_string());
    format::json(serde_json::json!({
        "data": {
            "application_id": app.id,
            "status": app.status,
            "container_status": container_status,
            "target_server_id": server.id,
            "target_server": server.name,
        },
        "message": "ok"
    }))
}
#[debug_handler]
pub async fn logs(
    _auth: auth::JWT,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let app = ApplicationModel::find_by_id(&ctx.db, id).await?;
    let (server, runtime) = remote_runtime(&ctx.db, &app).await?;
    let logs = runtime
        .container_logs(&app.container_name, 200)
        .await
        .unwrap_or_default();

    let vars = EnvVarModel::by_application(&ctx.db, app.id)
        .await
        .unwrap_or_default();
    let secrets: Vec<String> = vars
        .into_iter()
        .filter(|var| var.is_secret)
        .filter_map(|var| CryptoService::decrypt(&var.encrypted_value).ok())
        .collect();
    let safe_logs = redact_secrets(&logs, &secrets);

    format::json(serde_json::json!({
        "data": {
            "application_id": app.id,
            "container_name": app.container_name,
            "target_server_id": server.id,
            "target_server": server.name,
            "logs": safe_logs,
        },
        "message": "ok"
    }))
}
#[debug_handler]
pub async fn get_env(
    _auth: auth::JWT,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let vars = EnvVarModel::by_application(&ctx.db, id).await?;
    let safe: Vec<_> = vars.into_iter().map(|v| v.to_safe()).collect();
    format::json(serde_json::json!({
        "data": safe,
        "message": "ok"
    }))
}

#[debug_handler]
pub async fn set_env(
    _auth: auth::JWT,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
    Json(params): Json<SetEnvVarParams>,
) -> Result<Response> {
    let is_secret = params.is_secret.unwrap_or(false);
    let encrypted = if is_secret {
        CryptoService::encrypt(&params.value).map_err(|e| Error::BadRequest(e.to_string()))?
    } else {
        params.value
    };

    let existing = environment_variables::Entity::find()
        .filter(environment_variables::Column::ApplicationId.eq(id))
        .filter(environment_variables::Column::Key.eq(&params.key))
        .one(&ctx.db)
        .await?;

    let now = Utc::now();
    let model = match existing {
        Some(ev) => {
            let mut active: environment_variables::ActiveModel = ev.into();
            active.encrypted_value = Set(encrypted);
            active.is_secret = Set(is_secret);
            active.updated_at = Set(now.into());
            active.update(&ctx.db).await?
        }
        None => {
            let active = environment_variables::ActiveModel {
                application_id: Set(id),
                key: Set(params.key),
                encrypted_value: Set(encrypted),
                is_secret: Set(is_secret),
                created_at: Set(now.into()),
                updated_at: Set(now.into()),
                ..Default::default()
            };
            active.insert(&ctx.db).await?
        }
    };

    format::json(serde_json::json!({
        "data": model.to_safe(),
        "message": "ok"
    }))
}

#[debug_handler]
pub async fn remove_env(
    _auth: auth::JWT,
    Path((id, key)): Path<(i64, String)>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let existing = environment_variables::Entity::find()
        .filter(environment_variables::Column::ApplicationId.eq(id))
        .filter(environment_variables::Column::Key.eq(key))
        .one(&ctx.db)
        .await?;

    if let Some(ev) = existing {
        environment_variables::Entity::delete_by_id(ev.id)
            .exec(&ctx.db)
            .await?;
    }

    format::json(serde_json::json!({
        "data": null,
        "message": "ok"
    }))
}

#[debug_handler]
pub async fn list_domains(
    _auth: auth::JWT,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let domains = DomainModel::by_application(&ctx.db, id).await?;
    format::json(serde_json::json!({
        "data": domains,
        "message": "ok"
    }))
}

#[debug_handler]
pub async fn add_domain(
    _auth: auth::JWT,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
    Json(params): Json<CreateDomainParams>,
) -> Result<Response> {
    ProxyService::validate_hostname(&params.hostname)
        .map_err(|e| Error::BadRequest(e.to_string()))?;
    let domain = DomainModel::create_domain(&ctx.db, id, &params).await?;
    format::json(serde_json::json!({
        "data": domain,
        "message": "ok"
    }))
}

#[debug_handler]
pub async fn remove_domain(
    _auth: auth::JWT,
    Path((_id, domain_id)): Path<(i64, i64)>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    domains::Entity::delete_by_id(domain_id)
        .exec(&ctx.db)
        .await?;
    format::json(serde_json::json!({
        "data": null,
        "message": "ok"
    }))
}
