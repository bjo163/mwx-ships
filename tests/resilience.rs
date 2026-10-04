use chrono::{Duration as ChronoDuration, Utc};
use loco_rs::testing::prelude::*;
use moonships::{
    app::App,
    models::{
        applications::{CreateApplicationParams, Model as ApplicationModel},
        deployments::Model as DeploymentModel,
        environments::{CreateEnvironmentParams, Model as EnvironmentModel},
        projects::{CreateProjectParams, Model as ProjectModel},
        servers,
    },
};
use sea_orm::{ActiveModelTrait, ActiveValue::Set};
use serial_test::serial;
use std::time::{Duration, Instant};

#[tokio::test]
#[serial]
async fn ga_deployment_state_machine_soak_and_failure_matrix() {
    let boot = boot_test::<App>()
        .await
        .expect("boot GA resilience test application");
    let db = &boot.app_context.db;
    let (server_id, application_id) = seed_application(db).await;

    let started = Instant::now();
    for attempt in 0..128_i64 {
        let deployment = DeploymentModel::create_deployment(
            db,
            application_id,
            server_id,
            Some(format!("{attempt:07x}")),
            Some(format!("GA soak attempt {attempt}")),
        )
        .await
        .expect("create soak deployment");

        assert!(
            ApplicationModel::claim_active_deployment(db, application_id, deployment.id)
                .await
                .expect("claim application during soak"),
            "each terminalized soak attempt must release ownership for the next attempt"
        );

        let claim = DeploymentModel::claim_for_execution(db, deployment.id, 7200)
            .await
            .expect("claim deployment execution")
            .expect("queued deployment must be claimable");

        let token = claim.token;
        let mut current = "connecting";
        for next in [
            "cloning",
            "building",
            "stopping_old",
            "starting_new",
            "healthchecking",
            "success",
        ] {
            let transitioned = DeploymentModel::transition_status_if_owned(
                db,
                deployment.id,
                &[current],
                next,
                Some(&token),
            )
            .await
            .expect("owned state transition")
            .unwrap_or_else(|| panic!("transition {current} -> {next} lost ownership"));
            assert_eq!(transitioned.status, next);
            current = next;
        }

        assert!(
            ApplicationModel::release_active_deployment(db, application_id, deployment.id)
                .await
                .expect("release application after successful soak attempt")
        );
    }

    assert!(
        started.elapsed() < Duration::from_secs(120),
        "128-attempt database state-machine soak exceeded the 120s GA budget"
    );

    let application = ApplicationModel::find_by_id(db, application_id)
        .await
        .expect("reload application after soak");
    assert!(
        application.active_deployment_id.is_none(),
        "soak must not leave application ownership locked"
    );

    let phases = [
        "connecting",
        "cloning",
        "building",
        "stopping_old",
        "starting_new",
        "healthchecking",
    ];

    for (index, failure_phase) in phases.iter().enumerate() {
        let deployment = DeploymentModel::create_deployment(
            db,
            application_id,
            server_id,
            Some(format!("f{index:06x}")),
            Some(format!("failure injection at {failure_phase}")),
        )
        .await
        .expect("create failure injection deployment");

        assert!(
            ApplicationModel::claim_active_deployment(db, application_id, deployment.id)
                .await
                .expect("claim application for failure injection")
        );

        let claim = DeploymentModel::claim_for_execution(db, deployment.id, 7200)
            .await
            .expect("claim failure-injection execution")
            .expect("failure-injection deployment must be claimable");

        let token = claim.token;
        let mut current = "connecting";
        for next in [
            "cloning",
            "building",
            "stopping_old",
            "starting_new",
            "healthchecking",
        ] {
            if current == *failure_phase {
                break;
            }
            DeploymentModel::transition_status_if_owned(
                db,
                deployment.id,
                &[current],
                next,
                Some(&token),
            )
            .await
            .expect("advance to failure phase")
            .expect("lease owner must advance to failure phase");
            current = next;
        }

        let before_failure = DeploymentModel::find_by_id(db, deployment.id)
            .await
            .expect("reload before failure");
        assert_eq!(before_failure.status, *failure_phase);
        assert_eq!(
            before_failure.execution_token.as_deref(),
            Some(token.as_str()),
            "failure must be recorded only by the current lease owner"
        );

        let failed = DeploymentModel::record_failure(
            db,
            deployment.id,
            "GA_FAILURE_INJECTION",
            &format!("synthetic failure at {failure_phase}"),
            Some(97),
        )
        .await
        .expect("record synthetic failure");
        assert_eq!(failed.status, "failed");
        assert!(failed.execution_token.is_none());
        assert!(failed.lease_expires_at.is_none());

        assert!(
            ApplicationModel::release_active_deployment(db, application_id, deployment.id)
                .await
                .expect("release application after failure")
        );
    }

    let stale = DeploymentModel::create_deployment(
        db,
        application_id,
        server_id,
        Some("5a1e000".to_string()),
        Some("stale lease recovery".to_string()),
    )
    .await
    .expect("create stale lease deployment");
    assert!(
        ApplicationModel::claim_active_deployment(db, application_id, stale.id)
            .await
            .expect("claim application for stale lease")
    );

    let first = DeploymentModel::claim_for_execution(db, stale.id, 7200)
        .await
        .expect("claim first execution")
        .expect("first claim exists");
    let mut active: moonships::models::deployments::ActiveModel = first.deployment.into();
    active.status = Set("building".to_string());
    active.lease_expires_at = Set(Some((Utc::now() - ChronoDuration::seconds(1)).into()));
    active.update(db).await.expect("expire execution lease");

    let recovered = DeploymentModel::claim_for_execution(db, stale.id, 7200)
        .await
        .expect("reclaim stale execution")
        .expect("stale execution must be recoverable");
    assert_eq!(recovered.recovered_from.as_deref(), Some("building"));
    assert_ne!(recovered.token, first.token);

    assert!(
        DeploymentModel::transition_status_if_owned(
            db,
            stale.id,
            &["connecting"],
            "cloning",
            Some(&first.token),
        )
        .await
        .expect("superseded owner transition")
        .is_none(),
        "superseded worker must not advance a reclaimed deployment"
    );

    DeploymentModel::record_failure(
        db,
        stale.id,
        "GA_STALE_RECOVERY_COMPLETE",
        "cleanup after stale-recovery drill",
        None,
    )
    .await
    .expect("terminalize stale-recovery drill");
    assert!(
        ApplicationModel::release_active_deployment(db, application_id, stale.id)
            .await
            .expect("release stale-recovery application lock")
    );
}

