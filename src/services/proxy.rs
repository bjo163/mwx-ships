#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    #[error("Invalid hostname: {0}")]
    InvalidHostname(String),
}

pub struct ProxyService;

impl ProxyService {
    pub const MANAGED_NETWORK: &'static str = "moonships-ingress";
    pub const MANAGED_PROXY_CONTAINER: &'static str = "moonships-traefik";

    pub fn managed_runtime_name(app_slug: &str, revision_hash: &str) -> String {
        let suffix = &revision_hash[..12.min(revision_hash.len())];
        format!("moonships-{app_slug}-{suffix}")
    }

    pub fn uses_managed_ingress(domain_count: usize, published_port: Option<i32>) -> bool {
        domain_count > 0 && published_port.is_none()
    }

    pub fn managed_route_config(
        app_slug: &str,
        runtime_name: &str,
        container_port: i32,
        https_domains: &[String],
        http_domains: &[String],
    ) -> serde_json::Value {
        let service_name = format!("moonships-{app_slug}");
        let mut routers = serde_json::Map::new();
        let mut middlewares = serde_json::Map::new();

        if !https_domains.is_empty() {
            routers.insert(
                format!("{service_name}-https"),
                serde_json::json!({
                    "rule": host_rule(https_domains),
                    "entryPoints": ["websecure"],
                    "service": service_name,
                    "tls": { "certResolver": "letsencrypt" }
                }),
            );
            routers.insert(
                format!("{service_name}-https-redirect"),
                serde_json::json!({
                    "rule": host_rule(https_domains),
                    "entryPoints": ["web"],
                    "service": service_name,
                    "middlewares": [format!("{service_name}-redirect")]
                }),
            );
            middlewares.insert(
                format!("{service_name}-redirect"),
                serde_json::json!({
                    "redirectScheme": {
                        "scheme": "https",
                        "permanent": true
                    }
                }),
            );
        }

        if !http_domains.is_empty() {
            routers.insert(
                format!("{service_name}-http"),
                serde_json::json!({
                    "rule": host_rule(http_domains),
                    "entryPoints": ["web"],
                    "service": service_name
                }),
            );
        }

        serde_json::json!({
            "http": {
                "routers": routers,
                "middlewares": middlewares,
                "services": {
                    service_name: {
                        "loadBalancer": {
                            "servers": [{
                                "url": format!("http://{runtime_name}:{container_port}")
                            }]
                        }
                    }
                }
            }
        })
    }

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

fn host_rule(domains: &[String]) -> String {
    let tick = char::from(96);
    domains
        .iter()
        .map(|domain| format!("Host({tick}{}{tick})", domain.trim().to_lowercase()))
        .collect::<Vec<_>>()
        .join(" || ")
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
    fn managed_ingress_requires_domains_without_host_port() {
        assert!(ProxyService::uses_managed_ingress(1, None));
        assert!(ProxyService::uses_managed_ingress(3, None));
        assert!(!ProxyService::uses_managed_ingress(0, None));
        assert!(!ProxyService::uses_managed_ingress(1, Some(8080)));
    }

    #[test]
    fn managed_runtime_name_is_revision_specific() {
        assert_eq!(
            ProxyService::managed_runtime_name("api", "0123456789abcdef"),
            "moonships-api-0123456789ab"
        );
    }

    #[test]
    fn managed_route_uses_atomic_service_target_and_https_redirect() {
        let config = ProxyService::managed_route_config(
            "my-app",
            "moonships-my-app-deadbeef",
            8080,
            &["secure.example.com".to_string()],
            &["plain.example.com".to_string()],
        );
        let json = config.to_string();
        assert!(json.contains("moonships-my-app-deadbeef:8080"));
        assert!(json.contains("secure.example.com"));
        assert!(json.contains("plain.example.com"));
        assert!(json.contains("redirectScheme"));
        assert!(json.contains("letsencrypt"));
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
