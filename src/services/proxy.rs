#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    #[error("Invalid hostname: {0}")]
    InvalidHostname(String),
}

pub struct ProxyService;

impl ProxyService {
    /// Validate RFC 1123 compliant hostname
    pub fn validate_hostname(hostname: &str) -> Result<(), ProxyError> {
        let trimmed = hostname.trim();
        if trimmed.is_empty() || trimmed.len() > 253 {
            return Err(ProxyError::InvalidHostname(
                "Hostname length invalid".to_string(),
            ));
        }

        for label in trimmed.split('.') {
            if label.is_empty() || label.len() > 63 {
                return Err(ProxyError::InvalidHostname(format!(
                    "Invalid label '{}'",
                    label
                )));
            }
            if label.starts_with('-') || label.ends_with('-') {
                return Err(ProxyError::InvalidHostname(format!(
                    "Label '{}' cannot start or end with hyphen",
                    label
                )));
            }
            if !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                return Err(ProxyError::InvalidHostname(format!(
                    "Label '{}' contains invalid characters",
                    label
                )));
            }
        }

        Ok(())
    }

    /// Generate Traefik Docker labels for application routing
    pub fn generate_traefik_labels(
        app_slug: &str,
        domains: &[String],
        container_port: i32,
        https_enabled: bool,
    ) -> Vec<(String, String)> {
        let mut labels = vec![("traefik.enable".to_string(), "true".to_string())];

        if domains.is_empty() {
            return labels;
        }

        // Generate Host(`domain1`) || Host(`domain2`) rule
        let host_rules: Vec<String> = domains
            .iter()
            .map(|d| format!("Host(`{}`)", d.trim().to_lowercase()))
            .collect();
        let combined_rule = host_rules.join(" || ");

        let router_name = format!("moonships-{}", app_slug);

        labels.push((
            format!("traefik.http.routers.{}.rule", router_name),
            combined_rule,
        ));

        labels.push((
            format!(
                "traefik.http.services.{}.loadbalancer.server.port",
                router_name
            ),
            container_port.to_string(),
        ));

        if https_enabled {
            labels.push((
                format!("traefik.http.routers.{}.entrypoints", router_name),
                "websecure".to_string(),
            ));
            labels.push((
                format!("traefik.http.routers.{}.tls", router_name),
                "true".to_string(),
            ));
            labels.push((
                format!("traefik.http.routers.{}.tls.certresolver", router_name),
                "letsencrypt".to_string(),
            ));
        } else {
            labels.push((
                format!("traefik.http.routers.{}.entrypoints", router_name),
                "web".to_string(),
            ));
        }

        labels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_hostnames() {
        assert!(ProxyService::validate_hostname("app.example.com").is_ok());
        assert!(ProxyService::validate_hostname("my-service.internal.net").is_ok());
        assert!(ProxyService::validate_hostname("localhost").is_ok());
        assert!(ProxyService::validate_hostname("").is_err());
        assert!(ProxyService::validate_hostname("-bad.domain.com").is_err());
        assert!(ProxyService::validate_hostname("bad..domain.com").is_err());
    }

    #[test]
    fn test_traefik_labels_generation() {
        let labels = ProxyService::generate_traefik_labels(
            "my-app",
            &["app.example.com".to_string(), "api.example.com".to_string()],
            8080,
            true,
        );

        assert!(labels
            .iter()
            .any(|(k, v)| k == "traefik.enable" && v == "true"));
        assert!(labels
            .iter()
            .any(|(k, v)| k == "traefik.http.routers.moonships-my-app.rule"
                && v.contains("Host(`app.example.com`)")));
        assert!(labels.iter().any(|(k, v)| k
            == "traefik.http.services.moonships-my-app.loadbalancer.server.port"
            && v == "8080"));
        assert!(labels
            .iter()
            .any(|(k, v)| k == "traefik.http.routers.moonships-my-app.tls" && v == "true"));
    }
}
