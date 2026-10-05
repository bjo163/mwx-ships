use crate::{
    models::{
        _entities::{environment_variables, managed_service_bindings},
        applications::Model as ApplicationModel,
        audit_events::{AuditEventInput, Model as AuditEventModel},
        managed_service_backups::{
            CreateManagedServiceBackupParams, Model as ManagedServiceBackupModel,
            UpdateManagedServiceBackupProtectionParams,
        },
        managed_service_bindings::{
            CreateManagedServiceBindingParams, Model as ManagedServiceBindingModel,
        },
        managed_services::{
            CreateManagedServiceRecord, ManagedServiceCredentials, Model as ManagedServiceModel,
        },
        persistent_volumes::Model as PersistentVolumeModel,
        servers::Model as ServerModel,
    },
    services::{
        access_control::{Permission, Principal},
        crypto::CryptoService,
        managed_service::{CreateManagedServiceParams, ManagedServiceTemplateService},
        managed_service_backup::ManagedServiceBackupService,
        proxy::ProxyService,
        remote::{redact_secrets, RemoteRuntime},
    },
};
use axum::http::HeaderMap;
use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use tokio::time::{sleep, Duration};

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/services")
        .add("/templates", get(templates))
        .add("/", get(list))
        .add("/", post(create))
        .add("{id}", get(get_one))
        .add("{id}/start", post(start))
        .add("{id}/stop", post(stop))
        .add("{id}/restart", post(restart))
        .add("{id}/status", get(status))
        .add("{id}/logs", get(logs))
        .add("{id}/bindings", get(list_bindings))
        .add("{id}/bindings", post(bind))
        .add("{id}/backups", get(list_backups))
        .add("{id}/backups", post(create_backup))
        .add("{id}/backups/{backup_id}/restore", post(restore_backup))
        .add(
            "{id}/backups/{backup_id}/protection",
            put(set_backup_protection),
        )
        .add("{id}/backups/{backup_id}", delete(remove_backup))
}

async fn authorized_service(
    ctx: &AppContext,
    headers: &HeaderMap,
    service_id: i64,
    permission: Permission,
) -> Result<(Principal, ManagedServiceModel)> {
    let principal = Principal::authenticate(ctx, headers).await?;
    let service = ManagedServiceModel::find_by_id(&ctx.db, service_id).await?;
    principal
        .require(&ctx.db, service.organization_id, permission)
        .await?;
    Ok((principal, service))
}

async fn audit_service(
    ctx: &AppContext,
    principal: &Principal,
    service: &ManagedServiceModel,
    action: &str,
    metadata: Option<serde_json::Value>,
) {
    let (actor_kind, actor_id) = principal.audit_actor();
    let _ = AuditEventModel::append(
        &ctx.db,
        AuditEventInput {
            organization_id: Some(service.organization_id),
            actor_kind: actor_kind.to_string(),
            actor_id,
            action: action.to_string(),
            resource_type: Some("managed_service".to_string()),
            resource_id: Some(service.id.to_string()),
            outcome: "success".to_string(),
            request_id: Some(principal.request_id.clone()),
            metadata: principal.audit_metadata(metadata),
        },
    )
    .await;
}

fn decrypt_credentials(service: &ManagedServiceModel) -> Result<ManagedServiceCredentials> {
    let plaintext = CryptoService::decrypt(&service.encrypted_credentials)
        .map_err(|error| Error::BadRequest(error.to_string()))?;
    serde_json::from_str(&plaintext)
        .map_err(|error| Error::BadRequest(format!("invalid managed service credentials: {error}")))
}

async fn runtime_for(
    db: &DatabaseConnection,
    service: &ManagedServiceModel,
) -> Result<(ServerModel, RemoteRuntime)> {
    let server = ServerModel::find_by_id(db, service.server_id).await?;
    let runtime = RemoteRuntime::connect(&server)
        .await
        .map_err(|error| Error::BadRequest(error.to_string()))?;
    Ok((server, runtime))
}

async fn wait_ready(runtime: &RemoteRuntime, service: &ManagedServiceModel) -> Result<()> {
    wait_ready_named(runtime, &service.container_name, &service.kind).await
}

