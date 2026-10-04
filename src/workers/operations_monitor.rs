use crate::services::operations::OperationsService;
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

pub struct OperationsMonitorWorker {
    pub ctx: AppContext,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct OperationsMonitorWorkerArgs {
    pub reason: String,
}

#[async_trait]
impl BackgroundWorker<OperationsMonitorWorkerArgs> for OperationsMonitorWorker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    async fn perform(&self, args: OperationsMonitorWorkerArgs) -> Result<()> {
        if let Err(error) = OperationsService::poll_targets(&self.ctx).await {
            tracing::error!(error = %error, reason = %args.reason, "Operations monitor failed");
        }
        Ok(())
    }
}
