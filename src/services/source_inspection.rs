use crate::services::remote::RemoteSourceSnapshot;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize)]
pub struct SourceInspectionRequest {
    pub project_id: i64,
    pub server_id: i64,
    pub git_repository: String,
    pub git_branch: Option<String>,
}

impl SourceInspectionRequest {
    pub fn branch(&self) -> &str {
        self.git_branch
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("main")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceBuildStrategy {
    Compose,
    Dockerfile,
    Static,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InspectionConfidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectionReason {
    pub code: String,
    pub path: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceInspection {
    pub inspection_version: u32,
    pub commit_sha: String,
    pub strategy: SourceBuildStrategy,
    pub confidence: InspectionConfidence,
    pub candidates: Vec<SourceBuildStrategy>,
    pub dockerfile_path: Option<String>,
    pub compose_file_path: Option<String>,
    pub docker_context: Option<String>,
    pub container_port_hint: Option<u16>,
    pub healthcheck_path_hint: Option<String>,
    pub static_output_dir_hint: Option<String>,
    pub package_manager: Option<String>,
    pub framework_hint: Option<String>,
    pub reasons: Vec<DetectionReason>,
}

pub struct SourceInspectionService;

impl SourceInspectionService {
    pub fn inspect(snapshot: &RemoteSourceSnapshot) -> SourceInspection {
        let files: BTreeMap<&str, &str> = snapshot
            .files
            .iter()
            .map(|(path, content)| (path.as_str(), content.as_str()))
            .collect();

        let compose_paths = [
            "compose.yaml",
            "compose.yml",
            "docker-compose.yaml",
            "docker-compose.yml",
        ]
        .into_iter()
        .filter(|path| files.contains_key(path))
        .collect::<Vec<_>>();

        let dockerfile_paths = ["Dockerfile", "docker/Dockerfile", ".docker/Dockerfile"]
            .into_iter()
            .filter(|path| files.contains_key(path))
            .collect::<Vec<_>>();

        let package_json = files
            .get("package.json")
            .and_then(|content| serde_json::from_str::<Value>(content).ok());
        let vite = package_json
            .as_ref()
            .map(has_vite_build)
            .unwrap_or(false);
        let root_index = files.contains_key("index.html");
        let static_candidate = root_index || vite;

        let package_manager = detect_package_manager(&files);
        let framework_hint = vite.then(|| "vite".to_string());

        let mut reasons = Vec::new();
        let mut candidates = Vec::new();

        if !compose_paths.is_empty() {
            candidates.push(SourceBuildStrategy::Compose);
        }
        if !dockerfile_paths.is_empty() {
            candidates.push(SourceBuildStrategy::Dockerfile);
        }
        if static_candidate {
            candidates.push(SourceBuildStrategy::Static);
        }

        if compose_paths.len() > 1 {
            reasons.push(reason(
                "multiple_compose_files",
                None,
                "Multiple Compose definitions were found; choose one explicitly.",
            ));
            return SourceInspection {
                inspection_version: 1,
                commit_sha: snapshot.commit_sha.clone(),
                strategy: SourceBuildStrategy::Unresolved,
                confidence: InspectionConfidence::Low,
                candidates,
                dockerfile_path: dockerfile_paths.first().map(|path| (*path).to_string()),
                compose_file_path: None,
                docker_context: Some(".".to_string()),
                container_port_hint: dockerfile_paths
                    .first()
                    .and_then(|path| files.get(path))
                    .and_then(|content| dockerfile_exposed_port(content)),
                healthcheck_path_hint: dockerfile_paths
                    .first()
                    .and_then(|path| files.get(path))
                    .and_then(|content| dockerfile_healthcheck_path(content)),
                static_output_dir_hint: vite.then(|| "dist".to_string()).or_else(|| {
                    root_index.then(|| ".".to_string())
                }),
                package_manager,
                framework_hint,
                reasons,
            };
        }

        if let Some(compose_path) = compose_paths.first() {
            reasons.push(reason(
                "compose_file_found",
                Some(*compose_path),
                "A root Compose definition is the strongest explicit workload signal.",
            ));
            if let Some(dockerfile_path) = dockerfile_paths.first() {
                reasons.push(reason(
                    "dockerfile_supporting_compose",
                    Some(*dockerfile_path),
                    "A Dockerfile is present and may be referenced by the Compose workload.",
                ));
            }
            return SourceInspection {
                inspection_version: 1,
                commit_sha: snapshot.commit_sha.clone(),
                strategy: SourceBuildStrategy::Compose,
                confidence: InspectionConfidence::High,
                candidates,
                dockerfile_path: dockerfile_paths.first().map(|path| (*path).to_string()),
                compose_file_path: Some((*compose_path).to_string()),
                docker_context: Some(".".to_string()),
                container_port_hint: dockerfile_paths
                    .first()
                    .and_then(|path| files.get(path))
                    .and_then(|content| dockerfile_exposed_port(content)),
                healthcheck_path_hint: dockerfile_paths
                    .first()
                    .and_then(|path| files.get(path))
                    .and_then(|content| dockerfile_healthcheck_path(content)),
                static_output_dir_hint: None,
                package_manager,
                framework_hint,
                reasons,
            };
        }

        if dockerfile_paths.len() > 1 {
            reasons.push(reason(
                "multiple_dockerfiles",
                None,
                "Multiple supported Dockerfile locations were found; choose one explicitly.",
            ));
            return SourceInspection {
                inspection_version: 1,
                commit_sha: snapshot.commit_sha.clone(),
                strategy: SourceBuildStrategy::Unresolved,
                confidence: InspectionConfidence::Low,
                candidates,
                dockerfile_path: None,
                compose_file_path: None,
                docker_context: Some(".".to_string()),
                container_port_hint: None,
                healthcheck_path_hint: None,
                static_output_dir_hint: vite.then(|| "dist".to_string()).or_else(|| {
                    root_index.then(|| ".".to_string())
                }),
                package_manager,
                framework_hint,
                reasons,
            };
        }

        if let Some(dockerfile_path) = dockerfile_paths.first() {
            reasons.push(reason(
                "dockerfile_found",
                Some(*dockerfile_path),
                "A supported Dockerfile was found.",
            ));
            let port = files
                .get(dockerfile_path)
                .and_then(|content| dockerfile_exposed_port(content));
            let healthcheck_path = files
                .get(dockerfile_path)
                .and_then(|content| dockerfile_healthcheck_path(content));
            if port.is_some() {
                reasons.push(reason(
                    "dockerfile_expose_found",
                    Some(*dockerfile_path),
                    "A numeric EXPOSE instruction provides a container-port hint.",
                ));
            }
            if healthcheck_path.is_some() {
                reasons.push(reason(
                    "dockerfile_healthcheck_found",
                    Some(*dockerfile_path),
                    "An HTTP(S) Dockerfile HEALTHCHECK provides a path hint.",
                ));
            }
            return SourceInspection {
                inspection_version: 1,
                commit_sha: snapshot.commit_sha.clone(),
                strategy: SourceBuildStrategy::Dockerfile,
                confidence: InspectionConfidence::High,
                candidates,
                dockerfile_path: Some((*dockerfile_path).to_string()),
                compose_file_path: None,
                docker_context: Some(".".to_string()),
                container_port_hint: port,
                healthcheck_path_hint: healthcheck_path,
                static_output_dir_hint: None,
                package_manager,
                framework_hint,
                reasons,
            };
        }

        if static_candidate {
            if root_index {
                reasons.push(reason(
                    "root_index_html_found",
                    Some("index.html"),
                    "A root index.html is a direct static-site signal.",
                ));
            }
            if vite {
                reasons.push(reason(
                    "vite_build_detected",
                    Some("package.json"),
                    "package.json declares Vite and a build script; scripts are not executed during inspection.",
                ));
            }
            return SourceInspection {
                inspection_version: 1,
                commit_sha: snapshot.commit_sha.clone(),
                strategy: SourceBuildStrategy::Static,
                confidence: if root_index {
                    InspectionConfidence::High
                } else {
                    InspectionConfidence::Medium
                },
                candidates,
                dockerfile_path: None,
                compose_file_path: None,
                docker_context: Some(".".to_string()),
                container_port_hint: None,
                healthcheck_path_hint: Some("/".to_string()),
                static_output_dir_hint: Some(if vite { "dist" } else { "." }.to_string()),
                package_manager,
                framework_hint,
                reasons,
            };
        }

        if package_json.is_some() {
            reasons.push(reason(
                "node_project_requires_explicit_build",
                Some("package.json"),
                "A Node project was found, but no safe supported build strategy can be inferred.",
            ));
        } else {
            reasons.push(reason(
                "no_supported_build_definition",
                None,
                "No supported Compose, Dockerfile, or static-site signal was found.",
            ));
        }

        SourceInspection {
            inspection_version: 1,
            commit_sha: snapshot.commit_sha.clone(),
            strategy: SourceBuildStrategy::Unresolved,
            confidence: InspectionConfidence::Low,
            candidates,
            dockerfile_path: None,
            compose_file_path: None,
            docker_context: Some(".".to_string()),
            container_port_hint: None,
            healthcheck_path_hint: None,
            static_output_dir_hint: None,
            package_manager,
            framework_hint,
            reasons,
        }
    }
}

fn has_vite_build(package: &Value) -> bool {
    let has_vite = ["dependencies", "devDependencies"]
        .iter()
        .filter_map(|key| package.get(*key))
        .filter_map(Value::as_object)
        .any(|deps| deps.contains_key("vite"));
    let has_build = package
        .get("scripts")
        .and_then(Value::as_object)
        .and_then(|scripts| scripts.get("build"))
        .and_then(Value::as_str)
        .map(|script| !script.trim().is_empty())
        .unwrap_or(false);
    has_vite && has_build
}

fn detect_package_manager(files: &BTreeMap<&str, &str>) -> Option<String> {
    let managers = [
        ("pnpm-lock.yaml", "pnpm"),
        ("yarn.lock", "yarn"),
        ("package-lock.json", "npm"),
        ("bun.lock", "bun"),
        ("bun.lockb", "bun"),
    ]
    .into_iter()
    .filter(|(path, _)| files.contains_key(path))
    .map(|(_, manager)| manager)
    .collect::<Vec<_>>();

    if managers.is_empty() {
        return None;
    }
    let first = managers[0];
    managers
        .iter()
        .all(|manager| *manager == first)
        .then(|| first.to_string())
}

fn dockerfile_exposed_port(content: &str) -> Option<u16> {
    for line in content.lines() {
        let instruction = line.split('#').next().unwrap_or("").trim();
        let mut parts = instruction.split_whitespace();
        if !parts
            .next()
            .map(|value| value.eq_ignore_ascii_case("EXPOSE"))
            .unwrap_or(false)
        {
            continue;
        }
        for value in parts {
            let value = value
                .split('/')
                .next()
                .unwrap_or(value)
                .trim_matches(|ch: char| !ch.is_ascii_digit());
            if let Ok(port) = value.parse::<u16>() {
                if port > 0 {
                    return Some(port);
                }
            }
        }
    }
    None
}

fn dockerfile_healthcheck_path(content: &str) -> Option<String> {
    for line in content.lines() {
        let instruction = line.trim();
        if !instruction
            .split_whitespace()
            .next()
            .map(|value| value.eq_ignore_ascii_case("HEALTHCHECK"))
            .unwrap_or(false)
        {
            continue;
        }

        for scheme in ["http://", "https://"] {
            if let Some(start) = instruction.find(scheme) {
                let rest = &instruction[start + scheme.len()..];
                let path = rest.find('/').map(|index| &rest[index..]).unwrap_or("/");
                let path = path
                    .split_whitespace()
                    .next()
                    .unwrap_or("/")
                    .trim_matches(|ch| matches!(ch, '\'' | '"' | ')' | ']' | ';'));
                if path.starts_with('/')
                    && path.len() <= 256
                    && !path.contains(['\n', '\r', '\0'])
                {
                    return Some(path.to_string());
                }
            }
        }
    }
    None
}

fn reason(code: &str, path: Option<&str>, message: &str) -> DetectionReason {
    DetectionReason {
        code: code.to_string(),
        path: path.map(str::to_string),
        message: message.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(files: &[(&str, &str)]) -> RemoteSourceSnapshot {
        RemoteSourceSnapshot {
            commit_sha: "0123456789abcdef".to_string(),
            files: files
                .iter()
                .map(|(path, content)| ((*path).to_string(), (*content).to_string()))
                .collect(),
        }
    }

    #[test]
    fn detects_dockerfile_and_exposed_port() {
        let result = SourceInspectionService::inspect(&snapshot(&[(
            "Dockerfile",
            "FROM alpine\nEXPOSE 8080/tcp\nHEALTHCHECK CMD curl -fsS http://localhost:8080/health || exit 1\nCMD [\"app\"]\n",
        )]));
        assert_eq!(result.strategy, SourceBuildStrategy::Dockerfile);
        assert_eq!(result.confidence, InspectionConfidence::High);
        assert_eq!(result.dockerfile_path.as_deref(), Some("Dockerfile"));
        assert_eq!(result.container_port_hint, Some(8080));
        assert_eq!(result.healthcheck_path_hint.as_deref(), Some("/health"));
    }

    #[test]
    fn compose_is_authoritative_when_it_has_a_supporting_dockerfile() {
        let result = SourceInspectionService::inspect(&snapshot(&[
            ("compose.yaml", "services:\n  web:\n    build: .\n"),
            ("Dockerfile", "FROM alpine\nEXPOSE 3000\n"),
        ]));
        assert_eq!(result.strategy, SourceBuildStrategy::Compose);
        assert_eq!(result.compose_file_path.as_deref(), Some("compose.yaml"));
        assert!(result
            .reasons
            .iter()
            .any(|reason| reason.code == "dockerfile_supporting_compose"));
    }

    #[test]
    fn multiple_compose_files_are_explicitly_unresolved() {
        let result = SourceInspectionService::inspect(&snapshot(&[
            ("compose.yaml", "services: {}\n"),
            ("docker-compose.yml", "services: {}\n"),
        ]));
        assert_eq!(result.strategy, SourceBuildStrategy::Unresolved);
        assert_eq!(result.confidence, InspectionConfidence::Low);
        assert!(result
            .reasons
            .iter()
            .any(|reason| reason.code == "multiple_compose_files"));
    }

    #[test]
    fn detects_vite_without_executing_package_scripts() {
        let result = SourceInspectionService::inspect(&snapshot(&[
            (
                "package.json",
                r#"{"scripts":{"build":"touch /tmp/never-run && vite build"},"devDependencies":{"vite":"^8"}}"#,
            ),
            ("pnpm-lock.yaml", "lockfileVersion: '9.0'\n"),
        ]));
        assert_eq!(result.strategy, SourceBuildStrategy::Static);
        assert_eq!(result.confidence, InspectionConfidence::Medium);
        assert_eq!(result.package_manager.as_deref(), Some("pnpm"));
        assert_eq!(result.static_output_dir_hint.as_deref(), Some("dist"));
    }

    #[test]
    fn generic_node_project_remains_unresolved() {
        let result = SourceInspectionService::inspect(&snapshot(&[(
            "package.json",
            r#"{"scripts":{"start":"node server.js"},"dependencies":{"express":"^5"}}"#,
        )]));
        assert_eq!(result.strategy, SourceBuildStrategy::Unresolved);
        assert!(result
            .reasons
            .iter()
            .any(|reason| reason.code == "node_project_requires_explicit_build"));
    }
}