async fn wait_ready_named(
    runtime: &RemoteRuntime,
    container_name: &str,
    kind: &str,
) -> Result<()> {
    for _ in 0..30 {
        if runtime
            .managed_service_ready(container_name, kind)
            .await
            .unwrap_or(false)
        {
            return Ok(());
        }
        sleep(Duration::from_secs(2)).await;
    }
    Err(Error::BadRequest(
        "managed service did not become ready within the bounded startup window".to_string(),
    ))
}

#[debug_handler]
pub async fn templates(headers: HeaderMap, State(ctx): State<AppContext>) -> Result<Response> {
    let _ = Principal::authenticate(&ctx, &headers).await?;
    format::json(serde_json::json!({
        "data": ManagedServiceTemplateService::list(),
        "message": "ok"
    }))
}

#[debug_handler]
pub async fn list(headers: HeaderMap, State(ctx): State<AppContext>) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    let organization_ids = principal.organization_ids(&ctx.db).await?;
    let services = ManagedServiceModel::all_for_organizations(&ctx.db, &organization_ids).await?;
    let safe = services
        .into_iter()
        .map(|service| service.to_safe())
        .collect::<Vec<_>>();
    format::json(serde_json::json!({ "data": safe, "message": "ok" }))
}

#[debug_handler]
pub async fn create(
    headers: HeaderMap,
    State(ctx): State<AppContext>,
    Json(params): Json<CreateManagedServiceParams>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    let organization_id = principal
        .server_organization(&ctx.db, params.server_id, Permission::ManageApplications)
        .await?;
    let prepared = ManagedServiceTemplateService::prepare(&params)?;

    let volume = PersistentVolumeModel::create_declared(
        &ctx.db,
        organization_id,
        params.server_id,
        &format!("{} Data", prepared.name),
        true,
    )
    .await?;

    let credentials_json = serde_json::to_string(&prepared.credentials)
        .map_err(|error| Error::BadRequest(format!("credential serialization failed: {error}")))?;
    let encrypted_credentials = CryptoService::encrypt(&credentials_json)
        .map_err(|error| Error::BadRequest(error.to_string()))?;

    let record = CreateManagedServiceRecord {
        organization_id,
        server_id: params.server_id,
        volume_id: volume.id,
        name: prepared.name,
        slug: prepared.slug,
        kind: prepared.kind,
        image: prepared.image,
        container_name: prepared.container_name,
        internal_port: prepared.internal_port,
        database_name: prepared.database_name,
        username: prepared.username,
        encrypted_credentials,
    };

    let service = match ManagedServiceModel::create_declared(&ctx.db, record).await {
        Ok(service) => service,
        Err(error) => {
            if let Ok(unprotected) =
                PersistentVolumeModel::set_deletion_protection(&ctx.db, volume.id, false).await
            {
                let _ = unprotected.delete_record(&ctx.db).await;
            }
            return Err(error);
        }
    };

    audit_service(
        &ctx,
        &principal,
        &service,
        "managed_service.create",
        Some(serde_json::json!({
            "kind": service.kind.clone(),
            "server_id": service.server_id,
            "volume_id": service.volume_id,
            "image": service.image.clone(),
        })),
    )
    .await;

    format::json(serde_json::json!({
        "data": service.to_safe(),
        "message": "Managed service declared; start explicitly when ready"
    }))
}

#[debug_handler]
pub async fn get_one(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let (_, service) = authorized_service(&ctx, &headers, id, Permission::View).await?;
    format::json(serde_json::json!({ "data": service.to_safe(), "message": "ok" }))
}

