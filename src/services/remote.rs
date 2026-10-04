use crate::{
    models::{applications, servers},
    services::{
        docker::{ContainerConfig, DockerService},
        ssh::{SshError, SshService, SshSession},
    },
};
use std::{
    path::{Component, Path},
    time::Duration,
};

#[derive(Debug, thiserror::Error)]
pub enum RemoteError {
    #[error(transparent)]
    Ssh(#[from] SshError),
    #[error("Remote validation failed: {0}")]
    Validation(String),
    #[error("Remote command failed ({operation}, exit {exit_code}): {message}")]
    CommandFailed {
        operation: String,
        exit_code: i32,
        message: String,
    },
}

impl RemoteError {
    pub fn exit_code(&self) -> Option<i32> {
        match self {
            Self::CommandFailed { exit_code, .. } => Some(*exit_code),
            _ => None,
        }
    }
}

pub struct RemoteRuntime {
    session: SshSession,
}

impl RemoteRuntime {
    pub async fn connect(server: &servers::Model) -> Result<Self, RemoteError> {
        Ok(Self {
            session: SshService::connect(server).await?,
        })
    }

    pub fn target(&self) -> String {
        self.session.target()
    }

    pub async fn ensure_docker(&self) -> Result<String, RemoteError> {
        self.exec_checked(
            "docker_preflight",
            "docker info >/dev/null 2>&1 && docker --version",
            Duration::from_secs(30),
        )
        .await
    }

    pub async fn sync_repository(
        &self,
        app: &applications::Model,
        requested_commit: Option<&str>,
    ) -> Result<(String, String), RemoteError> {
        self.sync_repository_config(
            app.id,
            &app.git_repository,
            &app.git_branch,
            requested_commit,
        )
        .await
    }

    pub async fn sync_repository_config(
        &self,
        application_id: i64,
        git_repository: &str,
        git_branch: &str,
        requested_commit: Option<&str>,
    ) -> Result<(String, String), RemoteError> {
        if git_repository.trim().is_empty() || git_branch.trim().is_empty() {
            return Err(RemoteError::Validation(
                "git repository and branch must not be empty".to_string(),
            ));
        }

        let workspace = Self::workspace_expr(application_id);
        let branch = shell_quote(git_branch);
        let remote_branch = shell_quote(&format!("origin/{git_branch}"));
        let repository = shell_quote(git_repository);

        let sync_command = format!(
            "mkdir -p \"$HOME/.moonships/apps\" && \
             if [ -d {workspace}/.git ]; then \
               git -C {workspace} fetch --prune origin {branch} && \
               git -C {workspace} checkout -B {branch} {remote_branch}; \
             else \
               rm -rf {workspace} && \
               git clone --single-branch --branch {branch} {repository} {workspace}; \
             fi"
        );
        self.exec_checked("git_sync", &sync_command, Duration::from_secs(600))
            .await?;

        if let Some(commit) = requested_commit {
            validate_git_commit(commit)?;
            self.exec_checked(
                "git_checkout_commit",
                &format!(
                    "git -C {workspace} checkout --detach {}",
                    shell_quote(commit)
                ),
                Duration::from_secs(60),
            )
            .await?;
        }

        let commit_sha = self
            .exec_checked(
                "git_revision",
                &format!("git -C {workspace} rev-parse HEAD"),
                Duration::from_secs(30),
            )
            .await?
            .trim()
            .to_string();

        let commit_message = self
            .exec_checked(
                "git_message",
                &format!("git -C {workspace} log -1 --pretty=%s"),
                Duration::from_secs(30),
            )
            .await?
            .trim()
            .to_string();

        if commit_sha.is_empty() {
            return Err(RemoteError::Validation(
                "remote git checkout produced no commit SHA".to_string(),
            ));
        }

        Ok((commit_sha, commit_message))
    }

    pub async fn build_image(
        &self,
        app: &applications::Model,
        image_tag: &str,
    ) -> Result<String, RemoteError> {
        self.build_image_config(app.id, &app.docker_context, &app.dockerfile_path, image_tag)
            .await
    }

