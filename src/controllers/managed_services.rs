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

async fn wait_ready_named(runtime: &RemoteRuntime, container_name: &str, kind: &str) -> Result<()> {
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

async fn backup_for_service(
    db: &DatabaseConnection,
    service_id: i64,
    backup_id: i64,
) -> Result<ManagedServiceBackupModel> {
    let backup = ManagedServiceBackupModel::find_by_id(db, backup_id).await?;
    if backup.service_id != service_id {
        return Err(Error::BadRequest(
            "backup does not belong to this managed service".to_string(),
        ));
    }
    Ok(backup)
}

async fn audit_backup(
    ctx: &AppContext,
    principal: &Principal,
    service: &ManagedServiceModel,
    backup_id: i64,
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
            resource_type: Some("managed_service_backup".to_string()),
            resource_id: Some(backup_id.to_string()),
            outcome: "success".to_string(),
            request_id: Some(principal.request_id.clone()),
            metadata: principal.audit_metadata(metadata),
        },
    )
    .await;
}

fn remote_backup_error<E: ToString>(error: E, credentials: &ManagedServiceCredentials) -> Error {
    Error::BadRequest(redact_secrets(
        &error.to_string(),
        &credential_secret_values(credentials),
    ))
}

async fn prune_service_backups(
    db: &DatabaseConnection,
    runtime: &RemoteRuntime,
    service_id: i64,
) -> Result<()> {
    let keep = std::env::var("MOONSHIPS_SERVICE_BACKUP_RETENTION")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(7)
        .max(1);
    let backups = ManagedServiceBackupModel::by_service(db, service_id).await?;
    let mut successful_seen = 0usize;
    for backup in backups {
        if backup.status != "success" {
            continue;
        }
        successful_seen += 1;
        if successful_seen <= keep || backup.deletion_protected {
            continue;
        }
        runtime
            .remove_volume_snapshot(service_id, backup.id)
            .await
            .map_err(|error| Error::BadRequest(error.to_string()))?;
        backup.delete_record(db).await?;
    }
    Ok(())
}

#[debug_handler]
pub async fn list_backups(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let (_, _) = authorized_service(&ctx, &headers, id, Permission::View).await?;
    let backups = ManagedServiceBackupModel::by_service(&ctx.db, id).await?;
    let safe = backups
        .into_iter()
        .map(|backup| backup.to_safe())
        .collect::<Vec<_>>();
    format::json(serde_json::json!({ "data": safe, "message": "ok" }))
}

