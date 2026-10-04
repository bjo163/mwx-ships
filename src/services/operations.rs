use crate::{
    models::{
        _entities::{deployments, servers},
        backup_runs, operational_events, server_health_checks,
        servers::Model as ServerModel,
    },
    services::{notification::NotificationService, ssh::SshService},
};
use chrono::{Duration, Utc};
use loco_rs::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize)]
pub struct DeploymentOutcomeMetrics {
    pub queued: u64,
    pub active: u64,
    pub stale_leases: u64,
    pub success_24h: u64,
    pub failed_24h: u64,
    pub cancelled_24h: u64,
    pub average_duration_seconds_24h: Option<f64>,
    pub recent_failure_codes: Vec<(String, u64)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServerMetrics {
    pub total: u64,
    pub online: u64,
    pub offline: u64,
    pub error: u64,
    pub min_recent_disk_available_gb: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BackupMetrics {
    pub latest_status: Option<String>,
    pub latest_verified: Option<bool>,
    pub latest_completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OperationsMetrics {
    pub generated_at: String,
    pub database: &'static str,
    pub deployments: DeploymentOutcomeMetrics,
    pub servers: ServerMetrics,
    pub backup: BackupMetrics,
}

#[derive(Debug, Clone, Serialize)]
pub struct ControlPlaneHealth {
    pub status: &'static str,
    pub database_ok: bool,
    pub queue_ok: bool,
    pub targets_ok: bool,
    pub backup_ok: bool,
    pub issues: Vec<String>,
    pub metrics: OperationsMetrics,
}

pub struct OperationsService;

impl OperationsService {
    pub async fn metrics(db: &DatabaseConnection) -> Result<OperationsMetrics> {
        let queued = deployments::Entity::find()
            .filter(deployments::Column::Status.eq("queued"))
            .count(db)
            .await?;

        let active = deployments::Entity::find()
            .filter(
                deployments::Column::Status.is_in(
                    crate::models::deployments::ACTIVE_STATUSES
                        .iter()
                        .map(|value| value.to_string()),
                ),
            )
            .count(db)
            .await?;

        let now = Utc::now();
        let stale_leases = deployments::Entity::find()
            .filter(
                deployments::Column::Status.is_in(
                    crate::models::deployments::ACTIVE_STATUSES
                        .iter()
                        .map(|value| value.to_string()),
                ),
            )
            .filter(deployments::Column::LeaseExpiresAt.lt(now))
            .count(db)
            .await?;

        let cutoff = now - Duration::hours(24);
        let recent = deployments::Entity::find()
            .filter(deployments::Column::QueuedAt.gte(cutoff))
            .order_by_desc(deployments::Column::QueuedAt)
            .limit(500)
            .all(db)
            .await?;

        let mut success_24h = 0;
        let mut failed_24h = 0;
        let mut cancelled_24h = 0;
        let mut durations = Vec::new();
        let mut failure_codes = BTreeMap::<String, u64>::new();

        for deployment in &recent {
            match deployment.status.as_str() {
                "success" => success_24h += 1,
                "failed" => {
                    failed_24h += 1;
                    if let Some(code) = deployment.error_code.as_deref() {
                        *failure_codes.entry(code.to_string()).or_default() += 1;
                    }
                }
                "cancelled" => cancelled_24h += 1,
                _ => {}
            }

            if let (Some(started), Some(finished)) = (
                deployment.started_at.as_ref(),
                deployment.finished_at.as_ref(),
            ) {
                let seconds = finished.timestamp() - started.timestamp();
                if seconds >= 0 {
                    durations.push(seconds as f64);
                }
            }
        }

        let average_duration_seconds_24h = if durations.is_empty() {
            None
        } else {
            Some(durations.iter().sum::<f64>() / durations.len() as f64)
        };

        let mut recent_failure_codes = failure_codes.into_iter().collect::<Vec<_>>();
        recent_failure_codes.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        recent_failure_codes.truncate(10);

        let all_servers = servers::Entity::find().all(db).await?;
        let mut online = 0;
        let mut offline = 0;
        let mut error = 0;
        for server in &all_servers {
            match server.status.as_str() {
                "online" => online += 1,
                "offline" => offline += 1,
                "error" => error += 1,
                _ => {}
            }
        }

        let recent_health = server_health_checks::Model::recent(db, 100).await?;
        let min_recent_disk_available_gb = recent_health
            .iter()
            .filter_map(|sample| sample.disk_available_gb)
            .reduce(f64::min);

        let latest_backup = backup_runs::Model::recent(db, 1).await?.into_iter().next();

        Ok(OperationsMetrics {
            generated_at: now.to_rfc3339(),
            database: "sqlite",
            deployments: DeploymentOutcomeMetrics {
                queued,
                active,
                stale_leases,
                success_24h,
                failed_24h,
                cancelled_24h,
                average_duration_seconds_24h,
                recent_failure_codes,
            },
            servers: ServerMetrics {
                total: all_servers.len() as u64,
                online,
                offline,
                error,
                min_recent_disk_available_gb,
            },
            backup: BackupMetrics {
                latest_status: latest_backup.as_ref().map(|run| run.status.clone()),
                latest_verified: latest_backup.as_ref().map(|run| run.verified),
                latest_completed_at: latest_backup
                    .as_ref()
                    .and_then(|run| run.completed_at.as_ref())
                    .map(ToString::to_string),
            },
        })
    }

    pub async fn health(db: &DatabaseConnection) -> Result<ControlPlaneHealth> {
        let metrics = Self::metrics(db).await?;
        let mut issues = Vec::new();

        let queue_ok = metrics.deployments.stale_leases == 0;
        if !queue_ok {
            issues.push(format!(
                "{} deployment execution lease(s) are stale",
                metrics.deployments.stale_leases
            ));
        }

        let targets_ok = metrics.servers.offline == 0 && metrics.servers.error == 0;
        if !targets_ok {
            issues.push(format!(
                "{} offline and {} error target(s)",
                metrics.servers.offline, metrics.servers.error
            ));
        }

        let backup_ok = match metrics.backup.latest_status.as_deref() {
            Some("success") => metrics.backup.latest_verified.unwrap_or(false),
            Some(_) => false,
            None => true,
        };
        if !backup_ok {
            issues.push("latest backup is not verified healthy".to_string());
        }

        Ok(ControlPlaneHealth {
            status: if issues.is_empty() {
                "healthy"
            } else {
                "degraded"
            },
            database_ok: true,
            queue_ok,
            targets_ok,
            backup_ok,
            issues,
            metrics,
        })
    }

    pub async fn poll_targets(ctx: &AppContext) -> Result<()> {
        let targets = servers::Entity::find().all(&ctx.db).await?;
        let disk_warning_gb = std::env::var("MOONSHIPS_DISK_WARNING_GB")
            .ok()
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or(2.0)
            .max(0.1);

        for target in targets {
            let previous_status = target.status.clone();
            match SshService::run_preflight(&target).await {
                Ok(report) => {
                    let status = if report.healthy {
                        "online"
                    } else if report.ssh_connected {
                        "error"
                    } else {
                        "offline"
                    };

                    let _ = ServerModel::update_status(&ctx.db, target.id, status).await;
                    let _ = server_health_checks::Model::record(&ctx.db, &report).await;

                    if previous_status != status || !report.healthy {
                        let severity = if report.healthy { "info" } else { "warning" };
                        let _ = operational_events::Model::record(
                            &ctx.db,
                            "target_health",
                            severity,
                            Some("server"),
                            Some(target.id),
                            &format!("Target '{}' is {}", target.name, status),
                            Some(&serde_json::json!({
                                "ssh_connected": report.ssh_connected,
                                "docker_running": report.docker_running,
                                "disk_available_gb": report.disk_available_gb,
                                "issues": report.issues,
                            })),
                        )
                        .await;
                    }

                    if !report.healthy {
                        let _ = NotificationService::notify(
                            ctx,
                            "target_unhealthy",
                            "warning",
                            &format!("Target {} unhealthy", target.name),
                            &format!(
                                "Target {} ({}@{}:{}) status={} issues={}",
                                target.name,
                                target.username,
                                target.host,
                                target.port,
                                status,
                                report.issues.join("; ")
                            ),
                        )
                        .await;
                    }

                    if let Some(gb) = report.disk_available_gb {
                        if gb < disk_warning_gb {
                            let _ = NotificationService::notify(
                                ctx,
                                "disk_pressure",
                                "warning",
                                &format!("Disk pressure on {}", target.name),
                                &format!(
                                    "Target {} has {:.1} GB available; threshold is {:.1} GB",
                                    target.name, gb, disk_warning_gb
                                ),
                            )
                            .await;
                        }
                    }
                }
                Err(error) => {
                    let _ = ServerModel::update_status(&ctx.db, target.id, "offline").await;
                    let _ = operational_events::Model::record(
                        &ctx.db,
                        "target_health",
                        "warning",
                        Some("server"),
                        Some(target.id),
                        &format!("Target '{}' preflight failed", target.name),
                        Some(&serde_json::json!({"error": error.to_string()})),
                    )
                    .await;
                    let _ = NotificationService::notify(
                        ctx,
                        "target_unhealthy",
                        "warning",
                        &format!("Target {} unreachable", target.name),
                        &error.to_string(),
                    )
                    .await;
                }
            }
        }

        let health = Self::health(&ctx.db).await?;
        if health.metrics.deployments.stale_leases > 0 {
            let _ = NotificationService::notify(
                ctx,
                "stale_deployment_lease",
                "critical",
                "Stale deployment execution lease",
                &format!(
                    "{} active deployment lease(s) are stale and eligible for recovery",
                    health.metrics.deployments.stale_leases
                ),
            )
            .await;
        }

        Ok(())
    }

    pub async fn recent_events(
        db: &DatabaseConnection,
        limit: u64,
    ) -> Result<Vec<operational_events::Model>> {
        operational_events::Model::recent(db, limit).await
    }
}