#[debug_handler]
pub async fn start(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let (principal, service) =
        authorized_service(&ctx, &headers, id, Permission::ManageApplications).await?;
    let volume = PersistentVolumeModel::find_by_id(&ctx.db, service.volume_id).await?;
    let credentials = decrypt_credentials(&service)?;
    let (_, runtime) = runtime_for(&ctx.db, &service).await?;

    ManagedServiceModel::update_status(&ctx.db, service.id, "starting").await?;
    runtime
        .ensure_docker()
        .await
        .map_err(|error| Error::BadRequest(error.to_string()))?;
    runtime
        .ensure_network(ProxyService::MANAGED_NETWORK)
        .await
        .map_err(|error| Error::BadRequest(error.to_string()))?;
    runtime
        .ensure_volume(&volume.docker_volume_name)
        .await
        .map_err(|error| Error::BadRequest(error.to_string()))?;
    runtime
        .pull_image(&service.image)
        .await
        .map_err(|error| Error::BadRequest(error.to_string()))?;

    let config = ManagedServiceTemplateService::runtime_config(
        &service,
        &volume.docker_volume_name,
        &credentials,
    )?;
    let _ = runtime
        .stop_and_remove_container(&service.container_name)
        .await;
    runtime
        .run_managed_service_container(service.id, &config)
        .await
        .map_err(|error| {
            Error::BadRequest(redact_secrets(
                &error.to_string(),
                &credential_secret_values(&credentials),
            ))
        })?;

    if let Err(error) = wait_ready(&runtime, &service).await {
        let _ = ManagedServiceModel::update_status(&ctx.db, service.id, "unhealthy").await;
        return Err(error);
    }

    let service = ManagedServiceModel::update_status(&ctx.db, service.id, "running").await?;
    audit_service(
        &ctx,
        &principal,
        &service,
        "managed_service.start",
        Some(serde_json::json!({"image": service.image.clone()})),
    )
    .await;

    format::json(serde_json::json!({
        "data": service.to_safe(),
        "message": "Managed service is ready"
    }))
}

#[debug_handler]
pub async fn stop(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let (principal, service) =
        authorized_service(&ctx, &headers, id, Permission::ManageApplications).await?;
    let (_, runtime) = runtime_for(&ctx.db, &service).await?;
    runtime
        .stop_container(&service.container_name)
        .await
        .map_err(|error| Error::BadRequest(error.to_string()))?;
    let service = ManagedServiceModel::update_status(&ctx.db, service.id, "stopped").await?;
    audit_service(&ctx, &principal, &service, "managed_service.stop", None).await;
    format::json(serde_json::json!({ "data": service.to_safe(), "message": "ok" }))
}

#[debug_handler]
pub async fn restart(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let (principal, service) =
        authorized_service(&ctx, &headers, id, Permission::ManageApplications).await?;
    let (_, runtime) = runtime_for(&ctx.db, &service).await?;
    runtime
        .restart_container(&service.container_name)
        .await
        .map_err(|error| Error::BadRequest(error.to_string()))?;
    if let Err(error) = wait_ready(&runtime, &service).await {
        let _ = ManagedServiceModel::update_status(&ctx.db, service.id, "unhealthy").await;
        return Err(error);
    }
    let service = ManagedServiceModel::update_status(&ctx.db, service.id, "running").await?;
    audit_service(&ctx, &principal, &service, "managed_service.restart", None).await;
    format::json(serde_json::json!({ "data": service.to_safe(), "message": "ok" }))
}

#[debug_handler]
pub async fn status(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let (_, service) = authorized_service(&ctx, &headers, id, Permission::View).await?;
    let (_, runtime) = runtime_for(&ctx.db, &service).await?;
    let observed = runtime
        .container_status(&service.container_name)
        .await
        .map_err(|error| Error::BadRequest(error.to_string()))?;
    let ready = if observed == "running" {
        runtime
            .managed_service_ready(&service.container_name, &service.kind)
            .await
            .unwrap_or(false)
    } else {
        false
    };
    format::json(serde_json::json!({
        "data": {
            "service": service.to_safe(),
            "observed_status": observed,
            "ready": ready,
        },
        "message": "ok"
    }))
}

#[debug_handler]
pub async fn logs(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let (_, service) = authorized_service(&ctx, &headers, id, Permission::View).await?;
    let credentials = decrypt_credentials(&service)?;
    let (_, runtime) = runtime_for(&ctx.db, &service).await?;
    let output = runtime
        .container_logs(&service.container_name, 200)
        .await
        .map_err(|error| Error::BadRequest(error.to_string()))?;
    let safe = redact_secrets(&output, &credential_secret_values(&credentials));
    format::json(serde_json::json!({
        "data": { "service_id": service.id, "logs": safe },
        "message": "ok"
    }))
}

