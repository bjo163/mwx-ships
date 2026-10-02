use std::path::Path;
use tokio::process::Command;

#[derive(Debug, thiserror::Error)]
pub enum SshError {
    #[error("SSH execution failed: {0}")]
    ExecutionFailed(String),
    #[error("SSH connection timed out to {host}:{port}")]
    Timeout { host: String, port: i32 },
    #[error("Authentication failed for user {user}@{host}")]
    AuthFailed { user: String, host: String },
    #[error("Host validation failed: {0}")]
    HostValidation(String),
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

pub struct SshService;

impl SshService {
    /// Builds safe SSH command arguments without shell interpolation
    fn build_ssh_args(
        host: &str,
        port: i32,
        user: &str,
        identity_file: Option<&Path>,
    ) -> Vec<String> {
        let mut args = vec![
            "-p".to_string(),
            port.to_string(),
            "-o".to_string(),
            "StrictHostKeyChecking=accept-new".to_string(),
            "-o".to_string(),
            "ConnectTimeout=10".to_string(),
            "-o".to_string(),
            "BatchMode=yes".to_string(),
        ];

        if let Some(key_path) = identity_file {
            args.push("-i".to_string());
            args.push(key_path.display().to_string());
        }

        args.push(format!("{}@{}", user, host));
        args
    }

    /// Execute a server-generated command on a remote host via SSH
    pub async fn execute_remote_command(
        host: &str,
        port: i32,
        user: &str,
        identity_file: Option<&Path>,
        remote_command: &str,
    ) -> Result<(i32, String, String), SshError> {
        let mut args = Self::build_ssh_args(host, port, user, identity_file);
        args.push(remote_command.to_string());

        let output = Command::new("ssh")
            .args(&args)
            .output()
            .await
            .map_err(|e| SshError::ExecutionFailed(e.to_string()))?;

        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok((exit_code, stdout, stderr))
    }

    /// Test basic SSH connectivity to a host
    pub async fn test_connection(
        host: &str,
        port: i32,
        user: &str,
        identity_file: Option<&Path>,
    ) -> Result<bool, SshError> {
        let (code, _, _) =
            Self::execute_remote_command(host, port, user, identity_file, "echo moonships-ok")
                .await?;

        Ok(code == 0)
    }

    /// Execute server preflight inspection
    pub async fn run_preflight(
        server_id: i64,
        host: &str,
        port: i32,
        user: &str,
        identity_file: Option<&Path>,
    ) -> Result<PreflightReport, SshError> {
        let mut issues = Vec::new();

        // 1. Test SSH
        let ssh_connected = match Self::test_connection(host, port, user, identity_file).await {
            Ok(true) => true,
            _ => {
                issues.push("SSH connection could not be established".to_string());
                false
            }
        };

        if !ssh_connected {
            return Ok(PreflightReport {
                server_id,
                ssh_connected: false,
                docker_installed: false,
                docker_version: None,
                docker_running: false,
                disk_available_gb: None,
                memory_available_mb: None,
                cpu_cores: None,
                healthy: false,
                issues,
            });
        }

        // 2. Test Docker installed and running
        let (d_code, d_out, _) = Self::execute_remote_command(
            host,
            port,
            user,
            identity_file,
            "docker --version && docker info >/dev/null 2>&1 && echo DOCKER_OK",
        )
        .await?;

        let docker_installed = d_code == 0 || d_out.contains("Docker version");
        let docker_running = d_out.contains("DOCKER_OK");
        let docker_version = if docker_installed {
            d_out.lines().next().map(|s| s.trim().to_string())
        } else {
            issues.push("Docker is not installed or not in PATH".to_string());
            None
        };

        if docker_installed && !docker_running {
            issues
                .push("Docker daemon is not running or current user lacks permissions".to_string());
        }

        // 3. Inspect Disk space (in GB)
        let (_, disk_out, _) = Self::execute_remote_command(
            host,
            port,
            user,
            identity_file,
            "df -k / | tail -1 | awk '{print $4}'",
        )
        .await?;
        let disk_available_gb = disk_out
            .trim()
            .parse::<f64>()
            .ok()
            .map(|kbytes| (kbytes / 1024.0 / 1024.0 * 10.0).round() / 10.0);

        if let Some(gb) = disk_available_gb {
            if gb < 2.0 {
                issues.push(format!("Low disk space: only {:.1} GB available", gb));
            }
        }

        // 4. Inspect RAM (in MB)
        let (_, mem_out, _) = Self::execute_remote_command(
            host,
            port,
            user,
            identity_file,
            "free -m | grep Mem: | awk '{print $7}'",
        )
        .await?;
        let memory_available_mb = mem_out.trim().parse::<u64>().ok();

        // 5. Inspect CPU cores
        let (_, cpu_out, _) = Self::execute_remote_command(
            host,
            port,
            user,
            identity_file,
            "nproc || grep -c ^processor /proc/cpuinfo",
        )
        .await?;
        let cpu_cores = cpu_out.trim().parse::<u32>().ok();

        let healthy = ssh_connected && docker_installed && docker_running && issues.is_empty();

        Ok(PreflightReport {
            server_id,
            ssh_connected,
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
}
