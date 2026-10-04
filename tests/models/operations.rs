use chrono::Utc;
use migration::{Migrator, MigratorTrait};
use moonships::{
    models::{
        _entities::environment_variables,
        applications::{CreateApplicationParams, Model as ApplicationModel},
        deployment_revisions::Model as DeploymentRevisionModel,
        environments::{CreateEnvironmentParams, Model as EnvironmentModel},
        projects::{CreateProjectParams, Model as ProjectModel},
        servers::{self, Model as ServerModel},
    },
    services::{backup::BackupService, crypto::CryptoService},
};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, Database, EntityTrait, PaginatorTrait};
use serial_test::serial;
use std::{env, ffi::OsString};
use uuid::Uuid;

struct EnvGuard {
    values: Vec<(String, Option<OsString>)>,
}

impl EnvGuard {
    fn new(names: &[&str]) -> Self {
        Self {
            values: names
                .iter()
                .map(|name| ((*name).to_string(), env::var_os(name)))
                .collect(),
        }
    }

    fn set(&self, name: &str, value: impl AsRef<std::ffi::OsStr>) {
        env::set_var(name, value);
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (name, value) in self.values.drain(..) {
            match value {
                Some(value) => env::set_var(name, value),
                None => env::remove_var(name),
            }
        }
    }
}