    pub async fn build_image_config(
        &self,
        application_id: i64,
        docker_context: &str,
        dockerfile_path: &str,
        image_tag: &str,
    ) -> Result<String, RemoteError> {
        DockerService::validate_image_name(image_tag)
            .map_err(|e| RemoteError::Validation(e.to_string()))?;
        validate_relative_path(docker_context)?;
        validate_relative_path(dockerfile_path)?;

        let dockerfile = if docker_context == "." {
            dockerfile_path.to_string()
        } else {
            format!(
                "{}/{}",
                docker_context.trim_end_matches('/'),
                dockerfile_path.trim_start_matches("./")
            )
        };

        let command = format!(
            "cd {} && docker build -t {} -f {} {}",
            Self::workspace_expr(application_id),
            shell_quote(image_tag),
            shell_quote(&dockerfile),
            shell_quote(docker_context)
        );

        self.exec_checked("docker_build", &command, Duration::from_secs(1200))
            .await
    }

    pub async fn remove_image(&self, image: &str) -> Result<(), RemoteError> {
        DockerService::validate_image_name(image)
            .map_err(|e| RemoteError::Validation(e.to_string()))?;
        self.exec_checked(
            "docker_image_remove",
            &format!(
                "docker image rm {} >/dev/null 2>&1 || true",
                shell_quote(image)
            ),
            Duration::from_secs(60),
        )
        .await?;
        Ok(())
    }

    pub async fn disk_available_gb(&self) -> Result<f64, RemoteError> {
        let output = self
            .exec_checked(
                "disk_available",
                "df -Pk \"$HOME\" | tail -1 | awk '{print $4}'",
                Duration::from_secs(30),
            )
            .await?;
        let kbytes = output.trim().parse::<f64>().map_err(|_| {
            RemoteError::Validation("unable to parse remote disk availability".to_string())
        })?;
        Ok((kbytes / 1024.0 / 1024.0 * 10.0).round() / 10.0)
    }

    pub async fn ensure_network(&self, network: &str) -> Result<(), RemoteError> {
        DockerService::validate_container_name(network)
            .map_err(|e| RemoteError::Validation(e.to_string()))?;
        let quoted = shell_quote(network);
        self.exec_checked(
            "docker_network_ensure",
            &format!(
                "docker network inspect {quoted} >/dev/null 2>&1 || docker network create {quoted} >/dev/null"
            ),
            Duration::from_secs(60),
        )
        .await?;
        Ok(())
    }

    pub async fn ensure_managed_ingress(
        &self,
        network: &str,
        proxy_name: &str,
        image: &str,
        acme_email: Option<&str>,
    ) -> Result<(), RemoteError> {
        DockerService::validate_container_name(network)
            .map_err(|e| RemoteError::Validation(e.to_string()))?;
        DockerService::validate_container_name(proxy_name)
            .map_err(|e| RemoteError::Validation(e.to_string()))?;
        DockerService::validate_image_name(image)
            .map_err(|e| RemoteError::Validation(e.to_string()))?;

        self.ensure_network(network).await?;

        let network = shell_quote(network);
        let proxy_name = shell_quote(proxy_name);
        let image = shell_quote(image);
        let mut args = vec![
            "docker run -d".to_string(),
            format!("--name {proxy_name}"),
            "--restart unless-stopped".to_string(),
            format!("--network {network}"),
            "-p 80:80".to_string(),
            "-p 443:443".to_string(),
            "-v \"$HOME/.moonships/traefik/dynamic:/etc/traefik/dynamic:ro\"".to_string(),
            "-v \"$HOME/.moonships/traefik/acme.json:/acme.json\"".to_string(),
            image,
            "--providers.file.directory=/etc/traefik/dynamic".to_string(),
            "--providers.file.watch=true".to_string(),
            "--entrypoints.web.address=:80".to_string(),
            "--entrypoints.websecure.address=:443".to_string(),
        ];

        if let Some(email) = acme_email.filter(|value| !value.trim().is_empty()) {
            args.push(format!(
                "--certificatesresolvers.letsencrypt.acme.email={}",
                shell_quote(email.trim())
            ));
            args.push(
                "--certificatesresolvers.letsencrypt.acme.storage=/acme.json".to_string(),
            );
            args.push(
                "--certificatesresolvers.letsencrypt.acme.httpchallenge.entrypoint=web".to_string(),
            );
        }

        let command = format!(
            "mkdir -p \"$HOME/.moonships/traefik/dynamic\"; \
             touch \"$HOME/.moonships/traefik/acme.json\"; chmod 600 \"$HOME/.moonships/traefik/acme.json\"; \
             if docker inspect {proxy_name} >/dev/null 2>&1; then \
               docker start {proxy_name} >/dev/null 2>&1 || true; \
               docker network connect {network} {proxy_name} >/dev/null 2>&1 || true; \
             else \
               docker pull {image} >/dev/null && {}; \
             fi",
            args.join(" ")
        );

        self.exec_checked(
            "managed_ingress_ensure",
            &command,
            Duration::from_secs(600),
        )
        .await?;
        Ok(())
    }

