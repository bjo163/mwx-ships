use chrono::{Duration, Utc};
use loco_rs::testing::prelude::*;
use moonships::{
    app::App,
    models::{
        api_tokens::{CreateApiTokenParams, Model as ApiTokenModel},
        applications::{CreateApplicationParams, Model as ApplicationModel},
        audit_events::{AuditEventInput, Model as AuditEventModel},
        auth_rate_limits::Model as AuthRateLimitModel,
        deployment_logs::Model as DeploymentLogModel,
        deployment_revisions::Model as DeploymentRevisionModel,
        deployments::Model as DeploymentModel,
        domains::{CreateDomainParams, Model as DomainModel},
        environments::{CreateEnvironmentParams, Model as EnvironmentModel},
        git_integrations::{Model as GitIntegrationModel, UpsertGitIntegrationParams},
        organization_memberships::{Model as MembershipModel, SetMembershipParams},
        organizations::{CreateOrganizationParams, Model as OrganizationModel},
        preview_deployments::{Model as PreviewModel, UpsertPreviewInput},
        projects::{CreateProjectParams, Model as ProjectModel},
        servers::{self, Model as ServerModel},
        users::{LoginParams, Model as UserModel, RegisterParams},
        webhook_deliveries::{Model as WebhookDeliveryModel, WebhookDeliveryInput},
    },
    services::{
        access_control::{Permission, Principal},
        crypto::CryptoService,
        deployment::DeploymentService,
    },
};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection};
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
            server_pool_id: None,
            resource_units: Some(1),
            name: "Core API".to_string(),
            slug: Some("core-api".to_string()),
            git_repository: "https://github.com/bjo163/mwx-ships.git".to_string(),
            git_branch: Some("main".to_string()),
            build_type: Some("dockerfile".to_string()),
            workload_type: Some("single".to_string()),
            compose_file_path: None,
            compose_project_name: None,
            registry_credential_id: None,
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
    assert_eq!(domain.verification_status, "pending");
    assert_eq!(domain.tls_status, "pending");

    let verified = DomainModel::update_verification(db, domain.id, true, None)
        .await
        .expect("verify domain state");
    assert_eq!(verified.verification_status, "verified");
    assert!(verified.verified_at.is_some());

    let tls_active = DomainModel::update_tls_status(db, domain.id, "active", None)
        .await
        .expect("activate TLS state");
    assert_eq!(tls_active.tls_status, "active");

    let by_hostname = DomainModel::find_by_application_hostname(db, app.id, "API.DOMAIN.COM")
        .await
        .expect("find domain by normalized hostname")
        .expect("domain exists");
    assert_eq!(by_hostname.id, domain.id);

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

    assert!(
        ApplicationModel::claim_active_deployment(db, app.id, dep.id)
            .await
            .expect("claim active deployment"),
        "first deployment intent must atomically claim application ownership"
    );
    assert!(
        !ApplicationModel::claim_active_deployment(db, app.id, dep.id + 999)
            .await
            .expect("competing deployment claim"),
        "competing deployment intent must lose the application CAS lock"
    );

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
    assert!(
        ApplicationModel::release_active_deployment(db, app.id, dep.id)
            .await
            .expect("release active deployment"),
        "terminal deployment must release application ownership"
    );

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

    let retry_cancelled = DeploymentService::cancel_deployment(db, retry.id)
        .await
        .expect("cancel retry through service");
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

    DeploymentService::cancel_deployment(db, rollback.id)
        .await
        .expect("cancel queued rollback through service");

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

    // 13. Execution leases reject duplicate workers and allow stale recovery.
    let leased = DeploymentModel::create_deployment(
        db,
        app.id,
        server.id,
        Some("feed123".to_string()),
        Some("Lease test".to_string()),
    )
    .await
    .expect("create lease test deployment");

    let first_claim = DeploymentModel::claim_for_execution(db, leased.id, 7200)
        .await
        .expect("claim deployment")
        .expect("first worker should acquire execution lease");
    assert_eq!(first_claim.deployment.status, "connecting");
    assert_eq!(first_claim.deployment.attempt_count, 1);
    assert!(first_claim.recovered_from.is_none());

    let duplicate_claim = DeploymentModel::claim_for_execution(db, leased.id, 7200)
        .await
        .expect("duplicate claim check");
    assert!(
        duplicate_claim.is_none(),
        "second worker must not acquire an unexpired lease"
    );

    let mut stale: moonships::models::deployments::ActiveModel =
        first_claim.deployment.clone().into();
    stale.status = Set("building".to_string());
    stale.lease_expires_at = Set(Some((Utc::now() - Duration::minutes(1)).into()));
    let stale = stale.update(db).await.expect("make lease stale");

    let recovered = DeploymentModel::claim_for_execution(db, stale.id, 7200)
        .await
        .expect("reclaim stale deployment")
        .expect("stale execution must be recoverable");
    assert_eq!(recovered.recovered_from.as_deref(), Some("building"));
    assert_eq!(recovered.deployment.status, "connecting");
    assert_eq!(recovered.deployment.attempt_count, 2);
    assert_ne!(recovered.token, first_claim.token);

    let old_owner_transition = DeploymentModel::transition_status_if_owned(
        db,
        leased.id,
        &["connecting"],
        "cloning",
        Some(&first_claim.token),
    )
    .await
    .expect("old owner transition");
    assert!(
        old_owner_transition.is_none(),
        "superseded worker token must not advance deployment state"
    );

    let new_owner_transition = DeploymentModel::transition_status_if_owned(
        db,
        leased.id,
        &["connecting"],
        "cloning",
        Some(&recovered.token),
    )
    .await
    .expect("new owner transition")
    .expect("current lease owner should advance state");
    assert_eq!(new_owner_transition.status, "cloning");

    let cancelled = DeploymentModel::cancel_if_safe(db, leased.id)
        .await
        .expect("cancel lease test")
        .expect("cloning remains a safe cancellation phase");
    assert_eq!(cancelled.status, "cancelled");
    assert!(cancelled.execution_token.is_none());
    assert!(cancelled.lease_expires_at.is_none());

    // 14. Git provider credentials remain encrypted and safe API output is secret-free.
    let integration = GitIntegrationModel::upsert(
        db,
        app.id,
        &UpsertGitIntegrationParams {
            provider: "github".to_string(),
            repository_ref: "bjo163/mwx-ships".to_string(),
            api_base_url: None,
            git_username: None,
            token: Some("github-token-super-secret".to_string()),
            webhook_secret: "webhook-secret-1234567890".to_string(),
            enabled: Some(true),
        },
    )
    .await
    .expect("create git integration");

    assert_ne!(
        integration.encrypted_token.as_deref(),
        Some("github-token-super-secret")
    );
    assert_ne!(
        integration.encrypted_webhook_secret,
        "webhook-secret-1234567890"
    );
    assert_eq!(
        integration.token().expect("decrypt token").as_deref(),
        Some("github-token-super-secret")
    );
    assert_eq!(
        integration
            .webhook_secret()
            .expect("decrypt webhook secret"),
        "webhook-secret-1234567890"
    );
    let safe_json =
        serde_json::to_string(&integration.to_safe()).expect("serialize safe integration");
    assert!(!safe_json.contains("github-token-super-secret"));
    assert!(!safe_json.contains("webhook-secret-1234567890"));

    // 15. Webhook delivery identity is idempotent even when provider retries.
    let delivery_input = WebhookDeliveryInput {
        application_id: app.id,
        provider: "github".to_string(),
        delivery_id: "delivery-model-test".to_string(),
        event_kind: "push".to_string(),
        source_ref: Some("main".to_string()),
        commit_sha: Some("0123456789abcdef".to_string()),
        external_request_id: None,
        action: None,
    };
    let first_delivery = WebhookDeliveryModel::create_or_get(db, &delivery_input)
        .await
        .expect("create webhook delivery");
    assert!(first_delivery.inserted);
    let duplicate_delivery = WebhookDeliveryModel::create_or_get(db, &delivery_input)
        .await
        .expect("dedupe webhook delivery");
    assert!(!duplicate_delivery.inserted);
    assert_eq!(
        duplicate_delivery.delivery.id, first_delivery.delivery.id,
        "same provider delivery must map to one ledger row"
    );

    let same_commit_new_delivery = WebhookDeliveryModel::create_or_get(
        db,
        &WebhookDeliveryInput {
            delivery_id: "delivery-model-test-2".to_string(),
            ..delivery_input.clone()
        },
    )
    .await
    .expect("dedupe same commit under new provider delivery id");
    assert!(!same_commit_new_delivery.inserted);
    assert_eq!(
        same_commit_new_delivery.delivery.id, first_delivery.delivery.id,
        "same push commit/ref must map to one deployment intent"
    );

    // 16. Preview identity is stable across synchronize events.
    let preview = PreviewModel::upsert(
        db,
        &UpsertPreviewInput {
            application_id: app.id,
            provider: "github".to_string(),
            external_request_id: "42".to_string(),
            source_ref: "feature/preview".to_string(),
            commit_sha: "abcdef0123456789".to_string(),
        },
    )
    .await
    .expect("create preview identity");
    assert_eq!(preview.preview_slug, format!("preview-{}-42", app.id));

    let preview_updated = PreviewModel::upsert(
        db,
        &UpsertPreviewInput {
            application_id: app.id,
            provider: "github".to_string(),
            external_request_id: "42".to_string(),
            source_ref: "feature/preview".to_string(),
            commit_sha: "fedcba9876543210".to_string(),
        },
    )
    .await
    .expect("update preview identity");
    assert_eq!(preview_updated.id, preview.id);
    assert_eq!(preview_updated.commit_sha, "fedcba9876543210");
}