#[debug_handler]
pub async fn create_backup(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
    Json(params): Json<CreateManagedServiceBackupParams>,
) -> Result<Response> {
    let (principal, service) =
        authorized_service(&ctx, &headers, id, Permission::ManageApplications).await?;
    let volume = PersistentVolumeModel::find_by_id(&ctx.db, service.volume_id).await?;
    let credentials = decrypt_credentials(&service)?;
    let (_, runtime) = runtime_for(&ctx.db, &service).await?;
    let helper_image = ManagedServiceBackupService::helper_image()?;

    runtime
        .ensure_docker()
        .await
        .map_err(|error| remote_backup_error(error, &credentials))?;
    runtime
        .ensure_network(ProxyService::MANAGED_NETWORK)
        .await
        .map_err(|error| remote_backup_error(error, &credentials))?;
    runtime
        .pull_image(&service.image)
        .await
        .map_err(|error| remote_backup_error(error, &credentials))?;

    let backup = ManagedServiceBackupModel::start(
        &ctx.db,
        service.organization_id,
        service.id,
        service.server_id,
        volume.id,
        &helper_image,
        params.deletion_protected.unwrap_or(false),
    )
    .await?;
    let plan = ManagedServiceBackupService::plan(service.id, backup.id, helper_image)?;
    let backup =
        ManagedServiceBackupModel::set_artifact_path(&ctx.db, backup.id, &plan.artifact_path)
            .await?;

    let observed = runtime
        .container_status(&service.container_name)
        .await
        .map_err(|error| remote_backup_error(error, &credentials))?;
    let was_running = observed == "running";

    let operation = async {
        if was_running {
            runtime
                .stop_container(&service.container_name)
                .await
                .map_err(|error| remote_backup_error(error, &credentials))?;
        }

        let snapshot = runtime
            .snapshot_volume(
                service.id,
                backup.id,
                &volume.docker_volume_name,
                &plan.helper_image,
            )
            .await
            .map_err(|error| remote_backup_error(error, &credentials))?;

        runtime
            .ensure_volume(&plan.verification_volume_name)
            .await
            .map_err(|error| remote_backup_error(error, &credentials))?;
        runtime
            .restore_volume_snapshot(
                service.id,
                backup.id,
                &plan.verification_volume_name,
                &plan.helper_image,
            )
            .await
            .map_err(|error| remote_backup_error(error, &credentials))?;

        let verification_config = ManagedServiceBackupService::verification_runtime_config(
            &service,
            &credentials,
            &plan,
        )?;
        let _ = runtime
            .stop_and_remove_container(&plan.verification_container_name)
            .await;
        runtime
            .run_managed_service_container(backup.id, &verification_config)
            .await
            .map_err(|error| remote_backup_error(error, &credentials))?;
        wait_ready_named(&runtime, &plan.verification_container_name, &service.kind).await?;

        Ok::<_, Error>(snapshot)
    }
    .await;

    let _ = runtime
        .stop_and_remove_container(&plan.verification_container_name)
        .await;
    let _ = runtime.remove_volume(&plan.verification_volume_name).await;

    let restart_result = if was_running {
        match runtime.start_container(&service.container_name).await {
            Ok(()) => wait_ready(&runtime, &service).await,
            Err(error) => Err(remote_backup_error(error, &credentials)),
        }
    } else {
        Ok(())
    };

    let snapshot = match (operation, restart_result) {
        (Ok(snapshot), Ok(())) => snapshot,
        (Err(error), _) => {
            let message =
                redact_secrets(&error.to_string(), &credential_secret_values(&credentials));
            let _ = ManagedServiceBackupModel::fail(&ctx.db, backup.id, &message).await;
            return Err(Error::BadRequest(message));
        }
        (Ok(_), Err(error)) => {
            let _ = ManagedServiceModel::update_status(&ctx.db, service.id, "unhealthy").await;
            let message =
                redact_secrets(&error.to_string(), &credential_secret_values(&credentials));
            let _ = ManagedServiceBackupModel::fail(&ctx.db, backup.id, &message).await;
            return Err(Error::BadRequest(format!(
                "backup snapshot verified but original service failed to resume: {message}"
            )));
        }
    };

    let verification_message = format!(
        "offline_volume_snapshot=ok; isolated_restore=ok; readiness=ok; helper_image={}",
        plan.helper_image
    );
    let backup = ManagedServiceBackupModel::complete(
        &ctx.db,
        backup.id,
        snapshot.size_bytes,
        &snapshot.sha256,
        &verification_message,
    )
    .await?;

    let _ = prune_service_backups(&ctx.db, &runtime, service.id).await;

    audit_backup(
        &ctx,
        &principal,
        &service,
        backup.id,
        "managed_service.backup.create",
        Some(serde_json::json!({
            "volume_id": backup.volume_id,
            "size_bytes": backup.size_bytes,
            "sha256": backup.sha256.clone(),
            "verified": backup.verified,
            "deletion_protected": backup.deletion_protected,
        })),
    )
    .await;

    format::json(serde_json::json!({
        "data": backup.to_safe(),
        "message": "Managed service backup completed and verified through an isolated restore"
    }))
}