    pub async fn write_managed_route(
        &self,
        application_id: i64,
        config_json: &str,
    ) -> Result<(), RemoteError> {
        let route_path = format!(
            "\"$HOME/.moonships/traefik/dynamic/app-{application_id}.json\""
        );
        let temp_path = format!(
            "\"$HOME/.moonships/traefik/dynamic/app-{application_id}.json.tmp\""
        );
        let command = format!(
            "umask 077; mkdir -p \"$HOME/.moonships/traefik/dynamic\"; \
             cat > {temp_path}; test -s {temp_path}; mv -f {temp_path} {route_path}"
        );
        let (code, _, stderr) = self
            .session
            .execute_with_input(&command, config_json)
            .await
            .map_err(RemoteError::Ssh)?;

        if code != 0 {
            return Err(RemoteError::CommandFailed {
                operation: "managed_route_write".to_string(),
                exit_code: code,
                message: stderr.trim().to_string(),
            });
        }
        Ok(())
    }

    pub async fn remove_managed_route(&self, application_id: i64) -> Result<(), RemoteError> {
        self.exec_checked(
            "managed_route_remove",
            &format!(
                "rm -f \"$HOME/.moonships/traefik/dynamic/app-{application_id}.json\""
            ),
            Duration::from_secs(30),
        )
        .await?;
        Ok(())
    }

    pub async fn verify_domain_target(
        &self,
        hostname: &str,
        target_host: &str,
    ) -> Result<bool, RemoteError> {
        if hostname.trim().is_empty() || target_host.trim().is_empty() {
            return Err(RemoteError::Validation(
                "hostname and target host must not be empty".to_string(),
            ));
        }
        let hostname = shell_quote(hostname);
        let target = shell_quote(target_host);
        let command = format!(
            "DOMAIN_IPS=$(getent ahostsv4 {hostname} 2>/dev/null | awk '{{print $1}}' | sort -u); \
             TARGET_IPS=$(getent ahostsv4 {target} 2>/dev/null | awk '{{print $1}}' | sort -u); \
             if [ -z \"$TARGET_IPS\" ]; then TARGET_IPS={target}; fi; \
             for ip in $DOMAIN_IPS; do for target_ip in $TARGET_IPS; do [ \"$ip\" = \"$target_ip\" ] && exit 0; done; done; exit 1"
        );
        let (code, _, _) = self
            .session
            .execute_with_timeout(&command, Duration::from_secs(20))
            .await?;
        Ok(code == 0)
    }

    pub async fn verify_tls(&self, hostname: &str) -> Result<bool, RemoteError> {
        if hostname.trim().is_empty() {
            return Err(RemoteError::Validation(
                "hostname must not be empty".to_string(),
            ));
        }
        let url = format!("https://{}/", hostname.trim());
        let (code, _, _) = self
            .session
            .execute_with_timeout(
                &format!(
                    "curl -fsSI --max-time 8 --resolve {}:443:127.0.0.1 {} >/dev/null",
                    shell_quote(hostname.trim()),
                    shell_quote(&url)
                ),
                Duration::from_secs(12),
            )
            .await?;
        Ok(code == 0)
    }

    pub async fn pull_image(&self, image: &str) -> Result<String, RemoteError> {
        DockerService::validate_image_name(image)
            .map_err(|e| RemoteError::Validation(e.to_string()))?;
        self.exec_checked(
            "docker_pull",
            &format!("docker pull {}", shell_quote(image)),
            Duration::from_secs(600),
        )
        .await
    }

    pub async fn stop_and_remove_container(&self, name: &str) -> Result<(), RemoteError> {
        DockerService::validate_container_name(name)
            .map_err(|e| RemoteError::Validation(e.to_string()))?;
        let quoted = shell_quote(name);
        self.exec_checked(
            "docker_replace",
            &format!(
                "docker stop {quoted} >/dev/null 2>&1 || true; docker rm -f {quoted} >/dev/null 2>&1 || true"
            ),
            Duration::from_secs(60),
        )
        .await?;
        Ok(())
    }

