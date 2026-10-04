# Changelog

All notable changes to **Moonships** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-10-04

### Added
- **Authenticated Control Plane**: Operational server, project, application, deployment, environment, domain, and container lifecycle APIs require JWT; `/api/health` remains public.
- **Protected Dashboard Routes**: Operational SPA pages are guarded by `RequireAuth`; the shared API client sends bearer tokens and clears stale tokens on HTTP 401.
- **Selected-Server Remote Execution**: Git synchronization, Docker build/pull/run/lifecycle, logs/status, and healthchecks execute on the selected remote Linux server over SSH.
- **SSH Host Verification**: Strict host-key checking uses an isolated `known_hosts` file; operators can optionally pin a SHA256 host fingerprint.
- **Remote Failure Classification**: Stable error codes distinguish SSH connection, remote Git, remote Docker, secret decryption, and healthcheck failures.
- **Remote Regression Coverage**: Tests enforce that deployment orchestration and application lifecycle do not fall back to the control-plane Docker daemon.

### Security
- SSH private keys are decrypted only into temporary files with restrictive permissions for the SSH session lifetime.
- Remote command execution has bounded timeouts and validates SSH targets, Docker names/paths, and environment-variable keys.
- Secret environment values are transferred over SSH stdin into a temporary target-side env file and redacted from surfaced logs/errors.
- The production control plane no longer mounts `/var/run/docker.sock` and does not need the Docker CLI for application lifecycle operations.
- Operational API authentication closes the v0.1 unauthenticated-management gap.

### Changed
- Deployment lifecycle is now `queued -> connecting -> cloning -> building -> stopping_old -> starting_new -> healthchecking -> success|failed`.
- Server preflight uses the configured encrypted SSH key and reports SSH, Docker daemon, disk, memory, and CPU status from the selected target.
- Release-image verification checks the SSH client rather than local Docker tooling.

### Known limitations
- Container replacement remains **stop-old -> start-new**; v0.2.0 does not claim zero-downtime rollout.
- The control plane remains SQLite-first and single-node; multi-writer clustering and organization RBAC are future work.
- Initial SSH host-key discovery without an explicitly pinned fingerprint is trust-on-first-use; production operators should pin fingerprints out of band.

## [0.1.0] - 2026-10-04

### Added
- **SQLite-First Architecture**: Persistent single-file database (`data/moonships.sqlite`) with zero external DB requirements.
- **Persistent Deployment Queue**: Loco `BackgroundQueue` with SQLite engine ensuring worker tasks survive crashes.
- **Core Domain Entities**: Full SeaORM models and migrations for `Server`, `Project`, `Environment`, `Application`, `EnvironmentVariable`, `Domain`, `Deployment`, and `DeploymentLog`.
- **Infrastructure Services**:
  - `CryptoService`: AES-256-GCM authenticated encryption for SSH keys and environment variables.
  - `GitService`: Generic Git repository clone, checkout, and commit SHA inspection.
  - `SshService`: SSH connectivity, host validation, and remote execution.
  - `DockerService`: Structured container lifecycle management (build, pull, start, stop, restart, inspect, logs).
  - `ProxyService`: Traefik dynamic routing labels and domain configuration.
  - `DeploymentService`: End-to-end orchestration coordinating Git, SSH, Docker, and Proxy services.
- **Asynchronous Deployment Engine**:
  - `DeploymentWorker`: Persistent background worker executing the deployment pipeline.
  - Deployment State Machine: `queued` -> `cloning` -> `building` -> `stopping_old` -> `starting_new` -> `healthchecking` -> `success`.
  - Deployment concurrency locking: single active deployment per application with 409 Conflict rejection.
  - Structured log streaming: persists sequential logs separated by stdout, stderr, and system.
  - HTTP healthcheck verification loop with configurable retry and timeout.
- **Server Preflight**: Comprehensive remote server validation endpoint (`/api/servers/:id/preflight`).
- **Modern Dashboard**: React Single Page Application dashboard with real-time deployment status, logs viewer, and server management.
- **Antigravity Integration**: Persistent rules in `AGENTS.md`, custom agents (`moonships-builder`, `moonships-reviewer`, `moonships-security`), and skills (`moonships-development`, `moonships-testing`, `moonships-github`, `moonships-release`).
- **Comprehensive Documentation**: Architectural decision records (ADRs 0001-0005) and operational runbooks in `docs/`.

### Release hardening
- Production Compose now requires explicit `ENCRYPTION_KEY` and `JWT_SECRET`; no repository-known secret fallback is allowed.
- Production SMTP is opt-in instead of blocking startup when mail settings are absent.
- Runtime image includes the Docker CLI required by the v0.1 local-host deployment engine.
- Docker CI validates Compose and smoke-tests the production image health endpoint.
- Documentation now states the v0.1 single-host execution boundary and tracks remote target execution in #46.
- Docker Compose publishes the control plane to loopback only by default while management API JWT enforcement is tracked in #47.
