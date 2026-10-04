use std::time::Duration;
use axum::http::HeaderMap;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Sha256;

use crate::models::git_integrations;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, thiserror::Error)]
pub enum GitProviderError {
    #[error("unsupported provider '{0}'")]
    UnsupportedProvider(String),
    #[error("missing webhook header '{0}'")]
    MissingHeader(String),
    #[error("invalid webhook signature")]
    InvalidSignature,
    #[error("invalid webhook payload: {0}")]
    InvalidPayload(String),
    #[error("provider token is not configured")]
    MissingToken,
    #[error("provider API base URL is required for Gitea")]
    MissingApiBaseUrl,
    #[error("provider status API returned HTTP {status}: {message}")]
    StatusApi { status: u16, message: String },
    #[error("provider HTTP request failed: {0}")]
    Http(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitWebhookEvent {
    pub provider: String,
    pub delivery_id: String,
    pub event_kind: String,
    pub source_ref: Option<String>,
    pub commit_sha: Option<String>,
    pub external_request_id: Option<String>,
    pub action: Option<String>,
    pub closed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitStatus {
    Pending,
    Success,
    Failure,
}

impl CommitStatus {
    fn github_value(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Success => "success",
            Self::Failure => "failure",
        }
    }

    fn gitlab_value(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Success => "success",
            Self::Failure => "failed",
        }
    }
}

pub struct GitProviderService;

impl GitProviderService {
    pub fn verify_and_parse(
        provider: &str,
        headers: &HeaderMap,
        body: &[u8],
        secret: &str,
    ) -> Result<GitWebhookEvent, GitProviderError> {
        let provider = normalize_provider(provider)?;
        let (delivery_id, event_header) = match provider.as_str() {
            "github" => {
                verify_hmac_header(headers, "x-hub-signature-256", "sha256=", body, secret)?;
                (
                    required_header(headers, "x-github-delivery")?,
                    required_header(headers, "x-github-event")?,
                )
            }
            "gitlab" => {
                verify_shared_token(headers, "x-gitlab-token", secret)?;
                (
                    required_header(headers, "x-gitlab-event-uuid")
                        .or_else(|_| required_header(headers, "x-request-id"))?,
                    required_header(headers, "x-gitlab-event")?,
                )
            }
            "gitea" => {
                verify_hmac_header(headers, "x-gitea-signature", "", body, secret)?;
                (
                    required_header(headers, "x-gitea-delivery")?,
                    required_header(headers, "x-gitea-event")?,
                )
            }
            _ => unreachable!(),
        };

        let payload: Value = serde_json::from_slice(body)
            .map_err(|err| GitProviderError::InvalidPayload(err.to_string()))?;

        let mut event = parse_payload(&provider, &event_header, &payload)?;
        event.delivery_id = delivery_id;
        Ok(event)
    }

    pub async fn set_commit_status(
        integration: &git_integrations::Model,
        commit_sha: &str,
        status: CommitStatus,
        description: &str,
        target_url: Option<&str>,
    ) -> Result<(), GitProviderError> {
        validate_commit_sha(commit_sha)?;
        let token = integration
            .token()
            .map_err(|err| GitProviderError::Http(err.to_string()))?
            .ok_or(GitProviderError::MissingToken)?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|err| GitProviderError::Http(err.to_string()))?;

        let response = match integration.provider.as_str() {
            "github" => {
                let base = integration
                    .api_base_url
                    .as_deref()
                    .unwrap_or("https://api.github.com")
                    .trim_end_matches('/');
                let url = format!(
                    "{base}/repos/{}/statuses/{commit_sha}",
                    integration.repository_ref
                );
                let mut body = serde_json::json!({
                    "state": status.github_value(),
                    "context": "moonships/deploy",
                    "description": description,
                });
                if let Some(target_url) = target_url {
                    body["target_url"] = Value::String(target_url.to_string());
                }
                client
                    .post(url)
                    .bearer_auth(&token)
                    .header("Accept", "application/vnd.github+json")
                    .header("X-GitHub-Api-Version", "2022-11-28")
                    .json(&body)
                    .send()
                    .await
            }
            "gitlab" => {
                let base = integration
                    .api_base_url
                    .as_deref()
                    .unwrap_or("https://gitlab.com/api/v4")
                    .trim_end_matches('/');
                let project = percent_encode_path(&integration.repository_ref);
                let url = format!("{base}/projects/{project}/statuses/{commit_sha}");
                let mut request = client.post(url).header("PRIVATE-TOKEN", &token).form(&[
                    ("state", status.gitlab_value()),
                    ("name", "moonships/deploy"),
                    ("description", description),
                ]);
                if let Some(target_url) = target_url {
                    request = request.query(&[("target_url", target_url)]);
                }
                request.send().await
            }
            "gitea" => {
                let base = integration
                    .api_base_url
                    .as_deref()
                    .ok_or(GitProviderError::MissingApiBaseUrl)?
                    .trim_end_matches('/');
                let url = format!(
                    "{base}/repos/{}/statuses/{commit_sha}",
                    integration.repository_ref
                );
                let mut body = serde_json::json!({
                    "state": status.github_value(),
                    "context": "moonships/deploy",
                    "description": description,
                });
                if let Some(target_url) = target_url {
                    body["target_url"] = Value::String(target_url.to_string());
                }
                client
                    .post(url)
                    .header("Authorization", format!("token {token}"))
                    .json(&body)
                    .send()
                    .await
            }
            provider => return Err(GitProviderError::UnsupportedProvider(provider.to_string())),
        }
        .map_err(|err| GitProviderError::Http(err.to_string()))?;