    pub async fn run_container(
        &self,
        application_id: i64,
        config: &ContainerConfig,
    ) -> Result<String, RemoteError> {
        DockerService::validate_container_name(&config.name)
            .map_err(|e| RemoteError::Validation(e.to_string()))?;
        DockerService::validate_image_name(&config.image)
            .map_err(|e| RemoteError::Validation(e.to_string()))?;

        let mut env_file = String::new();
        for (key, value) in &config.env_vars {
            validate_env_key(key)?;
            if value.contains('\n') || value.contains('\r') {
                return Err(RemoteError::Validation(format!(
                    "environment variable {key} contains a newline, which is unsupported"
                )));
            }
            env_file.push_str(key);
            env_file.push('=');
            env_file.push_str(value);
            env_file.push('\n');
        }

        let mut args = vec![
            "docker run -d".to_string(),
            format!("--name {}", shell_quote(&config.name)),
        ];

        if !config.restart_policy.is_empty() {
            args.push(format!("--restart {}", shell_quote(&config.restart_policy)));
        }
        if let Some(network) = &config.network {
            args.push(format!("--network {}", shell_quote(network)));
        }
        if let Some(published_port) = config.published_port {
            args.push(format!(
                "-p {}",
                shell_quote(&format!("{published_port}:{}", config.container_port))
            ));
        }

        args.push("--env-file \"$ENV_FILE\"".to_string());

        for (key, value) in &config.labels {
            args.push(format!("-l {}", shell_quote(&format!("{key}={value}"))));
        }
        args.push(shell_quote(&config.image));

        let env_path = format!("\"$HOME/.moonships/runtime/app-{application_id}.env\"");
        let command = format!(
            "umask 077; mkdir -p \"$HOME/.moonships/runtime\"; \
             ENV_FILE={env_path}; cat > \"$ENV_FILE\"; \
             {}; rc=$?; rm -f \"$ENV_FILE\"; exit $rc",
            args.join(" ")
        );

        let (code, stdout, stderr) = self
            .session
            .execute_with_input(&command, &env_file)
            .await
            .map_err(RemoteError::Ssh)?;

        if code != 0 {
            return Err(RemoteError::CommandFailed {
                operation: "docker_run".to_string(),
                exit_code: code,
                message: stderr.trim().to_string(),
            });
        }

        Ok(stdout.trim().to_string())
    }

    pub async fn start_container(&self, name: &str) -> Result<(), RemoteError> {
        self.container_action("docker_start", "start", name).await
    }

    pub async fn stop_container(&self, name: &str) -> Result<(), RemoteError> {
        self.container_action("docker_stop", "stop", name).await
    }

    pub async fn restart_container(&self, name: &str) -> Result<(), RemoteError> {
        self.container_action("docker_restart", "restart", name)
            .await
    }

    pub async fn remove_container(&self, name: &str) -> Result<(), RemoteError> {
        DockerService::validate_container_name(name)
            .map_err(|e| RemoteError::Validation(e.to_string()))?;
        self.exec_checked(
            "docker_remove",
            &format!("docker rm -f {}", shell_quote(name)),
            Duration::from_secs(60),
        )
        .await?;
        Ok(())
    }

    pub async fn container_status(&self, name: &str) -> Result<String, RemoteError> {
        DockerService::validate_container_name(name)
            .map_err(|e| RemoteError::Validation(e.to_string()))?;
        let (code, stdout, _) = self
            .session
            .execute_with_timeout(
                &format!(
                    "docker inspect --format '{{{{.State.Status}}}}' {} 2>/dev/null",
                    shell_quote(name)
                ),
                Duration::from_secs(30),
            )
            .await?;
        if code != 0 {
            return Ok("stopped".to_string());
        }
        Ok(stdout.trim().to_string())
    }

    pub async fn container_logs(&self, name: &str, tail: u32) -> Result<String, RemoteError> {
        DockerService::validate_container_name(name)
            .map_err(|e| RemoteError::Validation(e.to_string()))?;
        self.exec_checked(
            "docker_logs",
            &format!("docker logs --tail {tail} {} 2>&1", shell_quote(name)),
            Duration::from_secs(60),
        )
        .await
    }

    pub async fn healthcheck_once(
        &self,
        container_name: &str,
        published_port: Option<i32>,
        container_port: i32,
        path: &str,
    ) -> Result<(), RemoteError> {
        let normalized_path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{path}")
        };

        let (host, port) = if let Some(port) = published_port {
            ("127.0.0.1".to_string(), port)
        } else {
            DockerService::validate_container_name(container_name)
                .map_err(|e| RemoteError::Validation(e.to_string()))?;
            let ip = self
                .exec_checked(
                    "docker_health_target",
                    &format!(
                        "docker inspect --format '{{{{range .NetworkSettings.Networks}}}}{{{{.IPAddress}}}}{{{{end}}}}' {}",
                        shell_quote(container_name)
                    ),
                    Duration::from_secs(30),
                )
                .await?
                .trim()
                .to_string();
            if ip.is_empty() {
                return Err(RemoteError::Validation(
                    "container has no routable Docker network address".to_string(),
                ));
            }
            (ip, container_port)
        };

