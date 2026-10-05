use crate::{
    models::managed_services::{ManagedServiceCredentials, Model as ManagedServiceModel},
    services::{
        docker::{ContainerConfig, DockerService, VolumeMount},
        proxy::ProxyService,
    },
};
use aes_gcm::aead::{rand_core::RngCore, OsRng};
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct CreateManagedServiceParams {
    pub server_id: i64,
    pub name: String,
    pub kind: String,
    pub image: Option<String>,
    pub database_name: Option<String>,
    pub username: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedServiceTemplate {
    pub kind: &'static str,
    pub image: &'static str,
    pub internal_port: i32,
    pub volume_mount_path: &'static str,
    pub default_database: Option<&'static str>,
    pub default_username: Option<&'static str>,
    pub default_env_prefix: &'static str,
}

#[derive(Debug, Clone)]
pub struct PreparedManagedService {
    pub name: String,
    pub slug: String,
    pub kind: String,
    pub image: String,
    pub container_name: String,
    pub internal_port: i32,
    pub volume_mount_path: String,
    pub database_name: Option<String>,
    pub username: Option<String>,
    pub env_prefix: String,
    pub credentials: ManagedServiceCredentials,
}

#[derive(Debug, Clone, Serialize)]
pub struct ManagedServiceTemplateView {
    pub kind: String,
    pub default_image: String,
    pub internal_port: i32,
    pub volume_mount_path: String,
    pub default_env_prefix: String,
}

pub struct ManagedServiceTemplateService;

impl ManagedServiceTemplateService {
    pub const TEMPLATES: [ManagedServiceTemplate; 4] = [
        ManagedServiceTemplate {
            kind: "postgresql",
            image: "postgres:17.11-alpine",
            internal_port: 5432,
            volume_mount_path: "/var/lib/postgresql/data",
            default_database: Some("app"),
            default_username: Some("moonships"),
            default_env_prefix: "POSTGRES",
        },
        ManagedServiceTemplate {
            kind: "mysql",
            image: "mysql:8.4.11",
            internal_port: 3306,
            volume_mount_path: "/var/lib/mysql",
            default_database: Some("app"),
            default_username: Some("moonships"),
            default_env_prefix: "MYSQL",
        },
        ManagedServiceTemplate {
            kind: "mariadb",
            image: "mariadb:11.4.13",
            internal_port: 3306,
            volume_mount_path: "/var/lib/mysql",
            default_database: Some("app"),
            default_username: Some("moonships"),
            default_env_prefix: "MARIADB",
        },
        ManagedServiceTemplate {
            kind: "redis",
            image: "redis:8.10.2-alpine",
            internal_port: 6379,
            volume_mount_path: "/data",
            default_database: None,
            default_username: None,
            default_env_prefix: "REDIS",
        },
    ];

    pub fn list() -> Vec<ManagedServiceTemplateView> {
        Self::TEMPLATES
            .iter()
            .map(|template| ManagedServiceTemplateView {
                kind: template.kind.to_string(),
                default_image: template.image.to_string(),
                internal_port: template.internal_port,
                volume_mount_path: template.volume_mount_path.to_string(),
                default_env_prefix: template.default_env_prefix.to_string(),
            })
            .collect()
    }

    pub fn template(kind: &str) -> Result<&'static ManagedServiceTemplate> {
        let normalized = normalize_kind(kind);
        Self::TEMPLATES
            .iter()
            .find(|template| template.kind == normalized)
            .ok_or_else(|| {
                Error::BadRequest(
                    "managed service kind must be postgresql, mysql, mariadb, or redis".to_string(),
                )
            })
    }

    pub fn prepare(params: &CreateManagedServiceParams) -> Result<PreparedManagedService> {
        let template = Self::template(&params.kind)?;
        let name = params.name.trim();
        if name.len() < 2 || name.len() > 80 {
            return Err(Error::BadRequest(
                "managed service name must contain 2-80 characters".to_string(),
            ));
        }
        let slug = normalize_slug(name);
        if slug.is_empty() {
            return Err(Error::BadRequest(
                "managed service name must produce a non-empty slug".to_string(),
            ));
        }

        let image = params
            .image
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(template.image)
            .to_string();
        validate_pinned_image(&image)?;

        let database_name = match template.default_database {
            Some(default) => Some(normalize_identifier(
                params.database_name.as_deref().unwrap_or(default),
                "database_name",
            )?),
            None => None,
        };
        let username = match template.default_username {
            Some(default) => Some(normalize_identifier(
                params.username.as_deref().unwrap_or(default),
                "username",
            )?),
            None => None,
        };

        let credentials = ManagedServiceCredentials {
            password: random_secret(),
            root_password: matches!(template.kind, "mysql" | "mariadb").then(random_secret),
        };

        Ok(PreparedManagedService {
            name: name.to_string(),
            slug: slug.clone(),
            kind: template.kind.to_string(),
            image,
            container_name: format!("moonships-service-{slug}"),
            internal_port: template.internal_port,
            volume_mount_path: template.volume_mount_path.to_string(),
            database_name,
            username,
            env_prefix: template.default_env_prefix.to_string(),
            credentials,
        })
    }

    pub fn runtime_config(
        service: &ManagedServiceModel,
        volume_name: &str,
        credentials: &ManagedServiceCredentials,
    ) -> Result<ContainerConfig> {
        let template = Self::template(&service.kind)?;
        let mut env_vars = Vec::new();
        let mut command = Vec::new();

        match template.kind {
            "postgresql" => {
                env_vars.push((
                    "POSTGRES_DB".to_string(),
                    required(service.database_name.as_deref(), "database_name")?.to_string(),
                ));
                env_vars.push((
                    "POSTGRES_USER".to_string(),
                    required(service.username.as_deref(), "username")?.to_string(),
                ));
                env_vars.push((
                    "POSTGRES_PASSWORD".to_string(),
                    credentials.password.clone(),
                ));
            }
            "mysql" => {
                env_vars.push((
                    "MYSQL_DATABASE".to_string(),
                    required(service.database_name.as_deref(), "database_name")?.to_string(),
                ));
                env_vars.push((
                    "MYSQL_USER".to_string(),
                    required(service.username.as_deref(), "username")?.to_string(),
                ));
                env_vars.push(("MYSQL_PASSWORD".to_string(), credentials.password.clone()));
                env_vars.push((
                    "MYSQL_ROOT_PASSWORD".to_string(),
                    required(credentials.root_password.as_deref(), "root_password")?.to_string(),
                ));
            }
            "mariadb" => {
                env_vars.push((
                    "MARIADB_DATABASE".to_string(),
                    required(service.database_name.as_deref(), "database_name")?.to_string(),
                ));
                env_vars.push((
                    "MARIADB_USER".to_string(),
                    required(service.username.as_deref(), "username")?.to_string(),
                ));
                env_vars.push(("MARIADB_PASSWORD".to_string(), credentials.password.clone()));
                env_vars.push((
                    "MARIADB_ROOT_PASSWORD".to_string(),
                    required(credentials.root_password.as_deref(), "root_password")?.to_string(),
                ));
            }
            "redis" => {
                env_vars.push(("REDIS_PASSWORD".to_string(), credentials.password.clone()));
                command = vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    "exec redis-server --appendonly yes --requirepass \"$REDIS_PASSWORD\""
                        .to_string(),
                ];
            }
            _ => unreachable!("validated template kind"),
        }

        Ok(ContainerConfig {
            name: service.container_name.clone(),
            image: service.image.clone(),
            container_port: service.internal_port,
            published_port: None,
            env_vars,
            labels: vec![
                ("moonships.managed".to_string(), "true".to_string()),
                ("moonships.service.id".to_string(), service.id.to_string()),
                ("moonships.service.kind".to_string(), service.kind.clone()),
            ],
            restart_policy: "unless-stopped".to_string(),
            network: Some(ProxyService::MANAGED_NETWORK.to_string()),
            volume_mounts: vec![VolumeMount {
                source: volume_name.to_string(),
                target: template.volume_mount_path.to_string(),
                read_only: false,
            }],
            command,
        })
    }

    pub fn binding_values(
        service: &ManagedServiceModel,
        encrypted_password: &str,
        prefix: &str,
    ) -> Vec<(String, String, bool)> {
        let mut values = vec![
            (
                format!("{prefix}_HOST"),
                service.container_name.clone(),
                false,
            ),
            (
                format!("{prefix}_PORT"),
                service.internal_port.to_string(),
                false,
            ),
        ];
        if let Some(database) = &service.database_name {
            values.push((format!("{prefix}_DATABASE"), database.clone(), false));
        }
        if let Some(username) = &service.username {
            values.push((format!("{prefix}_USER"), username.clone(), false));
        }
        values.push((
            format!("{prefix}_PASSWORD"),
            encrypted_password.to_string(),
            true,
        ));
        values
    }

    pub fn normalize_env_prefix(value: Option<&str>, fallback: &str) -> Result<String> {
        let prefix = value.unwrap_or(fallback).trim().to_ascii_uppercase();
        if prefix.is_empty()
            || prefix.len() > 48
            || !prefix
                .chars()
                .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
            || prefix
                .chars()
                .next()
                .map(|ch| ch.is_ascii_digit())
                .unwrap_or(true)
        {
            return Err(Error::BadRequest(
                "env_prefix must be 1-48 uppercase letters, digits, or underscores and cannot start with a digit"
                    .to_string(),
            ));
        }
        Ok(prefix)
    }
}

