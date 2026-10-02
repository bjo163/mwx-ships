use loco_rs::prelude::*;

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/health")
        .add("/", get(health_check))
}

#[debug_handler]
pub async fn health_check(State(_ctx): State<AppContext>) -> Result<Response> {
    format::json(serde_json::json!({
        "data": {
            "status": "healthy",
            "service": "moonships-control-plane",
            "database": "sqlite",
            "version": env!("CARGO_PKG_VERSION"),
        },
        "message": "ok"
    }))
}
