use std::path::Path;
use tokio::process::Command;

#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("Invalid Git URL: {0}")]
    InvalidUrl(String),
    #[error("Invalid branch name: {0}")]
    InvalidBranch(String),
    #[error("Git execution failed ({step}): {message}")]
    ExecutionFailed { step: String, message: String },
}

pub struct GitService;

impl GitService {
    /// Validates that a Git URL is safe and well-formed (HTTPS or SSH)
    pub fn validate_url(url: &str) -> Result<(), GitError> {
        let trimmed = url.trim();
        if trimmed.is_empty() {
            return Err(GitError::InvalidUrl("Git URL cannot be empty".to_string()));
        }

        // Prevent shell injection characters
        if trimmed.contains(';')
            || trimmed.contains('&')
            || trimmed.contains('|')
            || trimmed.contains('`')
            || trimmed.contains('$')
            || trimmed.contains('\n')
            || trimmed.contains('\r')
        {
            return Err(GitError::InvalidUrl(
                "URL contains forbidden shell control characters".to_string(),
            ));
        }

        let is_https = trimmed.starts_with("https://") || trimmed.starts_with("http://");
        let is_ssh = trimmed.starts_with("git@") || trimmed.starts_with("ssh://");

        if !is_https && !is_ssh {
            return Err(GitError::InvalidUrl(
                "Git URL must start with https://, http://, git@, or ssh://".to_string(),
            ));
        }

        Ok(())
    }

    /// Validates a branch name according to Git naming conventions
    pub fn validate_branch(branch: &str) -> Result<(), GitError> {
        let trimmed = branch.trim();
        if trimmed.is_empty() {
            return Err(GitError::InvalidBranch(
                "Branch name cannot be empty".to_string(),
            ));
        }

        if trimmed.starts_with('-')
            || trimmed.contains("..")
            || trimmed.contains(' ')
            || trimmed.contains('~')
            || trimmed.contains('^')
            || trimmed.contains(':')
        {
            return Err(GitError::InvalidBranch(format!(
                "Branch name '{}' is malformed",
                branch
            )));
        }

        Ok(())
    }

    /// Clone a repository into the specified directory
    pub async fn clone_repository(
        repo_url: &str,
        branch: &str,
        target_dir: &Path,
    ) -> Result<String, GitError> {
        Self::validate_url(repo_url)?;
        Self::validate_branch(branch)?;

        let output = Command::new("git")
            .arg("clone")
            .arg("--depth")
            .arg("1")
            .arg("--branch")
            .arg(branch)
            .arg(repo_url)
            .arg(target_dir)
            .output()
            .await
            .map_err(|e| GitError::ExecutionFailed {
                step: "clone".to_string(),
                message: e.to_string(),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GitError::ExecutionFailed {
                step: "clone".to_string(),
                message: stderr.to_string(),
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Fetch latest changes in an existing repository directory
    pub async fn fetch_repository(repo_dir: &Path) -> Result<String, GitError> {
        let output = Command::new("git")
            .arg("-C")
            .arg(repo_dir)
            .arg("fetch")
            .arg("--all")
            .output()
            .await
            .map_err(|e| GitError::ExecutionFailed {
                step: "fetch".to_string(),
                message: e.to_string(),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GitError::ExecutionFailed {
                step: "fetch".to_string(),
                message: stderr.to_string(),
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Checkout a specific branch
    pub async fn checkout_branch(repo_dir: &Path, branch: &str) -> Result<String, GitError> {
        Self::validate_branch(branch)?;

        let output = Command::new("git")
            .arg("-C")
            .arg(repo_dir)
            .arg("checkout")
            .arg(branch)
            .output()
            .await
            .map_err(|e| GitError::ExecutionFailed {
                step: "checkout_branch".to_string(),
                message: e.to_string(),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GitError::ExecutionFailed {
                step: "checkout_branch".to_string(),
                message: stderr.to_string(),
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Retrieve the current commit SHA and message
    pub async fn get_current_commit(repo_dir: &Path) -> Result<(String, String), GitError> {
        let sha_output = Command::new("git")
            .arg("-C")
            .arg(repo_dir)
            .arg("rev-parse")
            .arg("HEAD")
            .output()
            .await
            .map_err(|e| GitError::ExecutionFailed {
                step: "rev-parse".to_string(),
                message: e.to_string(),
            })?;

        if !sha_output.status.success() {
            let stderr = String::from_utf8_lossy(&sha_output.stderr);
            return Err(GitError::ExecutionFailed {
                step: "rev-parse".to_string(),
                message: stderr.to_string(),
            });
        }

        let sha = String::from_utf8_lossy(&sha_output.stdout)
            .trim()
            .to_string();

        let msg_output = Command::new("git")
            .arg("-C")
            .arg(repo_dir)
            .arg("log")
            .arg("-1")
            .arg("--pretty=%B")
            .output()
            .await
            .map_err(|e| GitError::ExecutionFailed {
                step: "log".to_string(),
                message: e.to_string(),
            })?;

        let message = if msg_output.status.success() {
            String::from_utf8_lossy(&msg_output.stdout)
                .trim()
                .to_string()
        } else {
            String::new()
        };

        Ok((sha, message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_urls() {
        assert!(GitService::validate_url("https://github.com/bjo163/mwx-ships.git").is_ok());
        assert!(GitService::validate_url("git@github.com:bjo163/mwx-ships.git").is_ok());
        assert!(GitService::validate_url("ssh://git@gitlab.com/group/repo.git").is_ok());
    }

    #[test]
    fn test_invalid_urls() {
        assert!(GitService::validate_url("https://github.com/repo.git; rm -rf /").is_err());
        assert!(GitService::validate_url("ftp://example.com/repo.git").is_err());
        assert!(GitService::validate_url("").is_err());
    }

    #[test]
    fn test_branch_validation() {
        assert!(GitService::validate_branch("main").is_ok());
        assert!(GitService::validate_branch("feature/awesome-ui").is_ok());
        assert!(GitService::validate_branch("-bad-branch").is_err());
        assert!(GitService::validate_branch("branch with spaces").is_err());
    }
}
