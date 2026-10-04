use crate::{
    models::{
        _entities::{applications, projects, registry_credentials, servers},
        api_tokens::{CreateApiTokenParams, Model as ApiTokenModel},
        audit_events::{AuditEventInput, Model as AuditEventModel},
        auth_rate_limits::Model as AuthRateLimitModel,
        organization_memberships::{Model as MembershipModel, SetMembershipParams},
        organizations::{CreateOrganizationParams, Model as OrganizationModel},
        registry_credentials::{CreateRegistryCredentialParams, Model as RegistryCredentialModel},
        server_pool_members::{Model as ServerPoolMemberModel, SetServerPoolMemberParams},
        server_pools::{CreateServerPoolParams, Model as ServerPoolModel},
        servers::Model as ServerModel,
    },
    services::access_control::{Permission, Principal},
};
use axum::http::HeaderMap;
use loco_rs::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/organizations")
        .add("", get(list))
        .add("", post(create))
        .add("{id}", get(get_one))
        .add("{id}/members", get(list_members))
        .add("{id}/members", put(upsert_member))
        .add("{id}/tokens", get(list_tokens))
        .add("{id}/tokens", post(create_token))
        .add("{id}/tokens/{token_id}", delete(revoke_token))
        .add("{id}/audit", get(list_audit))
        .add("{id}/server-pools", get(list_server_pools))
        .add("{id}/server-pools", post(create_server_pool))
        .add(
            "{id}/server-pools/{pool_id}/members",
            put(upsert_server_pool_member),
        )
        .add("{id}/registry-credentials", get(list_registry_credentials))
        .add(
            "{id}/registry-credentials",
            post(create_registry_credential),
        )
        .add(
            "{id}/registry-credentials/{credential_id}",
            delete(remove_registry_credential),
        )
        .add("{id}/claim-legacy", post(claim_legacy))
}

#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    pub limit: Option<u64>,
}

async fn enforce_sensitive_action(
    ctx: &AppContext,
    principal: &Principal,
    organization_id: i64,
    action: &str,
) -> Result<()> {
    let key = format!("{}:{organization_id}:{action}", principal.user.id);
    let decision =
        AuthRateLimitModel::check_and_record(&ctx.db, "sensitive_action", &key, 30, 60, 60).await?;

    if !decision.allowed {
        return Err(Error::BadRequest(format!(
            "sensitive action rate limited; retry after {} seconds",
            decision.retry_after_seconds.unwrap_or(60)
        )));
    }

    Ok(())
}

#[debug_handler]
pub async fn list(headers: HeaderMap, State(ctx): State<AppContext>) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    let memberships = MembershipModel::list_for_user(&ctx.db, principal.user.id).await?;
    let mut organizations = Vec::new();
    for membership in memberships {
        if let Ok(org) = OrganizationModel::find_by_id(&ctx.db, membership.organization_id).await {
            organizations.push(serde_json::json!({
                "organization": org,
                "role": membership.role,
            }));
        }
    }
    format::json(serde_json::json!({"data": organizations}))
}

#[debug_handler]
pub async fn create(
    headers: HeaderMap,
    State(ctx): State<AppContext>,
    Json(params): Json<CreateOrganizationParams>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    let org = OrganizationModel::create(&ctx.db, &params).await?;
    let membership = MembershipModel::upsert(
        &ctx.db,
        org.id,
        &SetMembershipParams {
            user_id: principal.user.id,
            role: "owner".to_string(),
            is_active: Some(true),
        },
    )
    .await?;

    let (actor_kind, actor_id) = principal.audit_actor();
    let _ = AuditEventModel::append(
        &ctx.db,
        AuditEventInput {
            organization_id: Some(org.id),
            actor_kind: actor_kind.to_string(),
            actor_id,
            action: "organization.create".to_string(),
            resource_type: Some("organization".to_string()),
            resource_id: Some(org.id.to_string()),
            outcome: "success".to_string(),
            request_id: Some(principal.request_id.clone()),
            metadata: principal.audit_metadata(None),
        },
    )
    .await;

    format::json(serde_json::json!({
        "data": {
            "organization": org,
            "membership": membership
        }
    }))
}

