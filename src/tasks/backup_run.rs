use crate::services::backup::BackupService;
use loco_rs::prelude::*;

pub struct BackupRun;

#[async_trait]
impl Task for BackupRun {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "backup:run".to_string(),
            detail: "Create, verify, hash, optionally encrypt, and retain a Moonships SQLite backup.".to_string(),
        }
    }

    async fn run(&self, app_context: &AppContext, _vars: &task::Vars) -> Result<()> {
        let result = BackupService::run(&app_context.db)
            .await
            .map_err(|err| Error::string(&err.to_string()))?;
        println!("Backup verified: {}", result.path.display());
        println!("SHA256: {}", result.run.sha256.unwrap_or_default());
        Ok(())
    }
}
