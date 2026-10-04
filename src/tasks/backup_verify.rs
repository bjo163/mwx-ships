use crate::services::backup::BackupService;
use loco_rs::prelude::*;
use std::path::Path;

pub struct BackupVerify;

#[async_trait]
impl Task for BackupVerify {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "backup:verify".to_string(),
            detail: "Verify a plain or encrypted Moonships backup. Usage: cargo loco task backup:verify path:/backups/file".to_string(),
        }
    }

    async fn run(&self, _app_context: &AppContext, vars: &task::Vars) -> Result<()> {
        let path = vars
            .cli_arg("path")
            .map_err(|_| Error::string("path is mandatory"))?;
        let result = BackupService::verify_export(Path::new(path))
            .await
            .map_err(|err| Error::string(&err.to_string()))?;
        println!("Backup verification passed: {result}");
        Ok(())
    }
}