#[debug_handler]
pub async fn get_one(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    principal.require(&ctx.db, id, Permission::View).await?;
    let org = OrganizationModel::find_by_id(&ctx.db, id).await?;
    format::json(serde_json::json!({"data": org}))
}

#[debug_handler]
pub async fn list_members(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    principal
        .require(&ctx.db, id, Permission::ManageOrganization)
        .await?;
    let memberships = MembershipModel::list_for_organization(&ctx.db, id).await?;
    format::json(serde_json::json!({"data": memberships}))
}

#[debug_handler]
pub async fn upsert_member(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
    Json(params): Json<SetMembershipParams>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    let actor_membership = principal
        .require(&ctx.db, id, Permission::ManageOrganization)
        .await?;
    enforce_sensitive_action(&ctx, &principal, id, "membership.upsert").await?;

    let existing = MembershipModel::find_for_user(&ctx.db, id, params.user_id).await?;
    let target_is_owner = existing
        .as_ref()
        .map(MembershipModel::is_owner)
        .unwrap_or(false);
    let grants_owner =
        params.role.eq_ignore_ascii_case("owner") && params.is_active.unwrap_or(true);

    if (target_is_owner || grants_owner) && !actor_membership.is_owner() {
        return unauthorized("only an organization owner can modify owner membership");
    }

    if target_is_owner && (!grants_owner || !params.is_active.unwrap_or(true)) {
        let owners = MembershipModel::active_owner_count(&ctx.db, id).await?;
        if owners <= 1 {
            return Err(Error::BadRequest(
                "organization must retain at least one active owner".to_string(),
            ));
        }
    }

    let membership = MembershipModel::upsert(&ctx.db, id, &params).await?;
    let (actor_kind, actor_id) = principal.audit_actor();
    let _ = AuditEventModel::append(
        &ctx.db,
        AuditEventInput {
            organization_id: Some(id),
            actor_kind: actor_kind.to_string(),
            actor_id,
            action: "membership.upsert".to_string(),
            resource_type: Some("user".to_string()),
            resource_id: Some(params.user_id.to_string()),
            outcome: "success".to_string(),
            request_id: Some(principal.request_id.clone()),
            metadata: principal.audit_metadata(Some(serde_json::json!({"role": membership.role}))),
        },
    )
    .await;

    format::json(serde_json::json!({"data": membership}))
}

#[debug_handler]
pub async fn list_tokens(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    principal.require(&ctx.db, id, Permission::View).await?;
    let tokens = ApiTokenModel::list_for_user(&ctx.db, principal.user.id)
        .await?
        .into_iter()
        .filter(|token| token.organization_id == id)
        .map(|token| token.to_safe())
        .collect::<Vec<_>>();
    format::json(serde_json::json!({"data": tokens}))
}

#[debug_handler]
pub async fn create_token(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
    Json(mut params): Json<CreateApiTokenParams>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    principal
        .require(&ctx.db, id, Permission::ManageOrganization)
        .await?;
    enforce_sensitive_action(&ctx, &principal, id, "api_token.create").await?;
    params.organization_id = id;

    let created = ApiTokenModel::create_token(&ctx.db, principal.user.id, &params).await?;
    let (actor_kind, actor_id) = principal.audit_actor();
    let _ = AuditEventModel::append(
        &ctx.db,
        AuditEventInput {
            organization_id: Some(id),
            actor_kind: actor_kind.to_string(),
            actor_id,
            action: "api_token.create".to_string(),
            resource_type: Some("api_token".to_string()),
            resource_id: Some(created.record.id.to_string()),
            outcome: "success".to_string(),
            request_id: Some(principal.request_id.clone()),
            metadata: principal.audit_metadata(Some(serde_json::json!({
                "scopes": created.record.scopes(),
                "token_prefix": created.record.token_prefix,
            }))),
        },
    )
    .await;

    format::json(serde_json::json!({
        "data": {
            "token": created.token,
            "record": created.record.to_safe()
        },
        "message": "Store this API token now; it will not be shown again"
    }))
}

