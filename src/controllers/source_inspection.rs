use crate::{
    models::{
        audit_events::{AuditEventInput, Model as AuditEventModel},
        servers::Model as ServerModel,
    },
    services::{
        access_control::{Permission, Principal},
        remote::{RemoteError, RemoteRuntime},
        source_inspection::{SourceInspectionRequest, SourceInspectionService},
    },
};
use axum::http::HeaderMap;
use loco_rs::prelude::*;

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/source-inspection")
        .add("/", post(inspect))
}

#[debug_handler]
pub async fn inspect(
    headers: HeaderMap,
    State(ctx): State<AppContext>,
    Json(params): Json<SourceInspectionRequest>,
) -> Result<Response> {
    let principal = Principal::authenticate(&ctx, &headers).await?;
    let project_org = principal
        .project_organization(&ctx.db, params.project_id, Permission::ManageApplications)
        .await?;
    let server_org = principal
        .server_organization(&ctx.db, params.server_id, Permission::ManageApplications)
        .await?;

    if project_org != server_org {
        return Err(Error::BadRequest(
            "source inspection project and target server must belong to the same organization"
                .to_string(),
        ));
    }

    let server = ServerModel::find_by_id(&ctx.db, params.server_id).await?;
    let runtime = RemoteRuntime::connect(&server)
        .await
        .map_err(source_inspection_error)?;
    let branch = params.branch().to_string();
    let snapshot = runtime
        .inspect_source_repository(&params.git_repository, &branch)
        .await
        .map_err(source_inspection_error)?;
    let inspection = SourceInspectionService::inspect(&snapshot);

    let (actor_kind, actor_id) = principal.audit_actor();
    let _ = AuditEventModel::append(
        &ctx.db,
        AuditEventInput {
            organization_id: Some(project_org),
            actor_kind: actor_kind.to_string(),
            actor_id,
            action: "source.inspect".to_string(),
            resource_type: Some("project".to_string()),
            resource_id: Some(params.project_id.to_string()),
            outcome: "success".to_string(),
            request_id: Some(principal.request_id.clone()),
            metadata: principal.audit_metadata(Some(serde_json::json!({
                "server_id": params.server_id,
                "branch": branch,
                "commit_sha": inspection.commit_sha.clone(),
                "strategy": inspection.strategy,
                "confidence": inspection.confidence,
            }))),
        },
    )
    .await;

    format::json(serde_json::json!({
        "data": inspection,
        "message": "ok"
    }))
}

fn source_inspection_error(error: RemoteError) -> Error {
    match error {
        RemoteError::Validation(message) => Error::BadRequest(message),
        RemoteError::Ssh(_) | RemoteError::CommandFailed { .. } => {
            Error::BadRequest("source inspection failed on the selected target server".to_string())
        }
    }
}
