use chrono::Utc;
use loco_rs::testing::prelude::*;
use moonships::{
    app::App,
    models::{
        applications::{CreateApplicationParams, Model as ApplicationModel},
        deployment_logs::Model as DeploymentLogModel,
        deployment_revisions::Model as DeploymentRevisionModel,
        deployments::Model as DeploymentModel,
        domains::{CreateDomainParams, Model as DomainModel},
        environments::{CreateEnvironmentParams, Model as EnvironmentModel},
        projects::{CreateProjectParams, Model as ProjectModel},
        servers::{self, Model as ServerModel},
    },
    services::{crypto::CryptoService, deployment::DeploymentService},
};
use sea_orm::{ActiveModelTrait, ActiveValue::Set};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_models_lifecycle_and_constraints() {
    std::env::set_var(
        "ENCRYPTION_KEY",
        "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
    );

    let boot = boot_test::<App>()
        .await
        .expect("Failed to boot test application");
    let db = &boot.app_context.db;

    // 1. Server Model Test
    let server_active = servers::ActiveModel {
        name: Set("Model Test Host".to_string()),
        host: Set("10.0.0.1".to_string()),
        port: Set(22),
        username: Set("admin".to_string()),
        authentication_type: Set("ssh_key".to_string()),
        encrypted_private_key: Set(Some("encrypted-payload".to_string())),
        status: Set("online".to_string()),
        created_at: Set(Utc::now().into()),
        updated_at: Set(Utc::now().into()),
        ..Default::default()
    };
    let server = server_active.insert(db).await.expect("insert server");
    assert!(server.id > 0);

    let fetched_server = ServerModel::find_by_id(db, server.id)
        .await
        .expect("find server");
    assert_eq!(fetched_server.name, "Model Test Host");

    let updated_server = ServerModel::update_status(db, server.id, "maintenance")
        .await
        .expect("update server status");
    assert_eq!(updated_server.status, "maintenance");

    // 2. Project & Environment Model Test
    let project = ProjectModel::create_project(
        db,
        &CreateProjectParams {
            name: "Core Project".to_string(),
            slug: Some("core-project".to_string()),
            description: Some("Project for model test".to_string()),
        },
    )
    .await
    .expect("create project");

    let env = EnvironmentModel::create_environment(
        db,
        &CreateEnvironmentParams {
            project_id: project.id,
            name: "Staging".to_string(),
            slug: Some("staging".to_string()),
            description: Some("Staging environment".to_string()),
        },
    )
    .await
    .expect("create environment");
    assert_eq!(env.project_id, project.id);

    // 3. Application Model Test
    let app = ApplicationModel::create_application(
        db,
        &CreateApplicationParams {
            project_id: project.id,
            environment_id: env.id,
            server_id: server.id,
            name: "Core API".to_string(),
            slug: Some("core-api".to_string()),
            git_repository: "https://github.com/bjo163/mwx-ships.git".to_string(),
            git_branch: Some("main".to_string()),
            build_type: Some("dockerfile".to_string()),
            dockerfile_path: None,
            docker_context: None,
            docker_image: None,
            container_name: None,
            container_port: Some(8080),
            published_port: None,
            startup_command: None,
            healthcheck_path: Some("/health".to_string()),
            healthcheck_port: Some(8080),
            auto_deploy: Some(false),
        },
    )
    .await
    .expect("create application");
    assert_eq!(app.slug, "core-api");
    assert_eq!(app.status, "created");

    // 4. Environment Variables & Masking Test
    let env_active = moonships::models::environment_variables::ActiveModel {
        application_id: Set(app.id),
        key: Set("APP_KEY".to_string()),
        encrypted_value: Set(
            CryptoService::encrypt("SUPER_SECRET_123").expect("encrypt test secret")
        ),
        is_secret: Set(true),
        created_at: Set(Utc::now().into()),
        updated_at: Set(Utc::now().into()),
        ..Default::default()
    };
    let saved_env = env_active.insert(db).await.expect("insert env var");
    let safe = saved_env.to_safe();
    assert_eq!(safe.key, "APP_KEY");
    assert_eq!(
        safe.value, "********",
        "to_safe() must mask secret environment variables"
    );

    // 5. Domain Model Test
    let domain = DomainModel::create_domain(
        db,
        app.id,
        &CreateDomainParams {
            hostname: "api.domain.com".to_string(),
            port: Some(8080),
            https_enabled: Some(true),
        },
    )
    .await
    .expect("create domain");
    assert_eq!(domain.hostname, "api.domain.com");

    let domains_list = DomainModel::by_application(db, app.id)
        .await
        .expect("get domains");
    assert_eq!(domains_list.len(), 1);

    // 6. Deployments & Concurrency Lock Test
    assert!(
        !DeploymentModel::has_active_deployment(db, app.id)
            .await
            .expect("active deployment check"),
        "initially no deployment should be active"
    );

    let dep = DeploymentModel::create_deployment(
        db,
        app.id,
        server.id,
        Some("c0ffee1".to_string()),
        Some("Deploying new feature".to_string()),
    )
    .await
    .expect("create deployment");
    assert_eq!(dep.status, "queued");

    // has_active_deployment MUST now return true because status is 'queued'
    assert!(
        DeploymentModel::has_active_deployment(db, app.id)
            .await
            .expect("active deployment check"),
        "queued deployment must be recognized as active lock"
    );

    // 7. Immutable deployment revision snapshot
    let revision = DeploymentRevisionModel::create_or_get(
        db,
        &app,
        &server,
        "c0ffee1",
        Some("Deploying new feature".to_string()),
    )
    .await
    .expect("create immutable deployment revision");

    assert!(
        revision
            .image_reference
            .starts_with("moonships/core-api:rev-"),
        "dockerfile revisions must use immutable revision-specific image tags"
    );
    assert!(
        !revision.runtime_snapshot.contains("SUPER_SECRET_123"),
        "plaintext secret values must never appear in revision snapshots"
    );

    let snapshot = revision.snapshot().expect("decode revision snapshot");
    let secret = snapshot
        .environment
        .iter()
        .find(|item| item.key == "APP_KEY")
        .expect("secret env snapshot");
    assert!(secret.is_secret);
    assert!(
        secret.value.is_none(),
        "secret revision entries must not persist plaintext values"
    );
    assert!(!secret.value_fingerprint.is_empty());
    let encrypted_secret = secret
        .encrypted_value
        .as_deref()
        .expect("revision must retain encrypted secret material");
    assert!(!encrypted_secret.contains("SUPER_SECRET_123"));
    assert_eq!(
        CryptoService::decrypt(encrypted_secret).expect("decrypt revision secret"),
        "SUPER_SECRET_123"
    );

    let duplicate = DeploymentRevisionModel::create_or_get(
        db,
        &app,
        &server,
        "c0ffee1",
        Some("Deploying new feature".to_string()),
    )
    .await
    .expect("deduplicate identical revision");
    assert_eq!(
        duplicate.id, revision.id,
        "identical immutable state must resolve to the same revision"
    );

    let dep = DeploymentModel::attach_revision(
        db,
        dep.id,
        revision.id,
        "c0ffee1",
        Some("Deploying new feature".to_string()),
    )
    .await
    .expect("attach revision to deployment");
    assert_eq!(dep.revision_id, Some(revision.id));

    // Deployment logs append
    let log = DeploymentLogModel::append(db, dep.id, "stdout", "Build starting...")
        .await
        .expect("append log");
    assert_eq!(log.message, "Build starting...");

    let logs = DeploymentLogModel::by_deployment(db, dep.id)
        .await
        .expect("get logs");
    assert_eq!(logs.len(), 1);

    DeploymentRevisionModel::mark_healthy(db, revision.id)
        .await
        .expect("mark first revision healthy");
    let promoted = ApplicationModel::promote_revision(db, app.id, revision.id)
        .await
        .expect("promote first revision");
    assert_eq!(promoted.current_revision_id, Some(revision.id));
    assert_eq!(promoted.previous_revision_id, None);

    // Finish deployment -> success
    let finished_dep = DeploymentModel::update_status(db, dep.id, "success")
        .await
        .expect("update deployment status");
    assert_eq!(finished_dep.status, "success");

    // Lock is now released
    assert!(
        !DeploymentModel::has_active_deployment(db, app.id)
            .await
            .expect("active deployment check"),
        "once deployment is success, lock must be released"
    );

    // 8. A second successful revision moves the previous pointer without mutating history.
    let second_dep = DeploymentModel::create_deployment(
        db,
        app.id,
        server.id,
        Some("decaf02".to_string()),
        Some("Second revision".to_string()),
    )
    .await
    .expect("create second deployment");

    let second_revision = DeploymentRevisionModel::create_or_get(
        db,
        &app,
        &server,
        "decaf02",
        Some("Second revision".to_string()),
    )
    .await
    .expect("create second revision");
    assert_ne!(second_revision.id, revision.id);

    DeploymentModel::attach_revision(
        db,
        second_dep.id,
        second_revision.id,
        "decaf02",
        Some("Second revision".to_string()),
    )
    .await
    .expect("attach second revision");

    DeploymentRevisionModel::mark_healthy(db, second_revision.id)
        .await
        .expect("mark second revision healthy");
    let promoted = ApplicationModel::promote_revision(db, app.id, second_revision.id)
        .await
        .expect("promote second revision");
    assert_eq!(promoted.current_revision_id, Some(second_revision.id));
    assert_eq!(promoted.previous_revision_id, Some(revision.id));

    let second_dep = DeploymentModel::update_status(db, second_dep.id, "success")
        .await
        .expect("finish second deployment");
    assert_eq!(second_dep.status, "success");

    // 9. Safe cancellation is terminal and releases the active lock.
    let cancellable = DeploymentModel::create_deployment_attempt(
        db,
        app.id,
        server.id,
        Some("decaf02".to_string()),
        Some("Retryable revision".to_string()),
        "manual",
        None,
        Some(second_revision.id),
    )
    .await
    .expect("create cancellable deployment");

    let cancelled = DeploymentModel::cancel_if_safe(db, cancellable.id)
        .await
        .expect("cancel deployment")
        .expect("queued deployment should be cancellable");
    assert_eq!(cancelled.status, "cancelled");
    assert!(
        !DeploymentModel::has_active_deployment(db, app.id)
            .await
            .expect("active lock after cancellation"),
        "cancelled deployment must release the application lock"
    );

    let resurrect =
        DeploymentModel::transition_status_if(db, cancelled.id, &["queued"], "connecting")
            .await
            .expect("attempt transition from cancelled");
    assert!(
        resurrect.is_none(),
        "cancelled deployment must not be resurrected by worker phase transition"
    );

    // 10. Retry creates a new attempt and preserves immutable revision/source intent.
    let retry = DeploymentService::retry_deployment(db, cancelled.id)
        .await
        .expect("retry cancelled deployment");
    assert_ne!(retry.id, cancelled.id);
    assert_eq!(retry.status, "queued");
    assert_eq!(retry.revision_id, Some(second_revision.id));
    assert_eq!(retry.commit_hash.as_deref(), Some("decaf02"));
    assert_eq!(retry.trigger_kind, "retry");
    assert_eq!(retry.source_deployment_id, Some(cancelled.id));

    let retry_cancelled = DeploymentModel::cancel_if_safe(db, retry.id)
        .await
        .expect("cancel retry")
        .expect("retry should still be in safe queued phase");
    assert_eq!(retry_cancelled.status, "cancelled");

    // 11. Rollback queues the previous immutable known-good revision as a new attempt.
    let rollback = DeploymentService::rollback_application(db, app.id)
        .await
        .expect("queue rollback");
    assert_eq!(rollback.status, "queued");
    assert_eq!(rollback.trigger_kind, "rollback");
    assert_eq!(rollback.revision_id, Some(revision.id));
    assert_eq!(rollback.commit_hash.as_deref(), Some("c0ffee1"));
    assert_eq!(rollback.source_deployment_id, Some(second_dep.id));

    DeploymentModel::cancel_if_safe(db, rollback.id)
        .await
        .expect("cancel queued rollback")
        .expect("rollback should be cancellable before execution");

    // 12. Cancellation is rejected once destructive replacement begins.
    let destructive = DeploymentModel::create_deployment(
        db,
        app.id,
        server.id,
        Some("deadbee".to_string()),
        Some("Destructive phase test".to_string()),
    )
    .await
    .expect("create destructive-phase deployment");
    let destructive = DeploymentModel::update_status(db, destructive.id, "stopping_old")
        .await
        .expect("move deployment to destructive phase");
    assert_eq!(destructive.status, "stopping_old");
    assert!(
        DeploymentModel::cancel_if_safe(db, destructive.id)
            .await
            .expect("attempt unsafe cancel")
            .is_none(),
        "cancel must fail closed after destructive replacement begins"
    );
    DeploymentModel::record_failure(db, destructive.id, "TEST_CLEANUP", "test cleanup", None)
        .await
        .expect("finish destructive-phase test deployment");
}