#[debug_handler]
pub async fn revoke_token(
    headers: HeaderMap,
    Path((id, token_id)): Path<(i64, i64)>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    principal
        .require(&ctx.db, id, Permission::ManageOrganization)
        .await?;
    enforce_sensitive_action(&ctx, &principal, id, "api_token.revoke").await?;

    let token = ApiTokenModel::revoke_for_organization(&ctx.db, token_id, id).await?;

    let (actor_kind, actor_id) = principal.audit_actor();
    let _ = AuditEventModel::append(
        &ctx.db,
        AuditEventInput {
            organization_id: Some(id),
            actor_kind: actor_kind.to_string(),
            actor_id,
            action: "api_token.revoke".to_string(),
            resource_type: Some("api_token".to_string()),
            resource_id: Some(token.id.to_string()),
            outcome: "success".to_string(),
            request_id: Some(principal.request_id.clone()),
            metadata: None,
        },
    )
    .await;

    format::json(serde_json::json!({"data": token.to_safe()}))
}

#[debug_handler]
pub async fn list_audit(
    headers: HeaderMap,
    Path(id): Path<i64>,
    Query(query): Query<AuditQuery>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    principal
        .require(&ctx.db, id, Permission::ManageOrganization)
        .await?;
    let events =
        AuditEventModel::list_for_organization(&ctx.db, id, query.limit.unwrap_or(100)).await?;
    format::json(serde_json::json!({"data": events}))
}

#[debug_handler]
pub async fn claim_legacy(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    principal.require(&ctx.db, id, Permission::Owner).await?;

    let project_result = projects::Entity::update_many()
        .col_expr(
            projects::Column::OrganizationId,
            sea_orm::sea_query::Expr::value(id),
        )
        .filter(projects::Column::OrganizationId.is_null())
        .exec(&ctx.db)
        .await?;

    let server_result = servers::Entity::update_many()
        .col_expr(
            servers::Column::OrganizationId,
            sea_orm::sea_query::Expr::value(id),
        )
        .filter(servers::Column::OrganizationId.is_null())
        .exec(&ctx.db)
        .await?;

    let (actor_kind, actor_id) = principal.audit_actor();
    let _ = AuditEventModel::append(
        &ctx.db,
        AuditEventInput {
            organization_id: Some(id),
            actor_kind: actor_kind.to_string(),
            actor_id,
            action: "organization.claim_legacy".to_string(),
            resource_type: Some("organization".to_string()),
            resource_id: Some(id.to_string()),
            outcome: "success".to_string(),
            request_id: Some(principal.request_id.clone()),
            metadata: principal.audit_metadata(Some(serde_json::json!({
                "projects_claimed": project_result.rows_affected,
                "servers_claimed": server_result.rows_affected,
            }))),
        },
    )
    .await;

    format::json(serde_json::json!({
        "data": {
            "projects_claimed": project_result.rows_affected,
            "servers_claimed": server_result.rows_affected
        }
    }))
}

#[debug_handler]
pub async fn list_server_pools(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    principal.require(&ctx.db, id, Permission::View).await?;
    let pools = ServerPoolModel::list_for_organization(&ctx.db, id).await?;
    format::json(serde_json::json!({"data": pools}))
}

