use chrono::{DateTime, FixedOffset, Utc};
use loco_rs::prelude::*;
use sea_orm::{ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};

pub use super::_entities::deployment_logs::{self, ActiveModel, Entity, Model};

#[derive(Debug, Deserialize, Serialize)]
pub struct LogEntry {
    pub sequence: i64,
    pub stream: String,
    pub message: String,
    pub timestamp: String,
}

impl Model {
    pub async fn by_deployment(db: &DatabaseConnection, deployment_id: i64) -> Result<Vec<Model>> {
        let logs = Entity::find()
            .filter(deployment_logs::Column::DeploymentId.eq(deployment_id))
            .order_by_asc(deployment_logs::Column::Sequence)
            .all(db)
            .await?;
        Ok(logs)
    }

    pub async fn next_sequence(db: &DatabaseConnection, deployment_id: i64) -> Result<i64> {
        let count = Entity::find()
            .filter(deployment_logs::Column::DeploymentId.eq(deployment_id))
            .count(db)
            .await?;
        Ok(count as i64 + 1)
    }

    pub async fn delete_older_than(
        db: &DatabaseConnection,
        cutoff: DateTime<FixedOffset>,
    ) -> Result<u64> {
        Ok(Entity::delete_many()
            .filter(deployment_logs::Column::CreatedAt.lt(cutoff))
            .exec(db)
            .await?
            .rows_affected)
    }

    pub async fn append(
        db: &DatabaseConnection,
        deployment_id: i64,
        stream: &str,
        message: &str,
    ) -> Result<Model> {
        let seq = Self::next_sequence(db, deployment_id).await?;
        let now = Utc::now();

        let active = ActiveModel {
            deployment_id: Set(deployment_id),
            sequence: Set(seq),
            stream: Set(stream.to_string()),
            message: Set(message.to_string()),
            created_at: Set(now.into()),
            ..Default::default()
        };

        let model = active.insert(db).await?;
        Ok(model)
    }
}
