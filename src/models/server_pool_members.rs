use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};

pub use super::_entities::server_pool_members::{self, ActiveModel, Entity, Model};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SetServerPoolMemberParams {
    pub server_id: i64,
    pub weight: Option<i32>,
}

impl Model {
    pub async fn list_for_pool(db: &DatabaseConnection, server_pool_id: i64) -> Result<Vec<Model>> {
        Ok(Entity::find()
            .filter(server_pool_members::Column::ServerPoolId.eq(server_pool_id))
            .order_by_desc(server_pool_members::Column::Weight)
            .order_by_asc(server_pool_members::Column::ServerId)
            .all(db)
            .await?)
    }

    pub async fn upsert(
        db: &DatabaseConnection,
        server_pool_id: i64,
        params: &SetServerPoolMemberParams,
    ) -> Result<Model> {
        let weight = params.weight.unwrap_or(100).clamp(1, 1000);
        if let Some(existing) = Entity::find()
            .filter(server_pool_members::Column::ServerPoolId.eq(server_pool_id))
            .filter(server_pool_members::Column::ServerId.eq(params.server_id))
            .one(db)
            .await?
        {
            let mut active: ActiveModel = existing.into();
            active.weight = Set(weight);
            return Ok(active.update(db).await?);
        }

        Ok(ActiveModel {
            server_pool_id: Set(server_pool_id),
            server_id: Set(params.server_id),
            weight: Set(weight),
            created_at: Set(Utc::now().into()),
            ..Default::default()
        }
        .insert(db)
        .await?)
    }
}
