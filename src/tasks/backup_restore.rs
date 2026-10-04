use crate::services::backup::BackupService;
use loco_rs::prelude::*;
use std::path::Path;

pub struct BackupRestore;

#[async_trait]
impl Task for BackupRestore {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "backup:restore".to_string(),
            detail: "Restore a verified backup into a target SQLite path. The Moonships service must be stopped. Usage: cargo loco task backup:restore path:/backup target:/data/moonships.sqlite confirm:RESTORE".to_string(),
        }
    }

    async fn run(&self, _app_context: &AppContext, vars: &task::Vars) -> Result<()> {
        let path = vars
            .cli_arg("path")
            .map_err(|_| Error::string("path is mandatory"))?;
        let target = vars
            .cli_arg("target")
            .map_err(|_| Error::string("target is mandatory"))?;
        let confirm = vars
            .cli_arg("confirm")
            .map_err(|_| Error::string("confirm:RESTORE is mandatory"))?;
        if confirm != "RESTORE" {
            return Err(Error::string("restore confirmation must equal RESTORE"));
        }

        let active = std::env::var("DATABASE_URL").unwrap_or_default();
        if active.contains(target) {
            return Err(Error::string(
                "refusing to overwrite the configured live DATABASE_URL from an online task; stop Moonships and restore to a staging path first",
            ));
        }

        let result = BackupService::restore_to_path(Path::new(path), Path::new(target))
            .await
            .map_err(|err| Error::string(&err.to_string()))?;
        println!("Restore completed and verified: {result}");
        Ok(())
    }
}