        if response.status().is_success() {
            return Ok(());
        }

        let status_code = response.status().as_u16();
        let message = response.text().await.unwrap_or_default();
        Err(GitProviderError::StatusApi {
            status: status_code,
            message: truncate_message(&message),
        })
    }
}

fn parse_payload(
    provider: &str,
    event_header: &str,
    payload: &Value,
) -> Result<GitWebhookEvent, GitProviderError> {
    match provider {
        "github" | "gitea" => parse_github_like(provider, event_header, payload),
        "gitlab" => parse_gitlab(event_header, payload),
        other => Err(GitProviderError::UnsupportedProvider(other.to_string())),
    }
}

fn parse_github_like(
    provider: &str,
    event_header: &str,
    payload: &Value,
) -> Result<GitWebhookEvent, GitProviderError> {
    match event_header {
        "push" => Ok(GitWebhookEvent {
            provider: provider.to_string(),
            delivery_id: String::new(),
            event_kind: "push".to_string(),
            source_ref: json_string(payload, &["ref"]).map(strip_heads),
            commit_sha: json_string(payload, &["after"]),
            external_request_id: None,
            action: None,
            closed: false,
        }),
        "pull_request" => {
            let action = json_string(payload, &["action"]);
            Ok(GitWebhookEvent {
                provider: provider.to_string(),
                delivery_id: String::new(),
                event_kind: "pull_request".to_string(),
                source_ref: json_string(payload, &["pull_request", "head", "ref"]),
                commit_sha: json_string(payload, &["pull_request", "head", "sha"]),
                external_request_id: payload
                    .get("number")
                    .and_then(Value::as_i64)
                    .map(|value| value.to_string()),
                closed: action
                    .as_deref()
                    .map(|value| matches!(value, "closed" | "merged"))
                    .unwrap_or(false),
                action,
            })
        }
        _ => Ok(GitWebhookEvent {
            provider: provider.to_string(),
            delivery_id: String::new(),
            event_kind: "ignored".to_string(),
            source_ref: None,
            commit_sha: None,
            external_request_id: None,
            action: Some(event_header.to_string()),
            closed: false,
        }),
    }
}

fn parse_gitlab(
    event_header: &str,
    payload: &Value,
) -> Result<GitWebhookEvent, GitProviderError> {
    match event_header {
        "Push Hook" => Ok(GitWebhookEvent {
            provider: "gitlab".to_string(),
            delivery_id: String::new(),
            event_kind: "push".to_string(),
            source_ref: json_string(payload, &["ref"]).map(strip_heads),
            commit_sha: json_string(payload, &["checkout_sha"])
                .or_else(|| json_string(payload, &["after"])),
            external_request_id: None,
            action: None,
            closed: false,
        }),
        "Merge Request Hook" => {
            let action = json_string(payload, &["object_attributes", "action"]);
            Ok(GitWebhookEvent {
                provider: "gitlab".to_string(),
                delivery_id: String::new(),
                event_kind: "pull_request".to_string(),
                source_ref: json_string(payload, &["object_attributes", "source_branch"]),
                commit_sha: json_string(payload, &["object_attributes", "last_commit", "id"])
                    .or_else(|| json_string(payload, &["last_commit", "id"])),
                external_request_id: payload
                    .pointer("/object_attributes/iid")
                    .and_then(Value::as_i64)
                    .map(|value| value.to_string()),
                closed: action
                    .as_deref()
                    .map(|value| matches!(value, "close" | "merge" | "merged"))
                    .unwrap_or(false),
                action,
            })
        }
        _ => Ok(GitWebhookEvent {
            provider: "gitlab".to_string(),
            delivery_id: String::new(),
            event_kind: "ignored".to_string(),
            source_ref: None,
            commit_sha: None,
            external_request_id: None,
            action: Some(event_header.to_string()),
            closed: false,
        }),
    }
}

