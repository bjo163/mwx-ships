use crate::{models::servers, services::crypto::CryptoService};
use std::{path::PathBuf, process::Stdio, time::Duration};
use tokio::{io::AsyncWriteExt, process::Command, time::timeout};
use uuid::Uuid;

const SSH_CONNECT_TIMEOUT_SECS: u64 = 10;
const SSH_COMMAND_TIMEOUT_SECS: u64 = 900;

#[derive(Debug, thiserror::Error)]
pub enum SshError {
    #[error("SSH execution failed: {0}")]
    ExecutionFailed(String),
    #[error("SSH command timed out for {target}")]
    Timeout { target: String },
    #[error("Authentication failed for {user}@{host}")]
    AuthFailed { user: String, host: String },
    #[error("Host validation failed: {0}")]
    HostValidation(String),
    #[error("SSH credential could not be prepared: {0}")]
    Credential(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PreflightReport {
    pub server_id: i64,
    pub ssh_connected: bool,
    pub docker_installed: bool,
    pub docker_version: Option<String>,
    pub docker_running: bool,
    pub disk_available_gb: Option<f64>,
    pub memory_available_mb: Option<u64>,
    pub cpu_cores: Option<u32>,
    pub healthy: bool,
    pub issues: Vec<String>,
}

#[derive(Debug)]
pub struct SshSession {
    host: String,
    port: i32,
    user: String,
    identity_file: Option<PathBuf>,
    known_hosts_file: PathBuf,
}

impl Drop for SshSession {
    fn drop(&mut self) {
        if let Some(path) = &self.identity_file {
            let _ = std::fs::remove_file(path);
        }
        let _ = std::fs::remove_file(&self.known_hosts_file);
    }
}

impl SshSession {
    pub fn target(&self) -> String {
        format!("{}@{}:{}", self.user, self.host, self.port)
    }

    fn command_timeout() -> Duration {
        std::env::var("MOONSHIPS_SSH_COMMAND_TIMEOUT_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0)
            .map(Duration::from_secs)
            .unwrap_or_else(|| Duration::from_secs(SSH_COMMAND_TIMEOUT_SECS))
    }

    fn build_args(&self) -> Vec<String> {
        let mut args = vec![
            "-p".to_string(),
            self.port.to_string(),
            "-o".to_string(),
            "StrictHostKeyChecking=yes".to_string(),
            "-o".to_string(),
            format!(
                "UserKnownHostsFile={}",
                self.known_hosts_file.to_string_lossy()
            ),
            "-o".to_string(),
            format!("ConnectTimeout={SSH_CONNECT_TIMEOUT_SECS}"),
            "-o".to_string(),
            "BatchMode=yes".to_string(),
        ];

        if let Some(key_path) = &self.identity_file {
            args.push("-i".to_string());
            args.push(key_path.to_string_lossy().to_string());
        }

        args.push(format!("{}@{}", self.user, self.host));
        args
    }

    pub async fn execute(&self, remote_command: &str) -> Result<(i32, String, String), SshError> {
        self.execute_with_timeout(remote_command, Self::command_timeout())
            .await
    }

    pub async fn execute_with_timeout(
        &self,
        remote_command: &str,
        command_timeout: Duration,
    ) -> Result<(i32, String, String), SshError> {
        let mut command = Command::new("ssh");
        command.args(self.build_args()).arg(remote_command);
        command.kill_on_drop(true);

        let output = timeout(command_timeout, command.output())
            .await
            .map_err(|_| SshError::Timeout {
                target: self.target(),
            })?
            .map_err(|e| SshError::ExecutionFailed(e.to_string()))?;

        Self::decode_output(&self.host, &self.user, output)
    }

    pub async fn execute_with_input(
        &self,
        remote_command: &str,
        input: &str,
    ) -> Result<(i32, String, String), SshError> {
        let mut command = Command::new("ssh");
        command
            .args(self.build_args())
            .arg(remote_command)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command.kill_on_drop(true);

        let mut child = command
            .spawn()
            .map_err(|e| SshError::ExecutionFailed(e.to_string()))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(input.as_bytes())
                .await
                .map_err(|e| SshError::ExecutionFailed(e.to_string()))?;
        }

        let output = timeout(Self::command_timeout(), child.wait_with_output())
            .await
            .map_err(|_| SshError::Timeout {
                target: self.target(),
            })?
            .map_err(|e| SshError::ExecutionFailed(e.to_string()))?;

        Self::decode_output(&self.host, &self.user, output)
    }

    fn decode_output(
        host: &str,
        user: &str,
        output: std::process::Output,
    ) -> Result<(i32, String, String), SshError> {
        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if stderr.contains("Permission denied") {
            return Err(SshError::AuthFailed {
                user: user.to_string(),
                host: host.to_string(),
            });
        }

        Ok((exit_code, stdout, stderr))
    }
}

pub struct SshService;

impl SshService {
    pub async fn connect(server: &servers::Model) -> Result<SshSession, SshError> {
        Self::validate_target(&server.host, server.port, &server.username)?;

        let scanned_host_keys = Self::scan_host_keys(&server.host, server.port).await?;
        if let Some(expected) = server
            .known_host_fingerprint
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            Self::verify_fingerprint(&scanned_host_keys, expected).await?;
        }

        let known_hosts_file = Self::write_temp_file(
            server.id,
            "known-hosts",
            scanned_host_keys.as_bytes(),
            false,
        )
        .await?;

        let identity_file = match &server.encrypted_private_key {
            Some(encrypted) => {
                let private_key = match CryptoService::decrypt(encrypted) {
                    Ok(value) => value,
                    Err(err) => {
                        let _ = tokio::fs::remove_file(&known_hosts_file).await;
                        return Err(SshError::Credential(err.to_string()));
                    }
                };
                match Self::write_temp_file(server.id, "identity", private_key.as_bytes(), true)
                    .await
                {
                    Ok(path) => Some(path),
                    Err(err) => {
                        let _ = tokio::fs::remove_file(&known_hosts_file).await;
                        return Err(err);
                    }
                }
            }
            None => None,
        };

        let session = SshSession {
            host: server.host.clone(),
            port: server.port,
            user: server.username.clone(),
            identity_file,
            known_hosts_file,
        };

        let (code, _, stderr) = session
            .execute_with_timeout("printf moonships-ok", Duration::from_secs(15))
            .await?;
        if code != 0 {
            return Err(SshError::ExecutionFailed(format!(
                "SSH connectivity probe exited with code {code}: {}",
                stderr.trim()
            )));
        }

        Ok(session)
    }

    pub async fn run_preflight(server: &servers::Model) -> Result<PreflightReport, SshError> {
        let session = match Self::connect(server).await {
            Ok(session) => session,
            Err(err) => {
                return Ok(PreflightReport {
                    server_id: server.id,
                    ssh_connected: false,
                    docker_installed: false,
                    docker_version: None,
                    docker_running: false,
                    disk_available_gb: None,
                    memory_available_mb: None,
                    cpu_cores: None,
                    healthy: false,
                    issues: vec![err.to_string()],
                });
            }
        };

        let mut issues = Vec::new();

        let (docker_code, docker_out, docker_err) = session
            .execute("docker --version; docker info >/dev/null 2>&1 && printf '\nDOCKER_OK\n'")
            .await?;

        let docker_installed = docker_out.contains("Docker version");
        let docker_running = docker_out.contains("DOCKER_OK") && docker_code == 0;
        let docker_version = docker_out
            .lines()
            .find(|line| line.contains("Docker version"))
            .map(|line| line.trim().to_string());

        if !docker_installed {
            issues.push("Docker is not installed or not in PATH".to_string());
        } else if !docker_running {
            let detail = docker_err.trim();
            issues.push(if detail.is_empty() {
                "Docker daemon is not running or current user lacks permissions".to_string()
            } else {
                format!("Docker unavailable: {detail}")
            });
        }

        let disk_available_gb =
            Self::parse_remote_f64(&session, "df -k / | tail -1 | awk '{print $4}'")
                .await?
                .map(|kbytes| (kbytes / 1024.0 / 1024.0 * 10.0).round() / 10.0);

        if let Some(gb) = disk_available_gb {
            if gb < 2.0 {
                issues.push(format!("Low disk space: only {gb:.1} GB available"));
            }
        }

        let memory_available_mb =
            Self::parse_remote_u64(&session, "free -m | awk '/^Mem:/ {print $7}'").await?;

        let cpu_cores =
            Self::parse_remote_u32(&session, "nproc || grep -c ^processor /proc/cpuinfo").await?;

        let healthy = docker_installed && docker_running && issues.is_empty();

        Ok(PreflightReport {
            server_id: server.id,
            ssh_connected: true,
            docker_installed,
            docker_version,
            docker_running,
            disk_available_gb,
            memory_available_mb,
            cpu_cores,
            healthy,
            issues,
        })
    }

    fn validate_target(host: &str, port: i32, user: &str) -> Result<(), SshError> {
        let valid_host = !host.trim().is_empty()
            && !host.starts_with('-')
            && host
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ':'));
        if !valid_host {
            return Err(SshError::HostValidation(format!(
                "invalid SSH host '{}'",
                host
            )));
        }