        let url = format!("http://{host}:{port}{normalized_path}");
        self.exec_checked(
            "healthcheck",
            &format!("curl -fsS --max-time 3 {}", shell_quote(&url)),
            Duration::from_secs(10),
        )
        .await?;
        Ok(())
    }

    async fn container_action(
        &self,
        operation: &str,
        action: &str,
        name: &str,
    ) -> Result<(), RemoteError> {
        DockerService::validate_container_name(name)
            .map_err(|e| RemoteError::Validation(e.to_string()))?;
        self.exec_checked(
            operation,
            &format!("docker {action} {}", shell_quote(name)),
            Duration::from_secs(60),
        )
        .await?;
        Ok(())
    }

    async fn exec_checked(
        &self,
        operation: &str,
        command: &str,
        timeout: Duration,
    ) -> Result<String, RemoteError> {
        let (code, stdout, stderr) = self
            .session
            .execute_with_timeout(command, timeout)
            .await
            .map_err(RemoteError::Ssh)?;
        if code != 0 {
            return Err(RemoteError::CommandFailed {
                operation: operation.to_string(),
                exit_code: code,
                message: stderr.trim().to_string(),
            });
        }
        Ok(stdout)
    }

    fn workspace_expr(application_id: i64) -> String {
        format!("\"$HOME/.moonships/apps/app-{application_id}\"")
    }
}

pub fn redact_secrets(input: &str, secrets: &[String]) -> String {
    let mut values: Vec<&String> = secrets.iter().filter(|value| !value.is_empty()).collect();
    values.sort_by_key(|value| std::cmp::Reverse(value.len()));

    let mut output = input.to_string();
    for value in values {
        output = output.replace(value.as_str(), "[REDACTED]");
    }
    output
}

pub fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn validate_relative_path(value: &str) -> Result<(), RemoteError> {
    let path = Path::new(value);
    if value.trim().is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(RemoteError::Validation(format!(
            "unsafe relative path '{value}'"
        )));
    }
    Ok(())
}

fn validate_git_commit(value: &str) -> Result<(), RemoteError> {
    let valid = (7..=64).contains(&value.len()) && value.chars().all(|c| c.is_ascii_hexdigit());
    if !valid {
        return Err(RemoteError::Validation(format!(
            "invalid requested Git commit '{value}'"
        )));
    }
    Ok(())
}

fn validate_env_key(key: &str) -> Result<(), RemoteError> {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return Err(RemoteError::Validation(
            "environment variable key cannot be empty".to_string(),
        ));
    };
    if !(first.is_ascii_alphabetic() || first == '_')
        || !chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(RemoteError::Validation(format!(
            "invalid environment variable key '{key}'"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_quote_blocks_command_substitution() {
        let quoted = shell_quote("value'; rm -rf /; echo '");
        assert_eq!(quoted, "'value'\"'\"'; rm -rf /; echo '\"'\"''");
    }

    #[test]
    fn rejects_path_traversal() {
        assert!(validate_relative_path(".").is_ok());
        assert!(validate_relative_path("docker/app").is_ok());
        assert!(validate_relative_path("../secret").is_err());
        assert!(validate_relative_path("/etc").is_err());
    }

    #[test]
    fn validates_git_commit_ids() {
        assert!(validate_git_commit("c0ffee1").is_ok());
        assert!(validate_git_commit("0123456789abcdef0123456789abcdef01234567").is_ok());
        assert!(validate_git_commit("HEAD~1").is_err());
        assert!(validate_git_commit("-deadbee").is_err());
    }

    #[test]
    fn validates_environment_keys() {
        assert!(validate_env_key("DATABASE_URL").is_ok());
        assert!(validate_env_key("_PRIVATE").is_ok());
        assert!(validate_env_key("BAD-KEY").is_err());
        assert!(validate_env_key("1BAD").is_err());
    }

    #[test]
    fn redacts_longest_secrets_first() {
        let output = redact_secrets(
            "token=abc123 and short=abc",
            &["abc".to_string(), "abc123".to_string()],
        );
        assert_eq!(output, "token=[REDACTED] and short=[REDACTED]");
    }
}
