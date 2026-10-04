use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, QueryOrder};
use serde_json::Value;

pub use super::_entities::operational_events::{self, ActiveModel, Entity, Model};

impl Model {
    #[allow(clippy::too_many_arguments)]
    pub async fn record(
        db: &DatabaseConnection,
        event_kind: &str,
        severity: &str,
        resource_type: Option<&str>,
        resource_id: Option<i64>,
        message: &str,
        metadata: Option<&Value>,
    ) -> Result<Model> {
        let active = ActiveModel {
            event_kind: Set(event_kind.to_string()),
            severity: Set(severity.to_string()),
            resource_type: Set(resource_type.map(str::to_string)),
            resource_id: Set(resource_id),
            message: Set(message.to_string()),
            metadata_json: Set(metadata.map(Value::to_string)),
            created_at: Set(Utc::now().into()),
            ..Default::default()
        };
        Ok(active.insert(db).await?)
    }

    pub async fn recent(db: &DatabaseConnection, limit: u64) -> Result<Vec<Model>> {
        Ok(Entity::find()
            .order_by_desc(operational_events::Column::CreatedAt)
            .limit(limit.min(200))
            .all(db)
            .await?)
    }
}