#[tokio::test]
#[serial]
async fn backup_restore_drill_preserves_schema_revision_and_secret() {
    let root = std::env::temp_dir().join(format!("moonships-restore-drill-{}", Uuid::new_v4()));
    tokio::fs::create_dir_all(&root)
        .await
        .expect("create temp root");
    let source = root.join("source.sqlite");
    let backup_dir = root.join("backups");
    let restored = root.join("restored.sqlite");

    let env_guard = EnvGuard::new(&[
        "ENCRYPTION_KEY",
        "MOONSHIPS_BACKUP_DIR",
        "MOONSHIPS_BACKUP_ENCRYPT",
        "MOONSHIPS_BACKUP_RETENTION",
    ]);
    env_guard.set(
        "ENCRYPTION_KEY",
        "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
    );
    env_guard.set("MOONSHIPS_BACKUP_DIR", &backup_dir);
    env_guard.set("MOONSHIPS_BACKUP_ENCRYPT", "true");
    env_guard.set("MOONSHIPS_BACKUP_RETENTION", "3");

    let db = Database::connect(format!("sqlite://{}?mode=rwc", source.display()))
        .await
        .expect("open isolated backup source");
    Migrator::up(&db, None)
        .await
        .expect("migrate isolated backup source");

    let server = servers::ActiveModel {
        name: Set("Restore Target".to_string()),
        host: Set("127.0.0.1".to_string()),
        port: Set(22),
        username: Set("moonships".to_string()),
        authentication_type: Set("ssh_key".to_string()),
        status: Set("online".to_string()),
        created_at: Set(Utc::now().into()),
        updated_at: Set(Utc::now().into()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("insert server");

    let project = ProjectModel::create_project(
        &db,
        &CreateProjectParams {
            name: "Restore Drill".to_string(),
            slug: Some(format!("restore-{}", Uuid::new_v4().simple())),
            description: None,
        },
    )
    .await
    .expect("create project");

    let environment = EnvironmentModel::create_environment(
        &db,
        &CreateEnvironmentParams {
            project_id: project.id,
            name: "Production".to_string(),
            slug: Some(format!("prod-{}", Uuid::new_v4().simple())),
            description: None,
        },
    )
    .await
    .expect("create environment");

    let app = ApplicationModel::create_application(
        &db,
        &CreateApplicationParams {
            project_id: project.id,
            environment_id: environment.id,
            server_id: server.id,
            name: "Restore API".to_string(),
            slug: Some(format!("restore-api-{}", Uuid::new_v4().simple())),
            git_repository: "https://github.com/example/restore-api.git".to_string(),
            git_branch: Some("main".to_string()),
            build_type: Some("dockerfile".to_string()),
            dockerfile_path: Some("Dockerfile".to_string()),
            docker_context: Some(".".to_string()),
            docker_image: None,
            container_name: None,
            container_port: Some(8080),
            published_port: None,
            startup_command: None,
            healthcheck_path: Some("/health".to_string()),
            healthcheck_port: None,
            auto_deploy: Some(false),
        },
    )
    .await
    .expect("create app");

    let encrypted = CryptoService::encrypt("RESTORED_SECRET_VALUE").expect("encrypt secret");
    environment_variables::ActiveModel {
        application_id: Set(app.id),
        key: Set("RESTORE_SECRET".to_string()),
        encrypted_value: Set(encrypted.clone()),
        is_secret: Set(true),
        created_at: Set(Utc::now().into()),
        updated_at: Set(Utc::now().into()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("insert encrypted env");

    let revision = DeploymentRevisionModel::create_or_get(
        &db,
        &app,
        &ServerModel::find_by_id(&db, server.id)
            .await
            .expect("load server"),
        "0123456789abcdef",
        Some("restore drill".to_string()),
    )
    .await
    .expect("create immutable revision");

    let backup = BackupService::run_from_source(&db, &source)
        .await
        .expect("create verified encrypted backup");
    assert!(backup.run.verified);
    assert!(backup.run.encrypted);
    assert!(backup.path.to_string_lossy().ends_with(".moonshipsbak"));
    BackupService::verify_export(&backup.path)
        .await
        .expect("verify encrypted export");

    BackupService::restore_to_path(&backup.path, &restored)
        .await
        .expect("restore into clean database");

    let restored_db = Database::connect(format!("sqlite://{}?mode=rwc", restored.display()))
        .await
        .expect("open restored database");

    let restored_app = moonships::models::_entities::applications::Entity::find_by_id(app.id)
        .one(&restored_db)
        .await
        .expect("query restored app")
        .expect("restored app exists");
    assert_eq!(restored_app.name, "Restore API");

    let revision_count = moonships::models::_entities::deployment_revisions::Entity::find()
        .count(&restored_db)
        .await
        .expect("count revisions");
    assert!(revision_count >= 1);
    let restored_revision =
        moonships::models::_entities::deployment_revisions::Entity::find_by_id(revision.id)
            .one(&restored_db)
            .await
            .expect("query restored revision")
            .expect("restored revision exists");
    assert_eq!(restored_revision.source_commit_hash, "0123456789abcdef");

    let restored_secret = environment_variables::Entity::find()
        .one(&restored_db)
        .await
        .expect("query restored secret")
        .expect("restored secret exists");
    assert_eq!(restored_secret.encrypted_value, encrypted);
    assert_eq!(
        CryptoService::decrypt(&restored_secret.encrypted_value).expect("decrypt restored secret"),
        "RESTORED_SECRET_VALUE"
    );

    drop(restored_db);
    drop(db);
    drop(env_guard);
    let _ = tokio::fs::remove_dir_all(&root).await;
}

#[tokio::test]
#[serial]
async fn notification_claim_deduplicates_during_cooldown() {
    let root = std::env::temp_dir().join(format!("moonships-alert-test-{}", Uuid::new_v4()));
    tokio::fs::create_dir_all(&root)
        .await
        .expect("create alert temp root");

    let db_path = root.join("db.sqlite");
    let db = Database::connect(format!("sqlite://{}?mode=rwc", db_path.display()))
        .await
        .expect("open isolated alert database");
    Migrator::up(&db, None)
        .await
        .expect("migrate isolated alert database");

    let first = moonships::models::notification_events::Model::claim(
        &db,
        "target_unhealthy",
        "warning",
        "target unavailable",
        "Target edge-1 unhealthy",
        900,
    )
    .await
    .expect("first alert claim");
    assert!(first.should_send);

    let duplicate = moonships::models::notification_events::Model::claim(
        &db,
        "target_unhealthy",
        "warning",
        "target unavailable again",
        "Target edge-1 unhealthy",
        900,
    )
    .await
    .expect("duplicate alert claim");
    assert!(!duplicate.should_send);
    assert_eq!(duplicate.event.id, first.event.id);

    drop(db);
    let _ = tokio::fs::remove_dir_all(&root).await;
}
