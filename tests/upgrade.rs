use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseBackend, Statement};
use serial_test::serial;

const VERSION_PREFIXES: &[(&str, u32)] = &[
    ("0.2", 2),
    ("0.3", 5),
    ("0.4", 6),
    ("0.5", 11),
    ("0.6", 12),
    ("0.7", 13),
    ("0.8", 15),
    ("0.9", 15),
];

#[tokio::test]
#[serial]
async fn supported_schema_versions_upgrade_to_latest_without_data_loss() {
    for (version, migration_count) in VERSION_PREFIXES {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("connect upgrade fixture database");

        Migrator::up(&db, Some(*migration_count))
            .await
            .unwrap_or_else(|err| panic!("migrate fixture v{version}: {err}"));

        seed_legacy_core(&db, version).await;

        Migrator::up(&db, None)
            .await
            .unwrap_or_else(|err| panic!("upgrade fixture v{version} to latest: {err}"));

        let application_slug = scalar_text(
            &db,
            "SELECT slug AS value FROM applications WHERE slug='upgrade-app'",
        )
        .await;
        assert_eq!(
            application_slug, "upgrade-app",
            "application identity must survive upgrade from v{version}"
        );

        let encrypted_secret = scalar_text(
            &db,
            "SELECT encrypted_value AS value FROM environment_variables WHERE key='LEGACY_SECRET'",
        )
        .await;
        assert_eq!(
            encrypted_secret, "ciphertext-preserved-verbatim",
            "encrypted secret bytes must not be rewritten while upgrading from v{version}"
        );

        let migration_rows =
            scalar_i64(&db, "SELECT COUNT(*) AS value FROM seaql_migrations").await;
        assert_eq!(
            migration_rows, 15,
            "fixture v{version} must reach every current migration"
        );

        for table in [
            "deployment_revisions",
            "organizations",
            "server_pools",
            "registry_credentials",
        ] {
            let count = scalar_i64(
                &db,
                &format!(
                    "SELECT COUNT(*) AS value FROM sqlite_master WHERE type='table' AND name='{table}'"
                ),
            )
            .await;
            assert_eq!(
                count, 1,
                "latest schema table '{table}' missing after upgrade from v{version}"
            );
        }

        let integrity = scalar_text(&db, "PRAGMA integrity_check").await;
        assert_eq!(
            integrity, "ok",
            "SQLite integrity check failed after upgrade from v{version}"
        );
    }
}

async fn seed_legacy_core(db: &sea_orm::DatabaseConnection, version: &str) {
    for sql in [
        "INSERT INTO servers(
            id,name,host,port,username,authentication_type,status,created_at,updated_at
         ) VALUES(
            1,'Upgrade Host','127.0.0.1',22,'deploy','ssh_key','online',
            '2026-10-05T00:00:00+00:00','2026-10-05T00:00:00+00:00'
         )",
        "INSERT INTO projects(
            id,name,slug,description,created_at,updated_at
         ) VALUES(
            1,'Upgrade Project','upgrade-project','upgrade fixture',
            '2026-10-05T00:00:00+00:00','2026-10-05T00:00:00+00:00'
         )",
        "INSERT INTO environments(
            id,project_id,name,slug,description,created_at,updated_at
         ) VALUES(
            1,1,'Production','production','upgrade fixture',
            '2026-10-05T00:00:00+00:00','2026-10-05T00:00:00+00:00'
         )",
        "INSERT INTO applications(
            id,project_id,environment_id,server_id,name,slug,git_repository,git_branch,
            build_type,dockerfile_path,docker_context,container_name,container_port,
            auto_deploy,status,created_at,updated_at
         ) VALUES(
            1,1,1,1,'Upgrade App','upgrade-app','https://example.invalid/repo.git','main',
            'dockerfile','Dockerfile','.','moonships-upgrade-app',8080,
            0,'running','2026-10-05T00:00:00+00:00','2026-10-05T00:00:00+00:00'
         )",
        "INSERT INTO environment_variables(
            id,application_id,key,encrypted_value,is_secret,created_at,updated_at
         ) VALUES(
            1,1,'LEGACY_SECRET','ciphertext-preserved-verbatim',1,
            '2026-10-05T00:00:00+00:00','2026-10-05T00:00:00+00:00'
         )",
    ] {
        db.execute_raw(Statement::from_string(DatabaseBackend::Sqlite, sql))
            .await
            .unwrap_or_else(|err| panic!("seed v{version} fixture failed: {err}\nSQL: {sql}"));
    }
}

async fn scalar_text(db: &sea_orm::DatabaseConnection, sql: &str) -> String {
    let row = db
        .query_one_raw(Statement::from_string(DatabaseBackend::Sqlite, sql))
        .await
        .expect("query scalar text")
        .expect("scalar text row");
    row.try_get_by_index(0).expect("scalar text value")
}

async fn scalar_i64(db: &sea_orm::DatabaseConnection, sql: &str) -> i64 {
    let row = db
        .query_one_raw(Statement::from_string(DatabaseBackend::Sqlite, sql))
        .await
        .expect("query scalar integer")
        .expect("scalar integer row");
    row.try_get_by_index(0).expect("scalar integer value")
}
