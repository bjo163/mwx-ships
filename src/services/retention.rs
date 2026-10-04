use crate::{
    models::{applications, deployment_logs, deployment_revisions, servers},
    services::remote::RemoteRuntime,
};
use chrono::{Duration, Utc};
use loco_rs::prelude::*;
use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct RetentionPolicy {
    pub revision_artifacts: usize,
    pub log_days: i64,
    pub disk_warning_gb: f64,
}

impl RetentionPolicy {
    pub fn from_env() -> Self {
        let revision_artifacts = std::env::var("MOONSHIPS_REVISION_ARTIFACT_RETENTION")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(10)
            .max(2);

        let log_days = std::env::var("MOONSHIPS_DEPLOYMENT_LOG_RETENTION_DAYS")
            .ok()
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or(30)
            .max(1);

        let disk_warning_gb = std::env::var("MOONSHIPS_DISK_WARNING_GB")
            .ok()
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or(2.0)
            .max(0.1);

        Self {
            revision_artifacts,
            log_days,
            disk_warning_gb,
        }
    }
}

#[derive(Debug, Default, Serialize)]
pub struct RetentionReport {
    pub revision_artifacts_considered: usize,
    pub images_pruned: usize,
    pub logs_pruned: u64,
    pub disk_available_gb: Option<f64>,
    pub warnings: Vec<String>,
}

pub struct RetentionService;

impl RetentionService {
    pub async fn cleanup_after_success(
        db: &DatabaseConnection,
        application_id: i64,
    ) -> Result<RetentionReport> {
        let policy = RetentionPolicy::from_env();
        let app = applications::Model::find_by_id(db, application_id).await?;
        let revisions = deployment_revisions::Model::by_application(db, application_id).await?;

        let protected: HashSet<i64> = [app.current_revision_id, app.previous_revision_id]
            .into_iter()
            .flatten()
            .collect();

        let candidates =
            select_artifact_cleanup_candidates(&revisions, &protected, policy.revision_artifacts);

        let mut report = RetentionReport {
            revision_artifacts_considered: candidates.len(),
            ..Default::default()
        };

        for revision in candidates {
            if !revision.image_reference.starts_with("moonships/") {
                continue;
            }

            let server = match servers::Model::find_by_id(db, revision.server_id).await {
                Ok(server) => server,
                Err(err) => {
                    report.warnings.push(format!(
                        "retention: server #{} unavailable for revision #{}: {err}",
                        revision.server_id, revision.id
                    ));
                    continue;
                }
            };

            match RemoteRuntime::connect(&server).await {
                Ok(runtime) => match runtime.remove_image(&revision.image_reference).await {
                    Ok(()) => report.images_pruned += 1,
                    Err(err) => report.warnings.push(format!(
                        "retention: failed pruning image '{}' on server #{}: {err}",
                        revision.image_reference, revision.server_id
                    )),
                },
                Err(err) => report.warnings.push(format!(
                    "retention: failed connecting to server #{} for image cleanup: {err}",
                    revision.server_id
                )),
            }
        }

        let cutoff = (Utc::now() - Duration::days(policy.log_days)).fixed_offset();
        report.logs_pruned = deployment_logs::Model::delete_older_than(db, cutoff).await?;

        match servers::Model::find_by_id(db, app.server_id).await {
            Ok(server) => match RemoteRuntime::connect(&server).await {
                Ok(runtime) => match runtime.disk_available_gb().await {
                    Ok(gb) => {
                        report.disk_available_gb = Some(gb);
                        if gb < policy.disk_warning_gb {
                            report.warnings.push(format!(
                                "target disk pressure: {gb:.1} GB available, warning threshold {:.1} GB",
                                policy.disk_warning_gb
                            ));
                        }
                    }
                    Err(err) => report
                        .warnings
                        .push(format!("retention: disk check failed: {err}")),
                },
                Err(err) => report
                    .warnings
                    .push(format!("retention: target connection for disk check failed: {err}")),
            },
            Err(err) => report
                .warnings
                .push(format!("retention: current target lookup failed: {err}")),
        }

        Ok(report)
    }
}

pub fn select_artifact_cleanup_candidates(
    revisions: &[deployment_revisions::Model],
    protected: &HashSet<i64>,
    keep_count: usize,
) -> Vec<deployment_revisions::Model> {
    revisions
        .iter()
        .enumerate()
        .filter(|(index, revision)| *index >= keep_count && !protected.contains(&revision.id))
        .map(|(_, revision)| revision.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn revision(id: i64) -> deployment_revisions::Model {
        let now = Utc::now().fixed_offset();
        deployment_revisions::Model {
            id,
            application_id: 1,
            server_id: 1,
            source_commit_hash: format!("commit{id}"),
            source_commit_message: None,
            image_reference: format!("moonships/app:rev-{id}"),
            revision_hash: format!("hash{id}"),
            runtime_snapshot: "{}".to_string(),
            status: "healthy".to_string(),
            healthy_at: Some(now),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn cleanup_selection_never_prunes_protected_revisions() {
        let revisions = (1..=6).rev().map(revision).collect::<Vec<_>>();
        let protected = HashSet::from([1, 3]);
        let candidates = select_artifact_cleanup_candidates(&revisions, &protected, 2);
        let ids = candidates.iter().map(|revision| revision.id).collect::<Vec<_>>();

        assert_eq!(ids, vec![4, 2]);
        assert!(!ids.contains(&1));
        assert!(!ids.contains(&3));
    }
}
