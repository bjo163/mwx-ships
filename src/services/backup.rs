use crate::{
    models::{backup_runs, operational_events},
    services::crypto::CryptoService,
};
use chrono::Utc;
use loco_rs::prelude::*;
use sha2::{Digest, Sha256};
use std::{
    env,
    path::{Path, PathBuf},
};
use tokio::{fs, process::Command};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum BackupError {
    #[error("Moonships backup supports SQLite DATABASE_URL only")]
    UnsupportedDatabase,
    #[error("invalid database or backup path")]
    InvalidPath,
    #[error("source database does not exist: {0}")]
    SourceMissing(String),
    #[error("sqlite3 backup/verification failed: {0}")]
    Sqlite(String),
    #[error("backup IO failed: {0}")]
    Io(String),
    #[error("backup encryption failed: {0}")]
    Crypto(String),
    #[error("backup integrity verification failed: {0}")]
    Verification(String),
}

#[derive(Debug, Clone)]
pub struct BackupResult {
    pub run: backup_runs::Model,
    pub path: PathBuf,
}

pub struct BackupService;

impl BackupService {
    pub async fn run(db: &DatabaseConnection) -> Result<BackupResult, BackupError> {
        let source = database_path_from_env()?;
        if !source.exists() {
            return Err(BackupError::SourceMissing(source.display().to_string()));
        }

        let backup_dir = backup_dir();
        fs::create_dir_all(&backup_dir)
            .await
            .map_err(|err| BackupError::Io(err.to_string()))?;

        let encrypted = env_flag("MOONSHIPS_BACKUP_ENCRYPT", false);
        let stamp = Utc::now().format("%Y%m%d-%H%M%S");
        let suffix = &Uuid::new_v4().simple().to_string()[..8];
        let plain_path = backup_dir.join(format!("moonships-{stamp}-{suffix}.sqlite"));
        let final_path = if encrypted {
            plain_path.with_extension("sqlite.moonshipsbak")
        } else {
            plain_path.clone()
        };

        validate_path(&plain_path)?;
        validate_path(&final_path)?;

        let run = backup_runs::Model::start(db, &final_path.display().to_string(), encrypted)
            .await
            .map_err(|err| BackupError::Io(err.to_string()))?;

        let result = Self::create_verified_backup(&source, &plain_path, &final_path, encrypted).await;
        match result {
            Ok((size_bytes, sha256, verification_message)) => {
                let completed = backup_runs::Model::complete(
                    db,
                    run.id,
                    size_bytes,
                    &sha256,
                    true,
                    Some(verification_message.clone()),
                )
                .await
                .map_err(|err| BackupError::Io(err.to_string()))?;

                let _ = operational_events::Model::record(
                    db,
                    "backup_success",
                    "info",
                    Some("backup"),
                    Some(completed.id),
                    "SQLite backup completed and verified",
                    Some(&serde_json::json!({
                        "path": completed.backup_path,
                        "size_bytes": completed.size_bytes,
                        "sha256": completed.sha256,
                        "encrypted": completed.encrypted,
                    })),
                )
                .await;

                Self::prune().await?;

                Ok(BackupResult {
                    run: completed,
                    path: final_path,
                })
            }
            Err(error) => {
                let _ = fs::remove_file(&plain_path).await;
                if final_path != plain_path {
                    let _ = fs::remove_file(&final_path).await;
                }
                let _ = backup_runs::Model::fail(db, run.id, &error.to_string()).await;
                let _ = operational_events::Model::record(
                    db,
                    "backup_failed",
                    "critical",
                    Some("backup"),
                    Some(run.id),
                    "SQLite backup failed",
                    Some(&serde_json::json!({"error": error.to_string()})),
                )
                .await;
                Err(error)
            }
        }
    }

    async fn create_verified_backup(
        source: &Path,
        plain_path: &Path,
        final_path: &Path,
        encrypted: bool,
    ) -> Result<(i64, String, String), BackupError> {
        sqlite_backup(source, plain_path).await?;
        let verification_message = verify_sqlite_file(plain_path).await?;

        if encrypted {
            let plaintext = fs::read(plain_path)
                .await
                .map_err(|err| BackupError::Io(err.to_string()))?;
            let encrypted_bytes = CryptoService::encrypt_bytes(&plaintext)
                .map_err(|err| BackupError::Crypto(err.to_string()))?;
            write_private_file(final_path, &encrypted_bytes).await?;
            fs::remove_file(plain_path)
                .await
                .map_err(|err| BackupError::Io(err.to_string()))?;
        }

        let bytes = fs::read(final_path)
            .await
            .map_err(|err| BackupError::Io(err.to_string()))?;
        let sha256 = hex::encode(Sha256::digest(&bytes));
        Ok((bytes.len() as i64, sha256, verification_message))
    }

    pub async fn verify_export(path: &Path) -> Result<String, BackupError> {
        validate_path(path)?;
        if !path.exists() {
            return Err(BackupError::SourceMissing(path.display().to_string()));
        }

        if is_encrypted_export(path) {
            let bytes = fs::read(path)
                .await
                .map_err(|err| BackupError::Io(err.to_string()))?;
            let plaintext = CryptoService::decrypt_bytes(&bytes)
                .map_err(|err| BackupError::Crypto(err.to_string()))?;
            let temp = env::temp_dir().join(format!("moonships-verify-{}.sqlite", Uuid::new_v4()));
            write_private_file(&temp, &plaintext).await?;
            let result = verify_sqlite_file(&temp).await;
            let _ = fs::remove_file(&temp).await;
            result
        } else {
            verify_sqlite_file(path).await
        }
    }