#[debug_handler]
pub async fn restore_backup(
    headers: HeaderMap,
    Path((id, backup_id)): Path<(i64, i64)>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let (principal, service) =
        authorized_service(&ctx, &headers, id, Permission::ManageApplications).await?;
    let backup = backup_for_service(&ctx.db, service.id, backup_id).await?;
    if backup.status != "success" || !backup.verified {
        return Err(Error::BadRequest(
            "only successful verified backups can be restored".to_string(),
        ));
    }
    if principal.confirmation.as_deref() != Some(service.name.as_str()) {
        return Err(Error::BadRequest(
            "restore confirmation must exactly match the managed service name in x-moonships-confirmation"
                .to_string(),
        ));
    }

    let volume = PersistentVolumeModel::find_by_id(&ctx.db, service.volume_id).await?;
    if backup.volume_id != volume.id || backup.server_id != service.server_id {
        return Err(Error::BadRequest(
            "backup identity no longer matches the service volume/server".to_string(),
        ));
    }

    let credentials = decrypt_credentials(&service)?;
    let (_, runtime) = runtime_for(&ctx.db, &service).await?;
    let observed_status = runtime
        .container_status(&service.container_name)
        .await
        .map_err(|error| remote_backup_error(error, &credentials))?;
    if observed_status == "running" {
        return Err(Error::BadRequest(
            "refusing to overwrite an active protected service; stop it before restore".to_string(),
        ));
    }

    let observed = runtime
        .volume_snapshot_metadata(service.id, backup.id)
        .await
        .map_err(|error| remote_backup_error(error, &credentials))?;
    ManagedServiceBackupService::verify_metadata(&backup, &observed)?;

    let plan =
        ManagedServiceBackupService::plan(service.id, backup.id, backup.helper_image.clone())?;

    runtime
        .ensure_network(ProxyService::MANAGED_NETWORK)
        .await
        .map_err(|error| remote_backup_error(error, &credentials))?;
    runtime
        .pull_image(&service.image)
        .await
        .map_err(|error| remote_backup_error(error, &credentials))?;
    runtime
        .ensure_volume(&plan.verification_volume_name)
        .await
        .map_err(|error| remote_backup_error(error, &credentials))?;
    runtime
        .restore_volume_snapshot(
            service.id,
            backup.id,
            &plan.verification_volume_name,
            &plan.helper_image,
        )
        .await
        .map_err(|error| remote_backup_error(error, &credentials))?;

    let verification_config =
        ManagedServiceBackupService::verification_runtime_config(&service, &credentials, &plan)?;
    let verification = async {
        let _ = runtime
            .stop_and_remove_container(&plan.verification_container_name)
            .await;
        runtime
            .run_managed_service_container(backup.id, &verification_config)
            .await
            .map_err(|error| remote_backup_error(error, &credentials))?;
        wait_ready_named(&runtime, &plan.verification_container_name, &service.kind).await
    }
    .await;
    let _ = runtime
        .stop_and_remove_container(&plan.verification_container_name)
        .await;
    let _ = runtime.remove_volume(&plan.verification_volume_name).await;
    verification?;

    runtime
        .restore_volume_snapshot(
            service.id,
            backup.id,
            &volume.docker_volume_name,
            &plan.helper_image,
        )
        .await
        .map_err(|error| remote_backup_error(error, &credentials))?;

    let _ = runtime
        .stop_and_remove_container(&service.container_name)
        .await;
    let config = ManagedServiceTemplateService::runtime_config(
        &service,
        &volume.docker_volume_name,
        &credentials,
    )?;
    runtime
        .run_managed_service_container(service.id, &config)
        .await
        .map_err(|error| remote_backup_error(error, &credentials))?;
    if let Err(error) = wait_ready(&runtime, &service).await {
        let _ = ManagedServiceModel::update_status(&ctx.db, service.id, "unhealthy").await;
        return Err(error);
    }

    let service = ManagedServiceModel::update_status(&ctx.db, service.id, "running").await?;
    let backup = ManagedServiceBackupModel::mark_restored(&ctx.db, backup.id).await?;
    audit_backup(
        &ctx,
        &principal,
        &service,
        backup.id,
        "managed_service.backup.restore",
        Some(serde_json::json!({
            "volume_id": backup.volume_id,
            "sha256": backup.sha256.clone(),
            "restored_at": backup.restored_at.clone(),
        })),
    )
    .await;

    format::json(serde_json::json!({
        "data": {
            "service": service.to_safe(),
            "backup": backup.to_safe(),
        },
        "message": "Backup restored into the protected service volume and service readiness verified"
    }))
}

#[debug_handler]
pub async fn set_backup_protection(
    headers: HeaderMap,
    Path((id, backup_id)): Path<(i64, i64)>,
    State(ctx): State<AppContext>,
    Json(params): Json<UpdateManagedServiceBackupProtectionParams>,
) -> Result<Response> {
    let (principal, service) =
        authorized_service(&ctx, &headers, id, Permission::ManageApplications).await?;
    let _ = backup_for_service(&ctx.db, service.id, backup_id).await?;
    let backup = ManagedServiceBackupModel::set_deletion_protection(
        &ctx.db,
        backup_id,
        params.deletion_protected,
    )
    .await?;
    audit_backup(
        &ctx,
        &principal,
        &service,
        backup.id,
        "managed_service.backup.protection.update",
        Some(serde_json::json!({
            "deletion_protected": backup.deletion_protected,
        })),
    )
    .await;
    format::json(serde_json::json!({ "data": backup.to_safe(), "message": "ok" }))
}

#[debug_handler]
pub async fn remove_backup(
    headers: HeaderMap,
    Path((id, backup_id)): Path<(i64, i64)>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let (principal, service) =
        authorized_service(&ctx, &headers, id, Permission::ManageApplications).await?;
    let backup = backup_for_service(&ctx.db, service.id, backup_id).await?;
    if backup.deletion_protected {
        return Err(Error::BadRequest(
            "backup deletion protection is enabled; disable it explicitly first".to_string(),
        ));
    }
    let expected_confirmation = format!("backup-{}", backup.id);
    if principal.confirmation.as_deref() != Some(expected_confirmation.as_str()) {
        return Err(Error::BadRequest(format!(
            "backup deletion confirmation must equal '{expected_confirmation}' in x-moonships-confirmation"
        )));
    }

    let credentials = decrypt_credentials(&service)?;
    let (_, runtime) = runtime_for(&ctx.db, &service).await?;
    runtime
        .remove_volume_snapshot(service.id, backup.id)
        .await
        .map_err(|error| remote_backup_error(error, &credentials))?;

    let deleted_id = backup.id;
    backup.delete_record(&ctx.db).await?;
    audit_backup(
        &ctx,
        &principal,
        &service,
        deleted_id,
        "managed_service.backup.delete",
        None,
    )
    .await;

    format::json(serde_json::json!({
        "data": null,
        "message": "Managed service backup artifact and metadata deleted"
    }))
}

fn credential_secret_values(credentials: &ManagedServiceCredentials) -> Vec<String> {
    let mut values = vec![credentials.password.clone()];
    if let Some(root) = &credentials.root_password {
        values.push(root.clone());
    }
    values
}
