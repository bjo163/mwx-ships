#![allow(elided_lifetimes_in_paths)]
#![allow(clippy::wildcard_imports)]
pub use sea_orm_migration::prelude::*;
mod m20220101_000001_users;
mod m20260101_000002_moonships_core;
mod m20261004_000003_deployment_revisions;
mod m20261004_000004_deployment_attempt_intent;
mod m20261004_000005_deployment_execution_lease;
mod m20261004_000006_managed_ingress;
mod m20261004_000007_git_automation;
mod m20261004_000008_preview_runtime;
mod m20261004_000009_preview_environment;
mod m20261004_000010_webhook_intent_dedupe;
mod m20261004_000011_git_clone_credentials;
mod m20261004_000012_operations;
mod m20261004_000013_access_control;
mod m20261004_000014_active_deployment_lock;
mod m20261004_000015_scale_workloads;
mod m20261005_000016_persistent_volumes;
mod m20261005_000017_managed_services;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_users::Migration),
            Box::new(m20260101_000002_moonships_core::Migration),
            Box::new(m20261004_000003_deployment_revisions::Migration),
            Box::new(m20261004_000004_deployment_attempt_intent::Migration),
            Box::new(m20261004_000005_deployment_execution_lease::Migration),
            Box::new(m20261004_000006_managed_ingress::Migration),
            Box::new(m20261004_000007_git_automation::Migration),
            Box::new(m20261004_000008_preview_runtime::Migration),
            Box::new(m20261004_000009_preview_environment::Migration),
            Box::new(m20261004_000010_webhook_intent_dedupe::Migration),
            Box::new(m20261004_000011_git_clone_credentials::Migration),
            Box::new(m20261004_000012_operations::Migration),
            Box::new(m20261004_000013_access_control::Migration),
            Box::new(m20261004_000014_active_deployment_lock::Migration),
            Box::new(m20261004_000015_scale_workloads::Migration),
            Box::new(m20261005_000016_persistent_volumes::Migration),
            Box::new(m20261005_000017_managed_services::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
