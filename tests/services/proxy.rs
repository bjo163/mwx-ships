use moonships::services::proxy::ProxyService;

#[test]
fn test_valid_hostnames() {
    assert!(ProxyService::validate_hostname("example.com").is_ok());
    assert!(ProxyService::validate_hostname("api.cool-app.internal.io").is_ok());
    assert!(ProxyService::validate_hostname("localhost").is_ok());
}

#[test]
fn test_invalid_hostnames() {
    assert!(ProxyService::validate_hostname("-leading-dash.com").is_err());
    assert!(ProxyService::validate_hostname("trailing-dash-.com").is_err());
    assert!(ProxyService::validate_hostname("invalid_underscore.com").is_err());
    assert!(ProxyService::validate_hostname("host name with spaces").is_err());
    assert!(ProxyService::validate_hostname("").is_err());
}

#[test]
fn test_traefik_labels_generation() {
    let domains = vec!["app.example.com".to_string(), "api.example.com".to_string()];
    let labels = ProxyService::generate_traefik_labels("my-web-app", &domains, 8080, true);

    let labels_map: std::collections::HashMap<String, String> = labels.into_iter().collect();

    assert_eq!(labels_map.get("traefik.enable").unwrap(), "true");
    assert_eq!(
        labels_map
            .get("traefik.http.routers.moonships-my-web-app.rule")
            .unwrap(),
        "Host(`app.example.com`) || Host(`api.example.com`)"
    );
    assert_eq!(
        labels_map
            .get("traefik.http.services.moonships-my-web-app.loadbalancer.server.port")
            .unwrap(),
        "8080"
    );
    assert_eq!(
        labels_map
            .get("traefik.http.routers.moonships-my-web-app.tls")
            .unwrap(),
        "true"
    );
}
