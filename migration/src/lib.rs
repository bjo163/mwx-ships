#![allow(elided_lifetimes_in_paths)]
#![allow(clippy::wildcard_imports)]
pub use sea_orm_migration::prelude::*;
mod m20220101_000001_users;
mod m20260101_000002_moonships_core;
mod m20261004_000003_deployment_revisions;
mod m20261004_000004_deployment_attempt_intent;
mod m20261004_000005_deployment_execution_lease;

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
            // inject-above (do not remove this comment)
        ]
    }
}
