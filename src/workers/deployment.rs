use crate::{
    models::deployments,
    services::{
        deployment::{DeploymentError, DeploymentService},
        notification::NotificationService,
        preview::PreviewService,
    },
};
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
                if let Err(error) = PreviewService::finalize_pending_close_for_deployment(
                    &self.ctx.db,
                    args.deployment_id,
                )
                .await
                {
                    tracing::warn!(
                        deployment_id = args.deployment_id,
                        error = %error,
                        "Deferred preview cleanup remains pending"
                    );
                }
                tracing::info!(
                    deployment_id = args.deployment_id,
                    "DeploymentWorker completed deployment successfully"
                );
                Ok(())
            }
            Err(DeploymentError::Cancelled) => {
                if let Err(error) = PreviewService::finalize_pending_close_for_deployment(
                    &self.ctx.db,
                    args.deployment_id,
                )
                .await
                {
                    tracing::warn!(
                        deployment_id = args.deployment_id,
                        error = %error,
                        "Deferred preview cleanup remains pending after cancellation"
                    );
                }
                tracing::info!(
                    deployment_id = args.deployment_id,
                    "DeploymentWorker observed a user-cancelled deployment"
                );
                Ok(())
            }
            Err(DeploymentError::ExecutionClaimUnavailable { status }) => {
                tracing::info!(
                    deployment_id = args.deployment_id,
                    status = %status,
                    "DeploymentWorker skipped duplicate delivery because another worker owns the lease"
                );
                Ok(())
            }
            Err(DeploymentError::ExecutionClaimLost) => {
                tracing::info!(
                    deployment_id = args.deployment_id,
                    "DeploymentWorker stopped because execution lease ownership changed"
                );
                Ok(())
            }
            Err(err) => {
                if let Err(cleanup_error) = PreviewService::finalize_pending_close_for_deployment(
                    &self.ctx.db,
                    args.deployment_id,
                )
                .await
                {
                    tracing::warn!(
                        deployment_id = args.deployment_id,
                        error = %cleanup_error,
                        "Deferred preview cleanup remains pending after deployment failure"
                    );
                }
                tracing::error!(
                    deployment_id = args.deployment_id,
                    error = %err,
                    "DeploymentWorker execution encountered error"
                );

                if let Ok(deployment) =
                    deployments::Model::find_by_id(&self.ctx.db, args.deployment_id).await
                {
                    let _ = NotificationService::notify(
                        &self.ctx,
                        "deployment_failed",
                        "critical",
                        &format!("Deployment failure for application #{}", deployment.application_id),
                        &format!(
                            "Deployment #{} failed with code {}: {}",
                            deployment.id,
                            deployment.error_code.as_deref().unwrap_or("UNKNOWN"),
                            deployment
                                .error_message
                                .as_deref()
                                .unwrap_or_else(|| err.to_string().as_str())
                        ),
                    )
                    .await;
                }

                // Return Ok so that queue marks the job handled, as failure state is recorded in DB.
                Ok(())
            }
        }
    }
}
