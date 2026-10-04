use crate::services::{backup::BackupService, notification::NotificationService};
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

pub struct BackupWorker {
    pub ctx: AppContext,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct BackupWorkerArgs {
    pub reason: String,
}

#[async_trait]
impl BackgroundWorker<BackupWorkerArgs> for BackupWorker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    async fn perform(&self, args: BackupWorkerArgs) -> Result<()> {
        match BackupService::run(&self.ctx.db).await {
            Ok(result) => {
                tracing::info!(
                    backup_id = result.run.id,
                    path = %result.path.display(),
                    reason = %args.reason,
                    "Verified Moonships backup completed"
                );
                Ok(())
            }
            Err(crate::services::backup::BackupError::Busy) => {
                tracing::info!(reason = %args.reason, "Backup request skipped because another backup owns the lock");
                Ok(())
            }
            Err(error) => {
                tracing::error!(error = %error, reason = %args.reason, "Moonships backup failed");
                let _ = NotificationService::notify(
                    &self.ctx,
                    "backup_failed",
                    "critical",
                    "Moonships backup failed",
                    &error.to_string(),
                )
                .await;
                Ok(())
            }
        }
    }
}