#[tokio::test]
#[serial]
async fn test_v07_access_control_security_contract() {
    let boot = boot_test::<App>()
        .await
        .expect("Failed to boot v0.7 test application");
    let db = &boot.app_context.db;

    let owner = UserModel::create_with_password(
        db,
        &RegisterParams {
            email: "owner@example.com".to_string(),
            password: "owner-password-123".to_string(),
            name: "Owner".to_string(),
        },
    )
    .await
    .expect("create owner user");

    let admin = UserModel::create_with_password(
        db,
        &RegisterParams {
            email: "admin@example.com".to_string(),
            password: "admin-password-123".to_string(),
            name: "Admin".to_string(),
        },
    )
    .await
    .expect("create admin user");

    let viewer = UserModel::create_with_password(
        db,
        &RegisterParams {
            email: "viewer@example.com".to_string(),
            password: "viewer-password-123".to_string(),
            name: "Viewer".to_string(),
        },
    )
    .await
    .expect("create viewer user");

    let org = OrganizationModel::create(
        db,
        &CreateOrganizationParams {
            name: "Moonships Org".to_string(),
            slug: Some("moonships-org".to_string()),
        },
    )
    .await
    .expect("create organization");

    MembershipModel::upsert(
        db,
        org.id,
        &SetMembershipParams {
            user_id: owner.id,
            role: "owner".to_string(),
            is_active: Some(true),
        },
    )
    .await
    .expect("owner membership");

    MembershipModel::upsert(
        db,
        org.id,
        &SetMembershipParams {
            user_id: admin.id,
            role: "admin".to_string(),
            is_active: Some(true),
        },
    )
    .await
    .expect("admin membership");

    MembershipModel::upsert(
        db,
        org.id,
        &SetMembershipParams {
            user_id: viewer.id,
            role: "viewer".to_string(),
            is_active: Some(true),
        },
    )
    .await
    .expect("viewer membership");

    assert_eq!(
        MembershipModel::active_owner_count(db, org.id)
            .await
            .expect("owner count"),
        1
    );

    let owner_principal = Principal {
        user: owner.clone(),
        actor_kind: "jwt",
        actor_id: owner.pid.to_string(),
        api_token: None,
        request_id: "test-owner-request".to_string(),
        confirmation: Some("confirmed".to_string()),
    };
    assert!(owner_principal
        .require(db, org.id, Permission::Owner)
        .await
        .is_ok());

    let viewer_principal = Principal {
        user: viewer.clone(),
        actor_kind: "jwt",
        actor_id: viewer.pid.to_string(),
        api_token: None,
        request_id: "test-viewer-request".to_string(),
        confirmation: None,
    };
    assert!(viewer_principal
        .require(db, org.id, Permission::View)
        .await
        .is_ok());
    assert!(
        viewer_principal
            .require(db, org.id, Permission::Deploy)
            .await
            .is_err(),
        "viewer must not receive deploy permission"
    );

    let other_org = OrganizationModel::create(
        db,
        &CreateOrganizationParams {
            name: "Other Org".to_string(),
            slug: Some("other-org".to_string()),
        },
    )
    .await
    .expect("create second organization");
    assert!(
        viewer_principal
            .require(db, other_org.id, Permission::View)
            .await
            .is_err(),
        "cross-organization access must fail closed"
    );

    let created = ApiTokenModel::create_token(
        db,
        owner.id,
        &CreateApiTokenParams {
            organization_id: org.id,
            name: "deploy token".to_string(),
            scopes: vec!["read".to_string(), "deploy".to_string()],
            expires_in_days: Some(30),
        },
    )
    .await
    .expect("create API token");

    let raw_token = created.token.clone();
    let safe_json =
        serde_json::to_string(&created.record.to_safe()).expect("serialize safe API token");
    assert!(!safe_json.contains(&created.record.token_hash));
    assert!(!safe_json.contains(&raw_token));
    assert_eq!(
        ApiTokenModel::authenticate(db, &raw_token)
            .await
            .expect("authenticate token")
            .expect("token accepted")
            .id,
        created.record.id
    );

    let api_principal = Principal {
        user: owner.clone(),
        actor_kind: "api_token",
        actor_id: created.record.id.to_string(),
        api_token: Some(created.record.clone()),
        request_id: "api-token-request".to_string(),
        confirmation: None,
    };
    assert!(api_principal
        .require(db, org.id, Permission::Deploy)
        .await
        .is_ok());
    assert!(
        api_principal
            .require(db, org.id, Permission::ManageOrganization)
            .await
            .is_err(),
        "scoped deploy token must not gain organization-admin permission"
    );

    ApiTokenModel::revoke_for_organization(db, created.record.id, org.id)
        .await
        .expect("revoke organization API token");
    assert!(ApiTokenModel::authenticate(db, &raw_token)
        .await
        .expect("authenticate revoked token")
        .is_none());

    let previous_session_version = owner.session_version;
    let revoked_user = UserModel::revoke_sessions(db, owner.id)
        .await
        .expect("revoke JWT sessions");
    assert_eq!(
        revoked_user.session_version,
        previous_session_version.saturating_add(1)
    );

    AuditEventModel::append(
        db,
        AuditEventInput {
            organization_id: Some(org.id),
            actor_kind: "jwt".to_string(),
            actor_id: owner.pid.to_string(),
            action: "test.destructive".to_string(),
            resource_type: Some("test".to_string()),
            resource_id: Some("1".to_string()),
            outcome: "success".to_string(),
            request_id: Some("audit-request-1".to_string()),
            metadata: owner_principal.audit_metadata(None),
        },
    )
    .await
    .expect("append audit event");
    let events = AuditEventModel::list_for_organization(db, org.id, 10)
        .await
        .expect("list audit events");
    assert!(events.iter().any(|event| {
        event.request_id.as_deref() == Some("audit-request-1") && event.action == "test.destructive"
    }));

    for attempt in 0..3 {
        let decision = AuthRateLimitModel::check_and_record(db, "v07-test", "same-key", 2, 60, 60)
            .await
            .expect("rate limit decision");
        if attempt < 2 {
            assert!(decision.allowed);
        } else {
            assert!(!decision.allowed);
        }
    }

    let _unused = LoginParams {
        email: owner.email.clone(),
        password: "owner-password-123".to_string(),
    };
}

