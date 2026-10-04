use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
};
use serde::{Deserialize, Serialize};

pub use super::_entities::audit_events::{self, ActiveModel, Entity, Model};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuditEventInput {
    pub organization_id: Option<i64>,
    pub actor_kind: String,
    pub actor_id: String,
    pub action: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub outcome: String,
    pub request_id: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

impl Model {
    pub async fn append(db: &DatabaseConnection, input: AuditEventInput) -> Result<Model> {
        Ok(ActiveModel {
            organization_id: Set(input.organization_id),
            actor_kind: Set(input.actor_kind),
            actor_id: Set(input.actor_id),
            action: Set(input.action),
            resource_type: Set(input.resource_type),
            resource_id: Set(input.resource_id),
            outcome: Set(input.outcome),
            request_id: Set(input.request_id),
            metadata_json: Set(input.metadata.map(|value| value.to_string())),
            created_at: Set(Utc::now().into()),
            ..Default::default()
        }
        .insert(db)
        .await?)
    }

    pub async fn list_for_organization(
        db: &DatabaseConnection,
        organization_id: i64,
        limit: u64,
    ) -> Result<Vec<Model>> {
        Ok(Entity::find()
            .filter(audit_events::Column::OrganizationId.eq(organization_id))
            .order_by_desc(audit_events::Column::CreatedAt)
            .limit(limit.clamp(1, 500))
            .all(db)
            .await?)
    }
}