fn required_header(headers: &HeaderMap, name: &str) -> Result<String, GitProviderError> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| GitProviderError::MissingHeader(name.to_string()))
}

fn verify_hmac_header(
    headers: &HeaderMap,
    name: &str,
    prefix: &str,
    body: &[u8],
    secret: &str,
) -> Result<(), GitProviderError> {
    let signature = required_header(headers, name)?;
    let signature = signature
        .strip_prefix(prefix)
        .ok_or(GitProviderError::InvalidSignature)?;
    let signature =
        hex::decode(signature).map_err(|_| GitProviderError::InvalidSignature)?;
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).map_err(|_| GitProviderError::InvalidSignature)?;
    mac.update(body);
    mac.verify_slice(&signature)
        .map_err(|_| GitProviderError::InvalidSignature)
}

fn verify_shared_token(
    headers: &HeaderMap,
    name: &str,
    secret: &str,
) -> Result<(), GitProviderError> {
    let provided = required_header(headers, name)?;
    if constant_time_eq(provided.as_bytes(), secret.as_bytes()) {
        Ok(())
    } else {
        Err(GitProviderError::InvalidSignature)
    }
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0u8, |diff, (a, b)| diff | (a ^ b))
        == 0
}

fn json_string(payload: &Value, path: &[&str]) -> Option<String> {
    let mut current = payload;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_str().map(str::to_string)
}

fn strip_heads(value: String) -> String {
    value
        .strip_prefix("refs/heads/")
        .unwrap_or(&value)
        .to_string()
}

fn normalize_provider(provider: &str) -> Result<String, GitProviderError> {
    let provider = provider.trim().to_ascii_lowercase();
    match provider.as_str() {
        "github" | "gitlab" | "gitea" => Ok(provider),
        _ => Err(GitProviderError::UnsupportedProvider(provider)),
    }
}

fn validate_commit_sha(value: &str) -> Result<(), GitProviderError> {
    if (7..=64).contains(&value.len()) && value.chars().all(|ch| ch.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(GitProviderError::InvalidPayload(
            "commit SHA is malformed".to_string(),
        ))
    }
}

fn percent_encode_path(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

fn truncate_message(value: &str) -> String {
    value.chars().take(512).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn github_headers(signature: String) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-hub-signature-256",
            HeaderValue::from_str(&signature).unwrap(),
        );
        headers.insert("x-github-delivery", HeaderValue::from_static("delivery-1"));
        headers.insert("x-github-event", HeaderValue::from_static("push"));
        headers
    }

    #[test]
    fn verifies_github_hmac_and_extracts_push() {
        let secret = "0123456789abcdef";
        let body = br#"{"ref":"refs/heads/main","after":"0123456789abcdef"}"#;
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let signature = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));

        let event =
            GitProviderService::verify_and_parse("github", &github_headers(signature), body, secret)
                .unwrap();

        assert_eq!(event.delivery_id, "delivery-1");
        assert_eq!(event.event_kind, "push");
        assert_eq!(event.source_ref.as_deref(), Some("main"));
        assert_eq!(event.commit_sha.as_deref(), Some("0123456789abcdef"));
    }

    #[test]
    fn rejects_invalid_hmac() {
        let body = br#"{"ref":"refs/heads/main","after":"0123456789abcdef"}"#;
        let result = GitProviderService::verify_and_parse(
            "github",
            &github_headers(format!("sha256={}", "00".repeat(32))),
            body,
            "0123456789abcdef",
        );
        assert!(matches!(result, Err(GitProviderError::InvalidSignature)));
    }

    #[test]
    fn parses_gitlab_merge_request_close() {
        let mut headers = HeaderMap::new();
        headers.insert("x-gitlab-token", HeaderValue::from_static("0123456789abcdef"));
        headers.insert("x-gitlab-event-uuid", HeaderValue::from_static("uuid-1"));
        headers.insert(
            "x-gitlab-event",
            HeaderValue::from_static("Merge Request Hook"),
        );
        let body = br#"{
            "object_attributes":{
                "iid":42,
                "action":"merge",
                "source_branch":"feature/x",
                "last_commit":{"id":"abcdef0123456789"}
            }
        }"#;

        let event = GitProviderService::verify_and_parse(
            "gitlab",
            &headers,
            body,
            "0123456789abcdef",
        )
        .unwrap();

        assert_eq!(event.external_request_id.as_deref(), Some("42"));
        assert_eq!(event.source_ref.as_deref(), Some("feature/x"));
        assert!(event.closed);
    }

    #[test]
    fn encodes_gitlab_project_path() {
        assert_eq!(percent_encode_path("group/sub/repo"), "group%2Fsub%2Frepo");
    }
}