async fn seed_application(db: &sea_orm::DatabaseConnection) -> (i64, i64) {
    let now = Utc::now();
    let server = servers::ActiveModel {
        name: Set("GA Resilience Host".to_string()),
        host: Set("127.0.0.1".to_string()),
        port: Set(22),
        username: Set("deploy".to_string()),
        authentication_type: Set("ssh_key".to_string()),
        encrypted_private_key: Set(None),
        status: Set("online".to_string()),
        created_at: Set(now.into()),
        updated_at: Set(now.into()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("insert GA resilience server");

    let project = ProjectModel::create_project(
        db,
        &CreateProjectParams {
            name: "GA Resilience Project".to_string(),
            slug: Some("ga-resilience-project".to_string()),
            description: Some("GA state-machine soak".to_string()),
        },
    )
    .await
    .expect("create GA resilience project");

    let environment = EnvironmentModel::create_environment(
        db,
        &CreateEnvironmentParams {
            project_id: project.id,
            name: "GA".to_string(),
            slug: Some("ga".to_string()),
            description: Some("GA resilience environment".to_string()),
        },
    )
    .await
    .expect("create GA resilience environment");

    let application = ApplicationModel::create_application(
        db,
        &CreateApplicationParams {
            project_id: project.id,
            environment_id: environment.id,
            server_id: server.id,
            server_pool_id: None,
            resource_units: Some(1),
            name: "GA Resilience App".to_string(),
            slug: Some("ga-resilience-app".to_string()),
            git_repository: "https://example.invalid/ga.git".to_string(),
            git_branch: Some("main".to_string()),
            build_type: Some("dockerfile".to_string()),
            workload_type: Some("single".to_string()),
            compose_file_path: None,
            compose_project_name: None,
            registry_credential_id: None,
            dockerfile_path: Some("Dockerfile".to_string()),
            docker_context: Some(".".to_string()),
            docker_image: None,
            container_name: Some("ga-resilience-app".to_string()),
            container_port: Some(8080),
            published_port: None,
            startup_command: None,
            healthcheck_path: Some("/health".to_string()),
            healthcheck_port: Some(8080),
            auto_deploy: Some(false),
        },
    )
    .await
    .expect("create GA resilience application");

    (server.id, application.id)
}
