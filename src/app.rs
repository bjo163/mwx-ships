use async_trait::async_trait;
use loco_rs::{
    app::{AppContext, Hooks, Initializer},
    bgworker::{BackgroundWorker, Queue},
    boot::{create_app, BootResult, StartMode},
    config::Config,
    controller::AppRoutes,
    db::{self, truncate_table},
    environment::Environment,
    task::Tasks,
    Result,
};
use migration::Migrator;
use std::{path::Path, time::Duration};

#[allow(unused_imports)]
use crate::{
    controllers,
    models::_entities::users,
    tasks,
    workers::{
        backup::{BackupWorker, BackupWorkerArgs},
        downloader::DownloadWorker,
    },
};

pub struct App;
#[async_trait]
impl Hooks for App {
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    fn app_version() -> String {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("dev")
        )
    }

    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: Config,
    ) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment, config).await
    }

    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![])
    }

    async fn before_run(app_context: &AppContext) -> Result<()> {
        let production_default = std::env::var("LOCO_ENV")
            .ok()
            .map(|value| value.eq_ignore_ascii_case("production"))
            .unwrap_or(false)
            .then_some(86_400u64)
            .unwrap_or(0);

        let interval = std::env::var("MOONSHIPS_BACKUP_INTERVAL_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(production_default);

        if interval > 0 {
            let ctx = app_context.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(interval)).await;
                loop {
                    if let Err(error) = BackupWorker::perform_later(
                        &ctx,
                        BackupWorkerArgs {
                            reason: "scheduled".to_string(),
                        },
                    )
                    .await
                    {
                        tracing::error!(error = %error, "Unable to enqueue scheduled backup");
                    }
                    tokio::time::sleep(Duration::from_secs(interval)).await;
                }
            });
        }

        Ok(())
    }

    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes() // controller routes below
            .add_route(controllers::auth::routes())
            .add_route(controllers::health::routes())
            .add_route(controllers::servers::routes())
            .add_route(controllers::projects::routes())
            .add_route(controllers::applications::routes())
            .add_route(controllers::deployments::routes())
            .add_route(controllers::webhooks::routes())
    }
    async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()> {
        queue.register(DownloadWorker::build(ctx)).await?;
        queue.register(BackupWorker::build(ctx)).await?;
        queue
            .register(crate::workers::deployment::DeploymentWorker::build(ctx))
            .await?;
        Ok(())
    }

    #[allow(unused_variables)]
    fn register_tasks(tasks: &mut Tasks) {
        // tasks-inject (do not remove)
        tasks.register(tasks::user_create::UserCreate);
        tasks.register(tasks::backup_run::BackupRun);
        tasks.register(tasks::backup_verify::BackupVerify);
        tasks.register(tasks::backup_restore::BackupRestore);
    }
    async fn truncate(ctx: &AppContext) -> Result<()> {
        truncate_table(&ctx.db, users::Entity).await?;
        Ok(())
    }
    async fn seed(ctx: &AppContext, base: &Path) -> Result<()> {
        db::seed::<users::ActiveModel>(&ctx.db, &base.join("users.yaml").display().to_string())
            .await?;
        Ok(())
    }
}
