use loco_rs::testing::prelude::*;
use moonships::app::App;
use serial_test::serial;

use super::prepare_data;

macro_rules! authed {
    ($builder:expr, $token:expr) => {{
        let (key, value) = prepare_data::auth_header($token);
        $builder.add_header(key, value)
    }};
}

#[tokio::test]
#[serial]
async fn test_health_endpoint() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/api/health").await;
        assert_eq!(res.status_code(), 200);
        let body: serde_json::Value = serde_json::from_str(&res.text()).unwrap();
        assert_eq!(body["data"]["status"], "healthy");
        assert_eq!(body["data"]["database"], "sqlite");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn test_management_api_requires_authentication() {
    request::<App, _, _>(|request, _ctx| async move {
        for path in [
            "/api/servers",
            "/api/projects",
            "/api/applications",
            "/api/deployments",
        ] {
            let res = request.get(path).await;
            assert_eq!(
                res.status_code(),
                401,
                "management endpoint {path} must reject unauthenticated requests"
            );
        }

        let health = request.get("/api/health").await;
        assert_eq!(health.status_code(), 200, "health endpoint stays public");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn test_full_api_workflow_and_security() {
    std::env::set_var(
        "ENCRYPTION_KEY",
        "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
    );

    request::<App, _, _>(|request, ctx| async move {
        let login = prepare_data::init_user_login(&request, &ctx).await;

        let organization_payload = serde_json::json!({
            "name": "E2E Test Organization",
            "slug": "e2e-test-org"
        });
        let res = authed!(
            request
                .post("/api/organizations")
                .json(&organization_payload),
            &login.token
        )
        .await;
        assert_eq!(res.status_code(), 200);
        let organization_res: serde_json::Value =
            serde_json::from_str(&res.text()).unwrap();
        assert!(
            organization_res["data"]["organization"]["id"].as_i64().is_some(),
            "authenticated user must become owner of the organization"
        );

        let server_payload = serde_json::json!({
            "name": "Integration Test Server",
            "host": "192.168.1.100",
            "port": 22,
            "username": "deploy",
            "authentication_type": "ssh_key",
            "private_key": "-----BEGIN OPENSSH PRIVATE KEY-----\ntest-secret-key\n-----END OPENSSH PRIVATE KEY-----"
        });

        let res = authed!(request.post("/api/servers").json(&server_payload), &login.token).await;
        assert_eq!(res.status_code(), 200);
        let server_res: serde_json::Value = serde_json::from_str(&res.text()).unwrap();
        let server_id = server_res["data"]["id"].as_i64().expect("server id");

        let res = authed!(request.get("/api/servers"), &login.token).await;
        assert_eq!(res.status_code(), 200);
        let servers_list: serde_json::Value = serde_json::from_str(&res.text()).unwrap();
        assert!(servers_list["data"].is_array());
        for s in servers_list["data"].as_array().unwrap() {
            assert!(s.get("private_key").is_none(), "private_key must NEVER be exposed");
            assert!(
                s.get("encrypted_private_key").is_none(),
                "encrypted_private_key must NEVER be exposed"
            );
        }

        let project_payload = serde_json::json!({
            "name": "E2E Test Project",
            "description": "Integration testing project"
        });
        let res = authed!(request.post("/api/projects").json(&project_payload), &login.token).await;
        assert_eq!(res.status_code(), 200);
        let project_res: serde_json::Value = serde_json::from_str(&res.text()).unwrap();
        let project_id = project_res["data"]["id"].as_i64().expect("project id");

        let env_payload = serde_json::json!({ "name": "Production" });
        let res = authed!(
            request
                .post(&format!("/api/projects/{}/environments", project_id))
                .json(&env_payload),
            &login.token
        )
        .await;
        assert_eq!(res.status_code(), 200);
        let env_res: serde_json::Value = serde_json::from_str(&res.text()).unwrap();
        let env_id = env_res["data"]["id"].as_i64().expect("env id");

        let app_payload = serde_json::json!({
            "name": "Demo Node App",
            "slug": "demo-node-app",
            "project_id": project_id,
            "environment_id": env_id,
            "server_id": server_id,
            "build_type": "dockerfile",
            "git_repository": "https://github.com/bjo163/mwx-ships.git",
            "git_branch": "main",
            "container_port": 3000
        });
        let res = authed!(
            request.post("/api/applications").json(&app_payload),
            &login.token
        )
        .await;
        assert_eq!(res.status_code(), 200);
        let app_res: serde_json::Value = serde_json::from_str(&res.text()).unwrap();
        let app_id = app_res["data"]["id"].as_i64().expect("app id");

        let secret_env_payload = serde_json::json!({
            "key": "DATABASE_PASSWORD",
            "value": "SuperSecretPassword123!",
            "is_secret": true
        });
        let res = authed!(
            request
                .post(&format!("/api/applications/{}/environment", app_id))
                .json(&secret_env_payload),
            &login.token
        )
        .await;
        assert_eq!(res.status_code(), 200);

        let res = authed!(
            request.get(&format!("/api/applications/{}/environment", app_id)),
            &login.token
        )
        .await;
        assert_eq!(res.status_code(), 200);
        let envs_res: serde_json::Value = serde_json::from_str(&res.text()).unwrap();
        let env_vars = envs_res["data"].as_array().expect("env vars array");
        assert_eq!(env_vars.len(), 1);
        assert_eq!(env_vars[0]["key"], "DATABASE_PASSWORD");
        assert_eq!(env_vars[0]["value"], "********");

        let domain_payload = serde_json::json!({
            "hostname": "app.moonships.io",
            "port": 80,
            "https_enabled": true
        });
        let res = authed!(
            request
                .post(&format!("/api/applications/{}/domains", app_id))
                .json(&domain_payload),
            &login.token
        )
        .await;
        assert_eq!(res.status_code(), 200);

        let res = authed!(request.get("/api/deployments"), &login.token).await;
        assert_eq!(res.status_code(), 200);
        let dep_list: serde_json::Value = serde_json::from_str(&res.text()).unwrap();
        assert!(dep_list["data"].is_array());

        let deploy_payload = serde_json::json!({
            "commit_message": "Automated test deployment"
        });
        let res = authed!(
            request
                .post(&format!("/api/applications/{}/deploy", app_id))
                .json(&deploy_payload),
            &login.token
        )
        .await;
        assert_eq!(res.status_code(), 202);
        let deploy_res: serde_json::Value = serde_json::from_str(&res.text()).unwrap();
        assert_eq!(deploy_res["data"]["status"], "queued");
    })
    .await;
}