#[tokio::test]
#[serial]
async fn test_active_deployment_claim_is_atomic_under_concurrency() {
    let boot = boot_test::<App>()
        .await
        .expect("Failed to boot test application");
    let db = &boot.app_context.db;

    let server = servers::ActiveModel {
        name: Set("CAS Worker Host".to_string()),
        host: Set("10.0.0.22".to_string()),
        port: Set(22),
        username: Set("deploy".to_string()),
        authentication_type: Set("ssh_key".to_string()),
        encrypted_private_key: Set(None),
        status: Set("online".to_string()),
        created_at: Set(Utc::now().into()),
        updated_at: Set(Utc::now().into()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("insert CAS server");

    let project = ProjectModel::create_project(
        db,
        &CreateProjectParams {
            name: "CAS Project".to_string(),
            slug: Some("cas-project".to_string()),
            description: None,
        },
    )
    .await
    .expect("create CAS project");

    let environment = EnvironmentModel::create_environment(
        db,
        &CreateEnvironmentParams {
            project_id: project.id,
            name: "CAS".to_string(),
            slug: Some("cas".to_string()),
            description: None,
        },
    )
    .await
    .expect("create CAS environment");

    let application = ApplicationModel::create_application(
        db,
        &CreateApplicationParams {
            project_id: project.id,
            environment_id: environment.id,
            server_id: server.id,
            server_pool_id: None,
            resource_units: Some(1),
            name: "CAS App".to_string(),
            slug: Some("cas-app".to_string()),
            git_repository: "https://example.com/cas.git".to_string(),
            git_branch: Some("main".to_string()),
            build_type: Some("dockerfile".to_string()),
            workload_type: Some("single".to_string()),
            compose_file_path: None,
            compose_project_name: None,
            registry_credential_id: None,
            dockerfile_path: Some("Dockerfile".to_string()),
            docker_context: Some(".".to_string()),
            docker_image: None,
            container_name: Some("cas-app".to_string()),
            container_port: Some(8080),
            published_port: None,
            startup_command: None,
            healthcheck_path: None,
            healthcheck_port: None,
            auto_deploy: Some(false),
        },
    )
    .await
    .expect("create CAS application");

    let mut handles = Vec::new();
    for candidate in 10_001_i64..10_033_i64 {
        let worker_db: DatabaseConnection = db.clone();
        let application_id = application.id;
        handles.push(tokio::spawn(async move {
            ApplicationModel::claim_active_deployment(&worker_db, application_id, candidate)
                .await
                .expect("atomic deployment claim")
        }));
    }

    let mut winners = 0usize;
    for handle in handles {
        if handle.await.expect("claim task") {
            winners += 1;
        }
    }

    assert_eq!(
        winners, 1,
        "exactly one concurrent worker may own the application deployment lock"
    );
    let locked = ApplicationModel::find_by_id(db, application.id)
        .await
        .expect("reload CAS application");
    assert!(locked.active_deployment_id.is_some());
}
