use crate::models::{
    _entities::projects::{ActiveModel, Entity},
    environments::{CreateEnvironmentParams, Model as EnvironmentModel},
    projects::{CreateProjectParams, Model as ProjectModel, UpdateProjectParams},
};
use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{ActiveValue::Set, EntityTrait};

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
pub async fn list(State(ctx): State<AppContext>) -> Result<Response> {
    let projects = ProjectModel::all(&ctx.db).await?;
    format::json(serde_json::json!({
        "data": projects,
        "message": "ok"
    }))
}

#[debug_handler]
pub async fn create(
    State(ctx): State<AppContext>,
    Json(params): Json<CreateProjectParams>,
) -> Result<Response> {
    let project = ProjectModel::create_project(&ctx.db, &params).await?;
    format::json(serde_json::json!({
        "data": project,
        "message": "ok"
    }))
}

#[debug_handler]
pub async fn get_one(Path(id): Path<i64>, State(ctx): State<AppContext>) -> Result<Response> {
    let project = ProjectModel::find_by_id(&ctx.db, id).await?;
    format::json(serde_json::json!({
        "data": project,
        "message": "ok"
    }))
}

#[debug_handler]
pub async fn update(
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
    Json(params): Json<UpdateProjectParams>,
) -> Result<Response> {
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
    format::json(serde_json::json!({
        "data": updated,
        "message": "ok"
    }))
}

#[debug_handler]
pub async fn remove(Path(id): Path<i64>, State(ctx): State<AppContext>) -> Result<Response> {
    let project = ProjectModel::find_by_id(&ctx.db, id).await?;
    Entity::delete_by_id(project.id).exec(&ctx.db).await?;
    format::json(serde_json::json!({
        "data": null,
        "message": "ok"
    }))
}

#[debug_handler]
pub async fn list_environments(
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let envs = EnvironmentModel::by_project(&ctx.db, id).await?;
    format::json(serde_json::json!({
        "data": envs,
        "message": "ok"
    }))
}

#[derive(Debug, serde::Deserialize)]
pub struct SubCreateEnvParams {
    pub name: String,
    pub slug: Option<String>,
    pub description: Option<String>,
}

#[debug_handler]
pub async fn create_environment(
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
    Json(params): Json<SubCreateEnvParams>,
) -> Result<Response> {
    let create_params = CreateEnvironmentParams {
        project_id: id,
        name: params.name,
        slug: params.slug,
        description: params.description,
    };
    let env = EnvironmentModel::create_environment(&ctx.db, &create_params).await?;
    format::json(serde_json::json!({
        "data": env,
        "message": "ok"
    }))
}
