use crate::services::deployment::DeploymentService;
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

pub struct DeploymentWorker {
    pub ctx: AppContext,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct DeploymentWorkerArgs {
    pub deployment_id: i64,
}

#[async_trait]
impl BackgroundWorker<DeploymentWorkerArgs> for DeploymentWorker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    async fn perform(&self, args: DeploymentWorkerArgs) -> Result<()> {
        tracing::info!(
            deployment_id = args.deployment_id,
            "DeploymentWorker starting asynchronous deployment execution"
        );

        match DeploymentService::execute_deployment(&self.ctx.db, args.deployment_id).await {
            Ok(()) => {
                tracing::info!(
                    deployment_id = args.deployment_id,
                    "DeploymentWorker completed deployment successfully"
                );
                Ok(())
            }
            Err(err) => {
                tracing::error!(
                    deployment_id = args.deployment_id,
                    error = %err,
                    "DeploymentWorker execution encountered error"
                );
                // Return Ok so that queue marks the job handled, as failure state is recorded in DB
                Ok(())
            }
        }
    }
}
