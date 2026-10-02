use chrono::Utc;
use loco_rs::testing::prelude::*;
use moonships::{
    app::App,
    models::{
        applications::{CreateApplicationParams, Model as ApplicationModel},
        deployment_logs::Model as DeploymentLogModel,
        deployments::Model as DeploymentModel,
        domains::{CreateDomainParams, Model as DomainModel},
        environments::{CreateEnvironmentParams, Model as EnvironmentModel},
        projects::{CreateProjectParams, Model as ProjectModel},
        servers::{self, Model as ServerModel},
    },
};
use sea_orm::{ActiveModelTrait, ActiveValue::Set};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_models_lifecycle_and_constraints() {
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
            git_branch: Some("master".to_string()),
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
        encrypted_value: Set("SUPER_SECRET_123".to_string()),
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

    // Deployment logs append
    let log = DeploymentLogModel::append(db, dep.id, "stdout", "Build starting...")
        .await
        .expect("append log");
    assert_eq!(log.message, "Build starting...");

    let logs = DeploymentLogModel::by_deployment(db, dep.id)
        .await
        .expect("get logs");
    assert_eq!(logs.len(), 1);

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
}