fn normalize_kind(value: &str) -> &str {
    match value.trim().to_ascii_lowercase().as_str() {
        "postgres" | "postgresql" => "postgresql",
        "mysql" => "mysql",
        "maria" | "mariadb" => "mariadb",
        "redis" | "redis-compatible" => "redis",
        _ => "",
    }
}

fn normalize_slug(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(48)
        .collect()
}

fn normalize_identifier(value: &str, field: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 63
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        || value
            .chars()
            .next()
            .map(|ch| ch.is_ascii_digit())
            .unwrap_or(true)
    {
        return Err(Error::BadRequest(format!(
            "{field} must be 1-63 letters, digits, or underscores and cannot start with a digit"
        )));
    }
    Ok(value.to_string())
}

fn validate_pinned_image(image: &str) -> Result<()> {
    DockerService::validate_image_name(image)
        .map_err(|error| Error::BadRequest(error.to_string()))?;
    if image.contains("@sha256:") {
        return Ok(());
    }
    let last = image.rsplit('/').next().unwrap_or(image);
    let Some((_, tag)) = last.rsplit_once(':') else {
        return Err(Error::BadRequest(
            "managed service image must include an explicit version tag or digest".to_string(),
        ));
    };
    if tag.is_empty() || tag.eq_ignore_ascii_case("latest") {
        return Err(Error::BadRequest(
            "managed service image cannot use an empty or latest tag".to_string(),
        ));
    }
    Ok(())
}

