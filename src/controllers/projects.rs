use crate::{
    models::{
        _entities::projects::{ActiveModel, Entity},
        audit_events::{AuditEventInput, Model as AuditEventModel},
        environments::{CreateEnvironmentParams, Model as EnvironmentModel},
        projects::{CreateProjectParams, Model as ProjectModel, UpdateProjectParams},
    },
    services::access_control::{Permission, Principal},
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
        .prefix("api/projects")
        .add("/", get(list))
        .add("/", post(create))
        .add("{id}", get(get_one))
        .add("{id}", put(update))
        .add("{id}", delete(remove))
        .add("{id}/environments", get(list_environments))
        .add("{id}/environments", post(create_environment))
}

#[debug_handler]
pub async fn list(headers: HeaderMap, State(ctx): State<AppContext>) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    let organization_ids = principal.organization_ids(&ctx.db).await?;
    let projects = ProjectModel::all_for_organizations(&ctx.db, &organization_ids).await?;
    format::json(serde_json::json!({"data": projects, "message": "ok"}))
}

#[debug_handler]
pub async fn create(
    headers: HeaderMap,
    Query(query): Query<OrganizationQuery>,
    State(ctx): State<AppContext>,
    Json(params): Json<CreateProjectParams>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    let organization_id = principal
        .resolve_organization(&ctx.db, query.organization_id)
        .await?;
    principal
        .require(&ctx.db, organization_id, Permission::ManageProjects)
        .await?;

    let project =
        ProjectModel::create_project_for_organization(&ctx.db, &params, Some(organization_id))
            .await?;
    audit(
        &ctx,
        &principal,
        organization_id,
        "project.create",
        Some(project.id),
        None,
    )
    .await;

    format::json(serde_json::json!({"data": project, "message": "ok"}))
}

#[debug_handler]
pub async fn get_one(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    principal
        .project_organization(&ctx.db, id, Permission::View)
        .await?;
    let project = ProjectModel::find_by_id(&ctx.db, id).await?;
    format::json(serde_json::json!({"data": project, "message": "ok"}))
}

#[debug_handler]
pub async fn update(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
    Json(params): Json<UpdateProjectParams>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    let organization_id = principal
        .project_organization(&ctx.db, id, Permission::ManageProjects)
        .await?;

    let project = ProjectModel::find_by_id(&ctx.db, id).await?;
    let mut active: ActiveModel = project.into();

    if let Some(name) = params.name {
        active.name = Set(name);
    }
    if let Some(desc) = params.description {
        active.description = Set(Some(desc));
    }
    active.updated_at = Set(Utc::now().into());

    let updated = active.update(&ctx.db).await?;
    audit(
        &ctx,
        &principal,
        organization_id,
        "project.update",
        Some(id),
        None,
    )
    .await;

    format::json(serde_json::json!({"data": updated, "message": "ok"}))
}

#[debug_handler]
pub async fn remove(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    let organization_id = principal
        .project_organization(&ctx.db, id, Permission::ManageProjects)
        .await?;
    let project = ProjectModel::find_by_id(&ctx.db, id).await?;
    Entity::delete_by_id(project.id).exec(&ctx.db).await?;

    audit(
        &ctx,
        &principal,
        organization_id,
        "project.delete",
        Some(id),
        None,
    )
    .await;

    format::json(serde_json::json!({"data": null, "message": "ok"}))
}

#[debug_handler]
pub async fn list_environments(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    principal
        .project_organization(&ctx.db, id, Permission::View)
        .await?;
    let envs = EnvironmentModel::by_project(&ctx.db, id).await?;
    format::json(serde_json::json!({"data": envs, "message": "ok"}))
}

#[derive(Debug, Deserialize)]
pub struct SubCreateEnvParams {
    pub name: String,
    pub slug: Option<String>,
    pub description: Option<String>,
}

#[debug_handler]
pub async fn create_environment(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
    Json(params): Json<SubCreateEnvParams>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    let organization_id = principal
        .project_organization(&ctx.db, id, Permission::ManageProjects)
        .await?;

    let create_params = CreateEnvironmentParams {
        project_id: id,
        name: params.name,
        slug: params.slug,
        description: params.description,
    };
    let env = EnvironmentModel::create_environment(&ctx.db, &create_params).await?;

    audit(
        &ctx,
        &principal,
        organization_id,
        "environment.create",
        Some(env.id),
        Some(serde_json::json!({"project_id": id})),
    )
    .await;

    format::json(serde_json::json!({"data": env, "message": "ok"}))
}

async fn audit(
    ctx: &AppContext,
    principal: &Principal,
    organization_id: i64,
    action: &str,
    resource_id: Option<i64>,
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
            resource_type: Some("project".to_string()),
            resource_id: resource_id.map(|id| id.to_string()),
            outcome: "success".to_string(),
            request_id: Some(principal.request_id.clone()),
            metadata: principal.audit_metadata(metadata),
        },
    )
    .await;
}
