use std::path::Path;
use tokio::process::Command;

#[derive(Debug, thiserror::Error)]
pub enum DockerError {
    #[error("Invalid container name: {0}")]
    InvalidContainerName(String),
    #[error("Invalid image name: {0}")]
    InvalidImageName(String),
    #[error("Invalid volume name: {0}")]
    InvalidVolumeName(String),
    #[error("Invalid volume mount path: {0}")]
    InvalidVolumeMountPath(String),
    #[error("Docker command failed ({operation}): {message}")]
    CommandFailed { operation: String, message: String },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct VolumeMount {
    pub source: String,
    pub target: String,
    pub read_only: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContainerConfig {
    pub name: String,
    pub image: String,
    pub container_port: i32,
    pub published_port: Option<i32>,
    pub env_vars: Vec<(String, String)>,
    pub labels: Vec<(String, String)>,
    pub restart_policy: String,
    pub network: Option<String>,
    #[serde(default)]
    pub volume_mounts: Vec<VolumeMount>,
    #[serde(default)]
    pub command: Vec<String>,
}

pub struct DockerService;

impl DockerService {
    /// Validates container names (alphanumeric, dots, dashes, underscores)
    pub fn validate_container_name(name: &str) -> Result<(), DockerError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(DockerError::InvalidContainerName(
                "Container name cannot be empty".to_string(),
            ));
        }

        if !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return Err(DockerError::InvalidContainerName(format!(
                "Container name '{}' contains invalid characters",
                name
            )));
        }

