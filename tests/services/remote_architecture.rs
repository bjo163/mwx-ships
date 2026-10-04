#[test]
fn deployment_and_lifecycle_do_not_use_local_docker_daemon() {
    let deployment = include_str!("../../src/services/deployment.rs");
    let applications = include_str!("../../src/controllers/applications.rs");

    assert!(
        !deployment.contains("DockerService::"),
        "deployment orchestration must not execute Docker on the control-plane host"
    );
    assert!(
        !applications.contains("DockerService::"),
        "application lifecycle endpoints must target the selected remote server"
    );
    assert!(deployment.contains("RemoteRuntime"));
    assert!(applications.contains("RemoteRuntime"));
}

#[test]
fn production_compose_does_not_mount_docker_socket() {
    let compose = include_str!("../../docker-compose.yml");
    assert!(
        !compose.contains("/var/run/docker.sock"),
        "v0.2 control plane must not require the host Docker socket"
    );
}

#[test]
fn remote_executor_contains_secret_redaction_and_host_pinning() {
    let remote = include_str!("../../src/services/remote.rs");
    let ssh = include_str!("../../src/services/ssh.rs");

    assert!(remote.contains("redact_secrets"));
    assert!(ssh.contains("known_host_fingerprint"));
    assert!(ssh.contains("StrictHostKeyChecking=yes"));
}
