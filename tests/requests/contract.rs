use loco_rs::testing::prelude::*;
use moonships::app::App;
use serde::Deserialize;
use serial_test::serial;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
struct OpenApi {
    paths: BTreeMap<String, BTreeMap<String, serde_json::Value>>,
}

#[tokio::test]
#[serial]
async fn v1_openapi_protected_routes_exist_and_reject_unauthenticated_requests() {
    let spec: OpenApi =
        serde_json::from_str(include_str!("../../docs/openapi-v1.json")).expect("valid v1 OpenAPI");

    let protected = spec
        .paths
        .iter()
        .flat_map(|(path, methods)| {
            methods.iter().filter_map(move |(method, operation)| {
                operation
                    .get("security")
                    .map(|_| (method.clone(), concrete_path(path)))
            })
        })
        .collect::<Vec<_>>();

    assert!(
        protected.len() >= 60,
        "v1 contract unexpectedly lost protected operations"
    );

    request::<App, _, _>(|request, _ctx| async move {
        for (method, path) in protected {
            let request_path = if method == "delete" && path == "/api/volumes/1" {
                format!("{path}?confirm=TEST_VOLUME")
            } else {
                path.clone()
            };
            let response = match method.as_str() {
                "get" => request.get(&request_path).await,
                "post" => request.post(&request_path).json(&serde_json::json!({})).await,
                "put" => request.put(&request_path).json(&serde_json::json!({})).await,
                "delete" => request.delete(&request_path).await,
                other => panic!("unsupported contract method {other}"),
            };
            let status = response.status_code();
            assert!(
                status == 401 || status == 422,
                "{method} {path} must exist and reject unauthenticated/invalid requests; got {status}"
            );
        }

        let health = request.get("/api/health").await;
        assert_eq!(
            health.status_code(),
            200,
            "public health route must remain available"
        );
    })
    .await;
}

#[test]
fn v1_openapi_contract_has_unique_operation_ids_and_expected_public_surfaces() {
    let spec: OpenApi =
        serde_json::from_str(include_str!("../../docs/openapi-v1.json")).expect("valid v1 OpenAPI");

    let mut ids = std::collections::BTreeSet::new();
    let mut operation_count = 0usize;
    for methods in spec.paths.values() {
        for operation in methods.values() {
            operation_count += 1;
            let id = operation["operationId"]
                .as_str()
                .expect("operationId required in frozen contract");
            assert!(ids.insert(id.to_string()), "duplicate operationId {id}");
        }
    }

    assert!(operation_count >= 70, "v1 API contract unexpectedly shrank");
    assert!(spec.paths.contains_key("/api/health"));
    assert!(spec
        .paths
        .contains_key("/api/webhooks/{provider}/{application_id}"));
    assert!(spec.paths.contains_key("/api/organizations/{id}/audit"));
    assert!(spec.paths.contains_key("/api/applications/{id}/rollback"));
}

fn concrete_path(template: &str) -> String {
    template
        .replace("{id}", "1")
        .replace("{domain_id}", "1")
        .replace("{revision_id}", "1")
        .replace("{token_id}", "1")
        .replace("{pool_id}", "1")
        .replace("{credential_id}", "1")
        .replace("{attachment_id}", "1")
        .replace("{backup_id}", "1")
        .replace("{key}", "TEST_KEY")
        .replace("{provider}", "github")
        .replace("{application_id}", "1")
        .replace("{token}", "invalid-token")
}