#[debug_handler]
pub async fn list_bindings(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let (_, _) = authorized_service(&ctx, &headers, id, Permission::View).await?;
    let bindings = ManagedServiceBindingModel::by_service(&ctx.db, id).await?;
    let mut safe = Vec::with_capacity(bindings.len());
    for binding in bindings {
        safe.push(binding.to_safe()?);
    }
    format::json(serde_json::json!({ "data": safe, "message": "ok" }))
}

#[debug_handler]
pub async fn bind(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
    Json(params): Json<CreateManagedServiceBindingParams>,
) -> Result<Response> {
    let (principal, service) =
        authorized_service(&ctx, &headers, id, Permission::ManageApplications).await?;
    let application = ApplicationModel::find_by_id(&ctx.db, params.application_id).await?;
    let application_org = principal
        .application_organization(&ctx.db, application.id, Permission::ManageApplications)
        .await?;

    if application_org != service.organization_id {
        return Err(Error::BadRequest(
            "managed service and application must belong to the same organization".to_string(),
        ));
    }
    if application.server_id != service.server_id {
        return Err(Error::BadRequest(
            "managed service and application must target the same server".to_string(),
        ));
    }
    if application.workload_type == "compose" {
        return Err(Error::BadRequest(
            "managed service binding currently supports single-container applications only"
                .to_string(),
        ));
    }

    let template = ManagedServiceTemplateService::template(&service.kind)?;
    let prefix = ManagedServiceTemplateService::normalize_env_prefix(
        params.env_prefix.as_deref(),
        template.default_env_prefix,
    )?;

    let existing_binding = managed_service_bindings::Entity::find()
        .filter(managed_service_bindings::Column::ServiceId.eq(service.id))
        .filter(managed_service_bindings::Column::ApplicationId.eq(application.id))
        .filter(managed_service_bindings::Column::EnvPrefix.eq(&prefix))
        .one(&ctx.db)
        .await?;
    if let Some(existing) = existing_binding {
        return format::json(serde_json::json!({
            "data": existing.to_safe()?,
            "message": "Binding already exists"
        }));
    }

    let credentials = decrypt_credentials(&service)?;
    let encrypted_password = CryptoService::encrypt(&credentials.password)
        .map_err(|error| Error::BadRequest(error.to_string()))?;
    let values =
        ManagedServiceTemplateService::binding_values(&service, &encrypted_password, &prefix);
    let now = Utc::now();
    let mut env_keys = Vec::with_capacity(values.len());

    for (key, value, is_secret) in values {
        env_keys.push(key.clone());
        let existing = environment_variables::Entity::find()
            .filter(environment_variables::Column::ApplicationId.eq(application.id))
            .filter(environment_variables::Column::Key.eq(&key))
            .one(&ctx.db)
            .await?;

        match existing {
            Some(model) => {
                let mut active: environment_variables::ActiveModel = model.into();
                active.encrypted_value = Set(value);
                active.is_secret = Set(is_secret);
                active.updated_at = Set(now.into());
                active.update(&ctx.db).await?;
            }
            None => {
                environment_variables::ActiveModel {
                    application_id: Set(application.id),
                    key: Set(key),
                    encrypted_value: Set(value),
                    is_secret: Set(is_secret),
                    created_at: Set(now.into()),
                    updated_at: Set(now.into()),
                    ..Default::default()
                }
                .insert(&ctx.db)
                .await?;
            }
        }
    }

    let binding =
        ManagedServiceBindingModel::create(&ctx.db, service.id, application.id, prefix, env_keys)
            .await?;

    audit_service(
        &ctx,
        &principal,
        &service,
        "managed_service.bind",
        Some(serde_json::json!({
            "application_id": application.id,
            "binding_id": binding.id,
            "env_keys": binding.env_keys()?,
        })),
    )
    .await;

    format::json(serde_json::json!({
        "data": binding.to_safe()?,
        "message": "Application binding created; redeploy the application to consume it"
    }))
}

fn credential_secret_values(credentials: &ManagedServiceCredentials) -> Vec<String> {
    let mut values = vec![credentials.password.clone()];
    if let Some(root) = &credentials.root_password {
        values.push(root.clone());
    }
    values
}
