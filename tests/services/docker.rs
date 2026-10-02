use moonships::services::docker::DockerService;

#[test]
fn test_valid_container_names() {
    assert!(DockerService::validate_container_name("moonships-app-1").is_ok());
    assert!(DockerService::validate_container_name("web.prod_v2").is_ok());
    assert!(DockerService::validate_container_name("my_service-123").is_ok());
}

#[test]
fn test_rejects_dangerous_container_names() {
    assert!(DockerService::validate_container_name("app; rm -rf /").is_err());
    assert!(DockerService::validate_container_name("app name with spaces").is_err());
    assert!(DockerService::validate_container_name("app$(whoami)").is_err());
    assert!(DockerService::validate_container_name("").is_err());
}

#[test]
fn test_image_name_validation() {
    assert!(DockerService::validate_image_name("nginx:alpine").is_ok());
    assert!(DockerService::validate_image_name("ghcr.io/org/repo:sha-123").is_ok());
    assert!(DockerService::validate_image_name("redis:7-bullseye").is_ok());

    assert!(DockerService::validate_image_name("nginx; whoami").is_err());
    assert!(DockerService::validate_image_name("nginx | cat").is_err());
    assert!(DockerService::validate_image_name("").is_err());
}
