use loco_rs::testing::prelude::*;
use moonships::app::App;
use serial_test::serial;

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
async fn test_full_api_workflow_and_security() {
    request::<App, _, _>(|request, _ctx| async move {
        // 1. Create a server with private key
        let server_payload = serde_json::json!({
            "name": "Integration Test Server",
            "host": "192.168.1.100",
            "port": 22,
            "username": "deploy",
            "authentication_type": "ssh_key",
            "private_key": "-----BEGIN OPENSSH PRIVATE KEY-----\ntest-secret-key\n-----END OPENSSH PRIVATE KEY-----"
        });

        let res = request.post("/api/servers").json(&server_payload).await;
        assert_eq!(res.status_code(), 200);
        let server_res: serde_json::Value = serde_json::from_str(&res.text()).unwrap();
        let server_id = server_res["data"]["id"].as_i64().expect("server id");

        // Verify GET /api/servers does NOT expose private_key
        let res = request.get("/api/servers").await;
        assert_eq!(res.status_code(), 200);
        let servers_list: serde_json::Value = serde_json::from_str(&res.text()).unwrap();
        assert!(servers_list["data"].is_array());
        for s in servers_list["data"].as_array().unwrap() {
            assert!(s.get("private_key").is_none(), "private_key must NEVER be exposed in server list");
        }

        // 2. Create a project
        let project_payload = serde_json::json!({
            "name": "E2E Test Project",
            "description": "Integration testing project"
        });
        let res = request.post("/api/projects").json(&project_payload).await;
        assert_eq!(res.status_code(), 200);
        let project_res: serde_json::Value = serde_json::from_str(&res.text()).unwrap();
        let project_id = project_res["data"]["id"].as_i64().expect("project id");

        // Create an environment under the project
        let env_payload = serde_json::json!({
            "name": "Production"
        });
        let res = request
            .post(&format!("/api/projects/{}/environments", project_id))
            .json(&env_payload)
            .await;
        assert_eq!(res.status_code(), 200);
        let env_res: serde_json::Value = serde_json::from_str(&res.text()).unwrap();
        let env_id = env_res["data"]["id"].as_i64().expect("env id");

        // 3. Create an application
        let app_payload = serde_json::json!({
            "name": "Demo Node App",
            "slug": "demo-node-app",
            "project_id": project_id,
            "environment_id": env_id,
            "server_id": server_id,
            "build_type": "dockerfile",
            "git_repository": "https://github.com/bjo163/mwx-ships.git",
            "git_branch": "master",
            "container_port": 3000
        });
        let res = request.post("/api/applications").json(&app_payload).await;
        assert_eq!(res.status_code(), 200);
        let app_res: serde_json::Value = serde_json::from_str(&res.text()).unwrap();
        let app_id = app_res["data"]["id"].as_i64().expect("app id");

        // 4. Secret Masking Test: Add secret environment variable
        let secret_env_payload = serde_json::json!({
            "key": "DATABASE_PASSWORD",
            "value": "SuperSecretPassword123!",
            "is_secret": true
        });
        let res = request
            .post(&format!("/api/applications/{}/environment", app_id))
            .json(&secret_env_payload)
            .await;
        assert_eq!(res.status_code(), 200);

        // Fetch environment variables and verify masking
        let res = request
            .get(&format!("/api/applications/{}/environment", app_id))
            .await;
        assert_eq!(res.status_code(), 200);
        let envs_res: serde_json::Value = serde_json::from_str(&res.text()).unwrap();
        let env_vars = envs_res["data"].as_array().expect("env vars array");
        assert_eq!(env_vars.len(), 1);
        assert_eq!(env_vars[0]["key"], "DATABASE_PASSWORD");
        assert_eq!(
            env_vars[0]["value"], "********",
            "secret environment variable values must be masked in API responses"
        );

        // 5. Add Domain to application
        let domain_payload = serde_json::json!({
            "hostname": "app.moonships.io",
            "port": 80,
            "https_enabled": true
        });
        let res = request
            .post(&format!("/api/applications/{}/domains", app_id))
            .json(&domain_payload)
            .await;
        assert_eq!(res.status_code(), 200);

        // 6. Verify Deployments list endpoint
        let res = request.get("/api/deployments").await;
        assert_eq!(res.status_code(), 200);
        let dep_list: serde_json::Value = serde_json::from_str(&res.text()).unwrap();
        assert!(dep_list["data"].is_array());

        // 7. Trigger Deployment -> 202 Accepted
        let deploy_payload = serde_json::json!({
            "commit_message": "Automated test deployment"
        });
        let res = request
            .post(&format!("/api/applications/{}/deploy", app_id))
            .json(&deploy_payload)
            .await;
        assert_eq!(
            res.status_code(),
            202,
            "Deployment trigger must return 202 Accepted"
        );
        let deploy_res: serde_json::Value = serde_json::from_str(&res.text()).unwrap();
        assert_eq!(deploy_res["data"]["status"], "queued");
    })
    .await;
}