fn required<'a>(value: Option<&'a str>, field: &str) -> Result<&'a str> {
    value.ok_or_else(|| Error::BadRequest(format!("managed service {field} is missing")))
}

fn random_secret() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_are_explicitly_version_pinned() {
        for template in ManagedServiceTemplateService::TEMPLATES {
            assert!(validate_pinned_image(template.image).is_ok());
            assert!(!template.image.ends_with(":latest"));
        }
        assert!(validate_pinned_image("postgres").is_err());
        assert!(validate_pinned_image("postgres:latest").is_err());
    }

    #[test]
    fn aliases_normalize_to_supported_service_kinds() {
        assert_eq!(
            ManagedServiceTemplateService::template("postgres")
                .unwrap()
                .kind,
            "postgresql"
        );
        assert_eq!(
            ManagedServiceTemplateService::template("maria")
                .unwrap()
                .kind,
            "mariadb"
        );
        assert!(ManagedServiceTemplateService::template("mongodb").is_err());
    }

    #[test]
    fn env_prefix_is_strict_and_predictable() {
        assert_eq!(
            ManagedServiceTemplateService::normalize_env_prefix(Some("primary_db"), "POSTGRES")
                .unwrap(),
            "PRIMARY_DB"
        );
        assert!(
            ManagedServiceTemplateService::normalize_env_prefix(Some("1DB"), "POSTGRES").is_err()
        );
        assert!(
            ManagedServiceTemplateService::normalize_env_prefix(Some("DB-PROD"), "POSTGRES")
                .is_err()
        );
    }

    #[test]
    fn generated_credentials_have_high_entropy_hex_shape() {
        let prepared = ManagedServiceTemplateService::prepare(&CreateManagedServiceParams {
            server_id: 1,
            name: "Primary Database".to_string(),
            kind: "postgresql".to_string(),
            image: None,
            database_name: None,
            username: None,
        })
        .unwrap();
        assert_eq!(prepared.credentials.password.len(), 64);
        assert!(prepared
            .credentials
            .password
            .chars()
            .all(|ch| ch.is_ascii_hexdigit()));
        assert_ne!(prepared.credentials.password, random_secret());
    }
}