#[debug_handler]
pub async fn create_server_pool(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
    Json(params): Json<CreateServerPoolParams>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    principal
        .require(&ctx.db, id, Permission::ManageServers)
        .await?;
    let pool = ServerPoolModel::create(&ctx.db, id, &params).await?;

    let (actor_kind, actor_id) = principal.audit_actor();
    let _ = AuditEventModel::append(
        &ctx.db,
        AuditEventInput {
            organization_id: Some(id),
            actor_kind: actor_kind.to_string(),
            actor_id,
            action: "server_pool.create".to_string(),
            resource_type: Some("server_pool".to_string()),
            resource_id: Some(pool.id.to_string()),
            outcome: "success".to_string(),
            request_id: Some(principal.request_id.clone()),
            metadata: principal.audit_metadata(Some(serde_json::json!({
                "required_tags": pool.required_tags().unwrap_or_default(),
            }))),
        },
    )
    .await;

    format::json(serde_json::json!({"data": pool}))
}

#[debug_handler]
pub async fn upsert_server_pool_member(
    headers: HeaderMap,
    Path((id, pool_id)): Path<(i64, i64)>,
    State(ctx): State<AppContext>,
    Json(params): Json<SetServerPoolMemberParams>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    principal
        .require(&ctx.db, id, Permission::ManageServers)
        .await?;

    let pool = ServerPoolModel::find_by_id(&ctx.db, pool_id).await?;
    if pool.organization_id != id {
        return unauthorized("server pool belongs to another organization");
    }

    let server = ServerModel::find_by_id(&ctx.db, params.server_id).await?;
    if server.organization_id != Some(id) {
        return unauthorized("server belongs to another organization");
    }

    let member = ServerPoolMemberModel::upsert(&ctx.db, pool_id, &params).await?;
    format::json(serde_json::json!({"data": member}))
}

#[debug_handler]
pub async fn list_registry_credentials(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    principal.require(&ctx.db, id, Permission::View).await?;
    let credentials = RegistryCredentialModel::list_for_organization(&ctx.db, id)
        .await?
        .iter()
        .map(RegistryCredentialModel::to_safe)
        .collect::<Vec<_>>();
    format::json(serde_json::json!({"data": credentials}))
}

#[debug_handler]
pub async fn create_registry_credential(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
    Json(params): Json<CreateRegistryCredentialParams>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    principal
        .require(&ctx.db, id, Permission::ManageApplications)
        .await?;
    enforce_sensitive_action(&ctx, &principal, id, "registry_credential.create").await?;

    let credential = RegistryCredentialModel::create(&ctx.db, id, &params).await?;
    let (actor_kind, actor_id) = principal.audit_actor();
    let _ = AuditEventModel::append(
        &ctx.db,
        AuditEventInput {
            organization_id: Some(id),
            actor_kind: actor_kind.to_string(),
            actor_id,
            action: "registry_credential.create".to_string(),
            resource_type: Some("registry_credential".to_string()),
            resource_id: Some(credential.id.to_string()),
            outcome: "success".to_string(),
            request_id: Some(principal.request_id.clone()),
            metadata: principal.audit_metadata(Some(serde_json::json!({
                "registry": credential.registry,
                "username": credential.username,
            }))),
        },
    )
    .await;

    format::json(serde_json::json!({"data": credential.to_safe()}))
}

#[debug_handler]
pub async fn remove_registry_credential(
    headers: HeaderMap,
    Path((id, credential_id)): Path<(i64, i64)>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    principal
        .require(&ctx.db, id, Permission::ManageApplications)
        .await?;
    enforce_sensitive_action(&ctx, &principal, id, "registry_credential.delete").await?;

    let credential = RegistryCredentialModel::find_by_id(&ctx.db, credential_id).await?;
    if credential.organization_id != id {
        return unauthorized("registry credential belongs to another organization");
    }

    let in_use = applications::Entity::find()
        .filter(applications::Column::RegistryCredentialId.eq(credential_id))
        .one(&ctx.db)
        .await?
        .is_some();
    if in_use {
        return Err(Error::BadRequest(
            "registry credential is still referenced by an application".to_string(),
        ));
    }

    registry_credentials::Entity::delete_by_id(credential_id)
        .exec(&ctx.db)
        .await?;
    format::json(serde_json::json!({"data": null, "message": "Registry credential removed"}))
}
