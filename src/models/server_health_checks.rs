use chrono::{Duration, Utc};
use loco_rs::prelude::*;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
};
use serde::{Deserialize, Serialize};

use crate::services::ssh::PreflightReport;

pub use super::_entities::server_health_checks::{self, ActiveModel, Entity, Model};

impl Model {
    pub async fn record(db: &DatabaseConnection, report: &PreflightReport) -> Result<Model> {
        let now = Utc::now();
        let active = ActiveModel {
            server_id: Set(report.server_id),
            ssh_connected: Set(report.ssh_connected),
            docker_running: Set(report.docker_running),
            disk_available_gb: Set(report.disk_available_gb),
            memory_available_mb: Set(report.memory_available_mb.map(|value| value as i64)),
            cpu_cores: Set(report.cpu_cores.map(|value| value as i32)),
            healthy: Set(report.healthy),
            issues_json: Set(serde_json::to_string(&report.issues)?),
            created_at: Set(now.into()),
            ..Default::default()
        };
        Ok(active.insert(db).await?)
    }

    pub async fn recent(db: &DatabaseConnection, limit: u64) -> Result<Vec<Model>> {
        Ok(Entity::find()
            .order_by_desc(server_health_checks::Column::CreatedAt)
            .limit(limit.min(200))
            .all(db)
            .await?)
    }

    pub async fn recent_for_server(
        db: &DatabaseConnection,
        server_id: i64,
        hours: i64,
    ) -> Result<Vec<Model>> {
        let cutoff = Utc::now() - Duration::hours(hours.max(1));
        Ok(Entity::find()
            .filter(server_health_checks::Column::ServerId.eq(server_id))
            .filter(server_health_checks::Column::CreatedAt.gte(cutoff))
            .order_by_desc(server_health_checks::Column::CreatedAt)
            .limit(200)
            .all(db)
            .await?)
    }
}
