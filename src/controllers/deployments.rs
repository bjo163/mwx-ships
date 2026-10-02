use crate::models::{
    deployment_logs::Model as DeploymentLogModel, deployments::Model as DeploymentModel,
};
use loco_rs::prelude::*;

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/deployments")
        .add("{id}", get(get_one))
        .add("{id}/logs", get(get_logs))
}

#[debug_handler]
pub async fn get_one(Path(id): Path<i64>, State(ctx): State<AppContext>) -> Result<Response> {
    let dep = DeploymentModel::find_by_id(&ctx.db, id).await?;
    format::json(serde_json::json!({
        "data": dep,
        "message": "ok"
    }))
}

#[debug_handler]
pub async fn get_logs(Path(id): Path<i64>, State(ctx): State<AppContext>) -> Result<Response> {
    let logs = DeploymentLogModel::by_deployment(&ctx.db, id).await?;
    format::json(serde_json::json!({
        "data": logs,
        "message": "ok"
    }))
}