        if !(1..=65535).contains(&port) {
            return Err(SshError::HostValidation(format!("invalid SSH port {port}")));
        }

        let valid_user = !user.trim().is_empty()
            && !user.starts_with('-')
            && user
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'));
        if !valid_user {
            return Err(SshError::HostValidation(format!(
                "invalid SSH username '{}'",
                user
            )));
        }

        Ok(())
    }

    async fn scan_host_keys(host: &str, port: i32) -> Result<String, SshError> {
        let mut command = Command::new("ssh-keyscan");
        command
            .arg("-T")
            .arg(SSH_CONNECT_TIMEOUT_SECS.to_string())
            .arg("-p")
            .arg(port.to_string())
            .arg(host);
        command.kill_on_drop(true);

        let output = timeout(Duration::from_secs(15), command.output())
            .await
            .map_err(|_| SshError::Timeout {
                target: format!("{host}:{port}"),
            })?
            .map_err(|e| SshError::ExecutionFailed(e.to_string()))?;

        let keys = String::from_utf8_lossy(&output.stdout).to_string();
        if !output.status.success() || keys.trim().is_empty() {
            return Err(SshError::HostValidation(format!(
                "could not obtain SSH host key for {host}:{port}"
            )));
        }

        Ok(keys)
    }

    async fn verify_fingerprint(scanned_keys: &str, expected: &str) -> Result<(), SshError> {
        let mut command = Command::new("ssh-keygen");
        command
            .arg("-lf")
            .arg("-")
            .arg("-E")
            .arg("sha256")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command.kill_on_drop(true);

        let mut child = command
            .spawn()
            .map_err(|e| SshError::ExecutionFailed(e.to_string()))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(scanned_keys.as_bytes())
                .await
                .map_err(|e| SshError::ExecutionFailed(e.to_string()))?;
        }

        let output = timeout(Duration::from_secs(10), child.wait_with_output())
            .await
            .map_err(|_| {
                SshError::HostValidation(
                    "timed out while verifying SSH host fingerprint".to_string(),
                )
            })?
            .map_err(|e| SshError::ExecutionFailed(e.to_string()))?;

        let fingerprints = String::from_utf8_lossy(&output.stdout);
        if !output.status.success() || !fingerprints.lines().any(|line| line.contains(expected)) {
            return Err(SshError::HostValidation(format!(
                "SSH host fingerprint mismatch; expected {expected}"
            )));
        }

        Ok(())
    }

    async fn write_temp_file(
        server_id: i64,
        kind: &str,
        bytes: &[u8],
        private: bool,
    ) -> Result<PathBuf, SshError> {
        let path =
            std::env::temp_dir().join(format!("moonships-{kind}-{server_id}-{}", Uuid::new_v4()));
        tokio::fs::write(&path, bytes)
            .await
            .map_err(|e| SshError::Credential(e.to_string()))?;

        if private {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                tokio::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
                    .await
                    .map_err(|e| SshError::Credential(e.to_string()))?;
            }
        }

        Ok(path)
    }

    async fn parse_remote_f64(
        session: &SshSession,
        command: &str,
    ) -> Result<Option<f64>, SshError> {
        let (_, stdout, _) = session.execute(command).await?;
        Ok(stdout.trim().parse::<f64>().ok())
    }

    async fn parse_remote_u64(
        session: &SshSession,
        command: &str,
    ) -> Result<Option<u64>, SshError> {
        let (_, stdout, _) = session.execute(command).await?;
        Ok(stdout.trim().parse::<u64>().ok())
    }

    async fn parse_remote_u32(
        session: &SshSession,
        command: &str,
    ) -> Result<Option<u32>, SshError> {
        let (_, stdout, _) = session.execute(command).await?;
        Ok(stdout.trim().parse::<u32>().ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_ssh_option_injection_targets() {
        assert!(SshService::validate_target("-oProxyCommand=whoami", 22, "deploy").is_err());
        assert!(SshService::validate_target("example.com", 0, "deploy").is_err());
        assert!(SshService::validate_target("example.com", 22, "-root").is_err());
        assert!(SshService::validate_target("2001:db8::1", 22, "deploy").is_ok());
    }
}
