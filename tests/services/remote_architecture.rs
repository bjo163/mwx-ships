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

#[test]
fn source_inspection_stays_on_remote_boundary() {
    let inspection = include_str!("../../src/services/source_inspection.rs");
    let remote = include_str!("../../src/services/remote.rs");
    let controller = include_str!("../../src/controllers/source_inspection.rs");

    assert!(
        !inspection.contains("tokio::process::Command")
            && !inspection.contains("std::process::Command")
            && !controller.contains("tokio::process::Command")
            && !controller.contains("std::process::Command"),
        "source inspection must not clone or execute repository tooling on the control-plane host"
    );
    assert!(controller.contains("RemoteRuntime"));
    assert!(remote.contains("source_inspection_clone"));
    assert!(remote.contains("GIT_TERMINAL_PROMPT=0"));
    assert!(remote.contains("GIT_LFS_SKIP_SMUDGE=1"));
    assert!(remote.contains("[ ! -L"));
    assert!(remote.contains("65536"));
}

#[test]
fn managed_services_stay_on_remote_runtime_boundary() {
    let controller = include_str!("../../src/controllers/managed_services.rs");
    let templates = include_str!("../../src/services/managed_service.rs");
    let remote = include_str!("../../src/services/remote.rs");

    assert!(
        !controller.contains("tokio::process::Command")
            && !controller.contains("std::process::Command")
            && !templates.contains("tokio::process::Command")
            && !templates.contains("std::process::Command"),
        "managed service lifecycle must not execute Docker on the control-plane host"
    );
    assert!(controller.contains("RemoteRuntime"));
    assert!(remote.contains("run_managed_service_container"));
    assert!(remote.contains("managed_service_ready"));
    assert!(controller.contains("redact_secrets"));
    assert!(
        !controller.contains("encrypted_credentials\":"),
        "managed service API responses must not serialize stored credential ciphertext"
    );
}

#[test]
fn stateful_backup_stays_on_remote_boundary_and_redacts_errors() {
    let controller = include_str!("../../src/controllers/managed_services.rs");
    let backup = include_str!("../../src/services/managed_service_backup.rs");
    let remote = include_str!("../../src/services/remote.rs");

    assert!(
        !controller.contains("tokio::process::Command")
            && !controller.contains("std::process::Command")
            && !backup.contains("tokio::process::Command")
            && !backup.contains("std::process::Command"),
        "stateful backup orchestration must not execute snapshot tooling on the control-plane host"
    );
    assert!(remote.contains("snapshot_volume"));
    assert!(remote.contains("restore_volume_snapshot"));
    assert!(remote.contains("volume_snapshot_metadata"));
    assert!(controller.contains("redact_secrets"));
    assert!(controller.contains("x-moonships-confirmation"));
    assert!(backup.contains("alpine:3.22.2"));
    assert!(backup.contains("tag.eq_ignore_ascii_case(\"latest\")"));
    assert!(backup.contains("backup helper image cannot use an empty or latest tag"));
}
