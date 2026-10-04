use crate::{
    models::{
        applications::Model as ApplicationModel,
        audit_events::{AuditEventInput, Model as AuditEventModel},
        persistent_volumes::{
            CreatePersistentVolumeParams, Model as PersistentVolumeModel,
            UpdateVolumeProtectionParams,
        },
        servers::Model as ServerModel,
        volume_attachments::{AttachVolumeParams, Model as VolumeAttachmentModel},
    },
    services::{
        access_control::{Permission, Principal},
        remote::RemoteRuntime,
    },
};
use axum::{extract::Query, http::HeaderMap};
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct DeleteVolumeQuery {
    pub confirm: String,
}

#[derive(Debug, Serialize)]
pub struct PersistentVolumeView {
    pub id: i64,
    pub organization_id: i64,
    pub server_id: i64,
    pub name: String,
    pub docker_volume_name: String,
    pub deletion_protected: bool,
    pub orphaned: bool,
    pub attachment: Option<VolumeAttachmentModel>,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
    pub updated_at: chrono::DateTime<chrono::FixedOffset>,
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/volumes")
        .add("/", get(list))
        .add("/", post(create))
        .add("{id}", get(get_one))
        .add("{id}", delete(remove))
        .add("{id}/protection", put(set_protection))
        .add("{id}/attachments", post(attach))
        .add("{id}/attachments/{attachment_id}", delete(detach))
}

async fn authorized_volume(
    ctx: &AppContext,
    headers: &HeaderMap,
    volume_id: i64,
    permission: Permission,
) -> Result<(Principal, PersistentVolumeModel)> {
    let principal = Principal::authenticate(ctx, headers).await?;
    let volume = PersistentVolumeModel::find_by_id(&ctx.db, volume_id).await?;
    principal
        .require(&ctx.db, volume.organization_id, permission)
        .await?;
    Ok((principal, volume))
}

async fn view(
    db: &DatabaseConnection,
    volume: PersistentVolumeModel,
) -> Result<PersistentVolumeView> {
    let attachment = VolumeAttachmentModel::by_volume(db, volume.id)
        .await?
        .into_iter()
        .next();
    Ok(PersistentVolumeView {
        id: volume.id,
        organization_id: volume.organization_id,
        server_id: volume.server_id,
        name: volume.name,
        docker_volume_name: volume.docker_volume_name,
        deletion_protected: volume.deletion_protected,
        orphaned: attachment.is_none(),
        attachment,
        created_at: volume.created_at,
        updated_at: volume.updated_at,
    })
}

async fn audit_volume(
    ctx: &AppContext,
    principal: &Principal,
    organization_id: i64,
    action: &str,
    volume_id: i64,
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
            resource_type: Some("persistent_volume".to_string()),
            resource_id: Some(volume_id.to_string()),
            outcome: "success".to_string(),
            request_id: Some(principal.request_id.clone()),
            metadata: principal.audit_metadata(metadata),
        },
    )
    .await;
}

#[debug_handler]
pub async fn list(headers: HeaderMap, State(ctx): State<AppContext>) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    let organization_ids = principal.organization_ids(&ctx.db).await?;
    let volumes =
        PersistentVolumeModel::all_for_organizations(&ctx.db, &organization_ids).await?;

    let mut result = Vec::with_capacity(volumes.len());
    for volume in volumes {
        result.push(view(&ctx.db, volume).await?);
    }
    format::json(serde_json::json!({ "data": result }))
}

#[debug_handler]
pub async fn create(
    headers: HeaderMap,
    State(ctx): State<AppContext>,
    Json(params): Json<CreatePersistentVolumeParams>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    let organization_id = principal
        .server_organization(&ctx.db, params.server_id, Permission::ManageApplications)
        .await?;

    let volume = PersistentVolumeModel::create_declared(
        &ctx.db,
        organization_id,
        params.server_id,
        &params.name,
        params.deletion_protected.unwrap_or(true),
    )
    .await?;

    audit_volume(
        &ctx,
        &principal,
        organization_id,
        "volume.create",
        volume.id,
        Some(serde_json::json!({
            "server_id": volume.server_id,
            "deletion_protected": volume.deletion_protected,
        })),
    )
    .await;

    format::json(serde_json::json!({
        "data": view(&ctx.db, volume).await?,
        "message": "Persistent volume declared"
    }))
}

#[debug_handler]
pub async fn get_one(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let (_, volume) = authorized_volume(&ctx, &headers, id, Permission::View).await?;
    format::json(serde_json::json!({ "data": view(&ctx.db, volume).await? }))
}