    pub async fn restore_to_path(backup: &Path, target: &Path) -> Result<String, BackupError> {
        validate_path(backup)?;
        validate_path(target)?;
        if !backup.exists() {
            return Err(BackupError::SourceMissing(backup.display().to_string()));
        }

        let incoming = if is_encrypted_export(backup) {
            let bytes = fs::read(backup)
                .await
                .map_err(|err| BackupError::Io(err.to_string()))?;
            CryptoService::decrypt_bytes(&bytes)
                .map_err(|err| BackupError::Crypto(err.to_string()))?
        } else {
            fs::read(backup)
                .await
                .map_err(|err| BackupError::Io(err.to_string()))?
        };

        let parent = target.parent().ok_or(BackupError::InvalidPath)?;
        fs::create_dir_all(parent)
            .await
            .map_err(|err| BackupError::Io(err.to_string()))?;

        let staging = target.with_extension(format!("restore-{}.sqlite", Uuid::new_v4()));
        write_private_file(&staging, &incoming).await?;
        let verification = verify_sqlite_file(&staging).await?;

        if target.exists() {
            let rollback = target.with_extension(format!(
                "pre-restore-{}.sqlite",
                Utc::now().format("%Y%m%d%H%M%S")
            ));
            fs::copy(target, &rollback)
                .await
                .map_err(|err| BackupError::Io(err.to_string()))?;
        }

        fs::rename(&staging, target)
            .await
            .map_err(|err| BackupError::Io(err.to_string()))?;
        Ok(verification)
    }

    pub async fn prune() -> Result<(), BackupError> {
        let keep = env::var("MOONSHIPS_BACKUP_RETENTION")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(14)
            .max(1);
        let dir = backup_dir();
        if !dir.exists() {
            return Ok(());
        }

        let mut entries = fs::read_dir(&dir)
            .await
            .map_err(|err| BackupError::Io(err.to_string()))?;
        let mut backups = Vec::new();
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|err| BackupError::Io(err.to_string()))?
        {
            let path = entry.path();
            let name = path.file_name().and_then(|v| v.to_str()).unwrap_or_default();
            if name.starts_with("moonships-")
                && (name.ends_with(".sqlite") || name.ends_with(".sqlite.moonshipsbak"))
            {
                backups.push(path);
            }
        }

        backups.sort();
        let remove_count = backups.len().saturating_sub(keep);
        for path in backups.into_iter().take(remove_count) {
            fs::remove_file(path)
                .await
                .map_err(|err| BackupError::Io(err.to_string()))?;
        }
        Ok(())
    }
}

pub fn database_path_from_env() -> Result<PathBuf, BackupError> {
    let url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://data/moonships.sqlite?mode=rwc".to_string());
    let raw = url
        .strip_prefix("sqlite://")
        .ok_or(BackupError::UnsupportedDatabase)?;
    let path = raw.split('?').next().unwrap_or_default();
    if path.is_empty() || path == ":memory:" {
        return Err(BackupError::InvalidPath);
    }
    Ok(PathBuf::from(path))
}

fn backup_dir() -> PathBuf {
    env::var("MOONSHIPS_BACKUP_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("backups"))
}

fn env_flag(name: &str, default: bool) -> bool {
    env::var(name)
        .ok()
        .map(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(default)
}

fn is_encrypted_export(path: &Path) -> bool {
    path.to_string_lossy().ends_with(".moonshipsbak")
}

fn validate_path(path: &Path) -> Result<(), BackupError> {
    let value = path.to_string_lossy();
    if value.is_empty() || value.contains(['\n', '\r', '\0', '\'']) {
        return Err(BackupError::InvalidPath);
    }
    Ok(())
}

async fn sqlite_backup(source: &Path, destination: &Path) -> Result<(), BackupError> {
    let command = format!(".backup '{}'", destination.display());
    let output = Command::new("sqlite3")
        .arg(source)
        .arg(command)
        .output()
        .await
        .map_err(|err| BackupError::Sqlite(err.to_string()))?;
    if !output.status.success() {
        return Err(BackupError::Sqlite(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    verify_sqlite_file(destination).await.map(|_| ())
}

async fn verify_sqlite_file(path: &Path) -> Result<String, BackupError> {
    let output = Command::new("sqlite3")
        .arg(path)
        .arg("PRAGMA integrity_check; SELECT count(*) FROM sqlite_master WHERE type='table' AND name IN ('users','servers','projects','environments','applications','deployments');")
        .output()
        .await
        .map_err(|err| BackupError::Sqlite(err.to_string()))?;

    if !output.status.success() {
        return Err(BackupError::Verification(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lines = stdout.lines();
    let integrity = lines.next().unwrap_or_default();
    let critical_tables = lines.next().unwrap_or("0").parse::<u64>().unwrap_or(0);
    if integrity != "ok" {
        return Err(BackupError::Verification(integrity.to_string()));
    }
    if critical_tables < 6 {
        return Err(BackupError::Verification(format!(
            "expected critical schema tables, found {critical_tables}/6"
        )));
    }

    Ok(format!(
        "integrity=ok; critical_schema_tables={critical_tables}/6"
    ))
}

async fn write_private_file(path: &Path, bytes: &[u8]) -> Result<(), BackupError> {
    fs::write(path, bytes)
        .await
        .map_err(|err| BackupError::Io(err.to_string()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .await
            .map_err(|err| BackupError::Io(err.to_string()))?;
    }

    Ok(())
}