        Ok(())
    }

    /// Validates image names
    pub fn validate_image_name(image: &str) -> Result<(), DockerError> {
        let trimmed = image.trim();
        if trimmed.is_empty() {
            return Err(DockerError::InvalidImageName(
                "Image name cannot be empty".to_string(),
            ));
        }

        if trimmed.contains(';')
            || trimmed.contains('&')
            || trimmed.contains('|')
            || trimmed.contains('`')
            || trimmed.contains(' ')
        {
            return Err(DockerError::InvalidImageName(format!(
                "Image name '{}' contains forbidden characters",
                image
            )));
        }

        Ok(())
    }

    pub fn validate_volume_name(name: &str) -> Result<(), DockerError> {
        let trimmed = name.trim();
        if trimmed.is_empty()
            || trimmed.len() > 128
            || !trimmed
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        {
            return Err(DockerError::InvalidVolumeName(name.to_string()));
        }
        Ok(())
    }

    pub fn validate_volume_mount_path(path: &str) -> Result<(), DockerError> {
        let path = path.trim();
        if !path.starts_with('/') || path == "/" || path.len() > 255 {
            return Err(DockerError::InvalidVolumeMountPath(path.to_string()));
        }
        let safe = path.split('/').skip(1).all(|segment| {
            !segment.is_empty()
                && !matches!(segment, "." | "..")
                && segment
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        });
        if !safe || path.contains(['\n', '\r', '\0']) {
            return Err(DockerError::InvalidVolumeMountPath(path.to_string()));
        }
        Ok(())
    }

    /// Build a Docker image from context and Dockerfile
    pub async fn build_image(
        context_path: &Path,
        dockerfile_path: &Path,
        tag: &str,
    ) -> Result<String, DockerError> {
        Self::validate_image_name(tag)?;

        let output = Command::new("docker")
            .arg("build")
            .arg("-t")
            .arg(tag)
            .arg("-f")
            .arg(dockerfile_path)
            .arg(context_path)
            .output()
            .await
            .map_err(|e| DockerError::CommandFailed {
                operation: "build".to_string(),
                message: e.to_string(),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DockerError::CommandFailed {
                operation: "build".to_string(),
                message: stderr.to_string(),
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Pull a Docker image
    pub async fn pull_image(image: &str) -> Result<String, DockerError> {
        Self::validate_image_name(image)?;

        let output = Command::new("docker")
            .arg("pull")
            .arg(image)
            .output()
            .await
            .map_err(|e| DockerError::CommandFailed {
                operation: "pull".to_string(),
                message: e.to_string(),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DockerError::CommandFailed {
                operation: "pull".to_string(),
                message: stderr.to_string(),
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Start a stopped container
    pub async fn start_container(name: &str) -> Result<(), DockerError> {
        Self::validate_container_name(name)?;

        let output = Command::new("docker")
            .arg("start")
            .arg(name)
            .output()
            .await
            .map_err(|e| DockerError::CommandFailed {
                operation: "start".to_string(),
                message: e.to_string(),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DockerError::CommandFailed {
                operation: "start".to_string(),
                message: stderr.to_string(),
            });
        }

        Ok(())
    }

    /// Stop a running container
    pub async fn stop_container(name: &str) -> Result<(), DockerError> {
        Self::validate_container_name(name)?;

        let output = Command::new("docker")
            .arg("stop")
            .arg(name)
            .output()
            .await
            .map_err(|e| DockerError::CommandFailed {
                operation: "stop".to_string(),
                message: e.to_string(),
            })?;

        // Ignore failure if container wasn't running
        let _ = output;
        Ok(())
    }

    /// Remove a container
    pub async fn remove_container(name: &str) -> Result<(), DockerError> {
        Self::validate_container_name(name)?;

        let _ = Command::new("docker")
            .arg("rm")
            .arg("-f")
            .arg(name)
            .output()
            .await;

        Ok(())
    }

    /// Restart a container
    pub async fn restart_container(name: &str) -> Result<(), DockerError> {
        Self::validate_container_name(name)?;

        let output = Command::new("docker")
            .arg("restart")
            .arg(name)
            .output()
            .await
            .map_err(|e| DockerError::CommandFailed {
                operation: "restart".to_string(),
                message: e.to_string(),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DockerError::CommandFailed {
                operation: "restart".to_string(),
                message: stderr.to_string(),
            });
        }

        Ok(())
    }

    /// Get container logs
    pub async fn container_logs(name: &str, tail: u32) -> Result<String, DockerError> {
        Self::validate_container_name(name)?;

        let output = Command::new("docker")
            .arg("logs")
            .arg("--tail")
            .arg(tail.to_string())
            .arg(name)
            .output()
            .await
            .map_err(|e| DockerError::CommandFailed {
                operation: "logs".to_string(),
                message: e.to_string(),
            })?;

        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        Ok(combined)
    }

    /// Inspect container status
    pub async fn container_status(name: &str) -> Result<String, DockerError> {
        Self::validate_container_name(name)?;

        let output = Command::new("docker")
            .arg("inspect")
            .arg("--format")
            .arg("{{.State.Status}}")
            .arg(name)
            .output()
            .await
            .map_err(|e| DockerError::CommandFailed {
                operation: "inspect".to_string(),
                message: e.to_string(),
            })?;

        if !output.status.success() {
            return Ok("stopped".to_string());
        }

        let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(status)
    }

    /// Run a container with structured configuration
    pub async fn run_container(config: &ContainerConfig) -> Result<String, DockerError> {
        Self::validate_container_name(&config.name)?;
        Self::validate_image_name(&config.image)?;

        // Stop & remove any previous container with this name
        let _ = Self::stop_container(&config.name).await;
        let _ = Self::remove_container(&config.name).await;

        let mut cmd = Command::new("docker");
        cmd.arg("run").arg("-d").arg("--name").arg(&config.name);

        if !config.restart_policy.is_empty() {
            cmd.arg("--restart").arg(&config.restart_policy);
        }

        if let Some(net) = &config.network {
            cmd.arg("--network").arg(net);
        }

        if let Some(pub_port) = config.published_port {
            cmd.arg("-p")
                .arg(format!("{}:{}", pub_port, config.container_port));
        }

        for (k, v) in &config.env_vars {
            cmd.arg("-e").arg(format!("{}={}", k, v));
        }

        for mount in &config.volume_mounts {
            Self::validate_volume_name(&mount.source)?;
            Self::validate_volume_mount_path(&mount.target)?;
            let mut spec = format!(
                "type=volume,source={},target={}",
                mount.source, mount.target
            );
            if mount.read_only {
                spec.push_str(",readonly");
            }
            cmd.arg("--mount").arg(spec);
        }

        for (k, v) in &config.labels {
            cmd.arg("-l").arg(format!("{}={}", k, v));
        }

        cmd.arg(&config.image);
        for arg in &config.command {
            cmd.arg(arg);
        }

        let output = cmd.output().await.map_err(|e| DockerError::CommandFailed {
            operation: "run".to_string(),
            message: e.to_string(),
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DockerError::CommandFailed {
                operation: "run".to_string(),
                message: stderr.to_string(),
            });
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(container_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_container_names() {
        assert!(DockerService::validate_container_name("moonships-app-1").is_ok());
        assert!(DockerService::validate_container_name("web.service_v2").is_ok());
        assert!(DockerService::validate_container_name("").is_err());
        assert!(DockerService::validate_container_name("bad name with spaces").is_err());
        assert!(DockerService::validate_container_name("name; rm -rf /").is_err());
    }

    #[test]
    fn test_valid_image_names() {
        assert!(DockerService::validate_image_name("nginx:alpine").is_ok());
        assert!(DockerService::validate_image_name("ghcr.io/org/repo:v1.0.0").is_ok());
        assert!(DockerService::validate_image_name("").is_err());
        assert!(DockerService::validate_image_name("image; whoami").is_err());
    }

    #[test]
    fn volume_mounts_are_strictly_validated() {
        assert!(DockerService::validate_volume_name("moonships-v-abc123").is_ok());
        assert!(DockerService::validate_volume_name("bad volume").is_err());
        assert!(DockerService::validate_volume_mount_path("/var/lib/data").is_ok());
        assert!(DockerService::validate_volume_mount_path("/var/../etc").is_err());
        assert!(DockerService::validate_volume_mount_path("/").is_err());
    }
}