#[debug_handler]
pub async fn set_protection(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
    Json(params): Json<UpdateVolumeProtectionParams>,
) -> Result<Response> {
    let (principal, volume) =
        authorized_volume(&ctx, &headers, id, Permission::ManageApplications).await?;
    let organization_id = volume.organization_id;
    let updated = PersistentVolumeModel::set_deletion_protection(
        &ctx.db,
        id,
        params.deletion_protected,
    )
    .await?;

    audit_volume(
        &ctx,
        &principal,
        organization_id,
        "volume.protection.update",
        id,
        Some(serde_json::json!({
            "deletion_protected": updated.deletion_protected,
        })),
    )
    .await;

    format::json(serde_json::json!({
        "data": view(&ctx.db, updated).await?,
        "message": "Volume protection updated"
    }))
}

#[debug_handler]
pub async fn attach(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
    Json(params): Json<AttachVolumeParams>,
) -> Result<Response> {
    let (principal, volume) =
        authorized_volume(&ctx, &headers, id, Permission::ManageApplications).await?;
    let application = ApplicationModel::find_by_id(&ctx.db, params.application_id).await?;
    let application_org = principal
        .application_organization(
            &ctx.db,
            application.id,
            Permission::ManageApplications,
        )
        .await?;

    if application_org != volume.organization_id {
        return Err(Error::BadRequest(
            "volume and application must belong to the same organization".to_string(),
        ));
    }
    if application.server_id != volume.server_id {
        return Err(Error::BadRequest(
            "volume and application must target the same server".to_string(),
        ));
    }
    if application.workload_type == "compose" {
        return Err(Error::BadRequest(
            "Moonships-managed persistent volume attachments are not supported for Compose workloads"
                .to_string(),
        ));
    }

    let attachment = VolumeAttachmentModel::attach(
        &ctx.db,
        volume.id,
        application.id,
        &params.mount_path,
        params.read_only.unwrap_or(false),
    )
    .await?;

    audit_volume(
        &ctx,
        &principal,
        volume.organization_id,
        "volume.attach",
        volume.id,
        Some(serde_json::json!({
            "application_id": application.id,
            "attachment_id": attachment.id,
            "mount_path": attachment.mount_path,
            "read_only": attachment.read_only,
        })),
    )
    .await;

    format::json(serde_json::json!({
        "data": view(&ctx.db, volume).await?,
        "message": "Volume attached"
    }))
}

#[debug_handler]
pub async fn detach(
    headers: HeaderMap,
    Path((id, attachment_id)): Path<(i64, i64)>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let (principal, volume) =
        authorized_volume(&ctx, &headers, id, Permission::ManageApplications).await?;
    let attachment = VolumeAttachmentModel::find_by_id(&ctx.db, attachment_id).await?;
    if attachment.volume_id != volume.id {
        return Err(Error::BadRequest(
            "attachment does not belong to this volume".to_string(),
        ));
    }
    let application_id = attachment.application_id;
    attachment.detach(&ctx.db).await?;

    audit_volume(
        &ctx,
        &principal,
        volume.organization_id,
        "volume.detach",
        volume.id,
        Some(serde_json::json!({
            "application_id": application_id,
            "attachment_id": attachment_id,
        })),
    )
    .await;

    format::json(serde_json::json!({
        "data": view(&ctx.db, volume).await?,
        "message": "Volume detached; volume data remains declared and protected"
    }))
}

#[debug_handler]
pub async fn remove(
    headers: HeaderMap,
    Path(id): Path<i64>,
    Query(params): Query<DeleteVolumeQuery>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let (principal, volume) =
        authorized_volume(&ctx, &headers, id, Permission::ManageApplications).await?;

    if params.confirm.trim() != volume.name {
        return Err(Error::BadRequest(
            "volume deletion confirmation must exactly match the volume name".to_string(),
        ));
    }
    if volume.deletion_protected {
        return Err(Error::BadRequest(
            "volume deletion protection is enabled; disable it explicitly first".to_string(),
        ));
    }
    if volume.attachment_count(&ctx.db).await? > 0 {
        return Err(Error::BadRequest(
            "attached volume cannot be deleted; detach it first".to_string(),
        ));
    }

    let server = ServerModel::find_by_id(&ctx.db, volume.server_id).await?;
    let runtime = RemoteRuntime::connect(&server)
        .await
        .map_err(|error| Error::BadRequest(format!("volume target unavailable: {error}")))?;
    runtime
        .remove_volume(&volume.docker_volume_name)
        .await
        .map_err(|error| Error::BadRequest(format!("remote volume deletion failed: {error}")))?;

    let organization_id = volume.organization_id;
    let name = volume.name.clone();
    volume.delete_record(&ctx.db).await?;

    audit_volume(
        &ctx,
        &principal,
        organization_id,
        "volume.delete",
        id,
        Some(serde_json::json!({ "name": name })),
    )
    .await;

    format::json(serde_json::json!({
        "message": "Persistent volume deleted from the target and control plane"
    }))
}
