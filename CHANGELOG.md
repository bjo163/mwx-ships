# Changelog

All notable changes to **Moonships** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2026-10-04

### Added
- **Managed Ingress Runtime**: Moonships can maintain a target-side Traefik container and dedicated `moonships-ingress` Docker network for applications with domains and no published host port.
- **Blue/Green Runtime Slots**: Revision-specific candidate containers run beside the active runtime instead of stopping production first.
- **Health-Before-Traffic**: Managed candidates must pass their application healthcheck before any route is changed.
- **Atomic Traffic Switch**: Traefik file-provider configuration is written to a temporary file and atomically renamed into place, then the old runtime is drained.
- **Domain Readiness State**: Domains persist DNS verification, verification time, TLS state, and the last verification error.
- **Domain Verification API/UI**: Operators can verify DNS target state from the application dashboard.
- **Managed ACME HTTP-01**: HTTPS-enabled managed domains use the configured Let's Encrypt resolver and HTTP-to-HTTPS redirects.
- **Active/Candidate Runtime Visibility**: Applications persist active and candidate runtime names for recovery, lifecycle operations, and operator visibility.

### Reliability
- An unhealthy candidate is removed without replacing the current active runtime.
- Managed deployments use deterministic candidate names derived from the immutable revision, allowing worker recovery to reconcile an already-running candidate.
- Current runtime promotion happens only after candidate health and route switch succeed.
- Explicit published-port applications retain the v0.3 compatibility replacement path.

### Security
- Managed ingress route writes use SSH stdin plus atomic file replacement.
- ACME state is created with restrictive target-side permissions.
- DNS verification runs on the selected target before managed traffic is switched.
- Lifecycle APIs resolve the active managed runtime rather than blindly addressing the legacy container name.

### Configuration
- `MOONSHIPS_TRAEFIK_IMAGE` selects the managed Traefik image (default `traefik:v3.1`).
- `MOONSHIPS_ACME_EMAIL` is required when a managed revision contains HTTPS domains.

### Known limitations
- Zero-downtime semantics apply only to applications using managed ingress (domains present, no published host port).
- v0.4 supports Let's Encrypt HTTP-01; DNS-01/provider automation remains future work.
- External/custom Traefik installations are not automatically adopted as Moonships-managed infrastructure.

## [0.3.0] - 2026-10-04

### Added
- **Immutable Deployment Revisions**: Every deployable state receives a deterministic revision identity covering source commit, target, runtime/build configuration, environment fingerprints, domains, and image reference.
- **Known-Good Revision Tracking**: Applications retain explicit `current_revision_id` and `previous_revision_id` pointers so rollback targets are deterministic.
- **Safe Cancellation & Retry**: Deployments can be cancelled during non-destructive phases and retried as new immutable attempts with source-deployment provenance.
- **One-Click Rollback**: Rollback queues the previous known-good revision as a new deployment instead of mutating historical deployment records.
- **Execution Leases**: Workers claim deployments with an execution token and bounded lease; duplicate queue deliveries become no-ops while stale executions can be reclaimed after interruption.
- **Deployment Provenance**: Deployment attempts record trigger kind (`manual`, `retry`, `rollback`) and source deployment identity for auditability.
- **Retention Policy**: Revision artifacts, old deployment logs, and target disk pressure are managed with configurable bounded cleanup while current/previous known-good revisions remain protected.
- **Dashboard Recovery Controls**: Deployment views expose cancel/retry actions, revision state, and rollback controls.

### Reliability
- Exact retry/rollback execution uses the immutable revision source commit and runtime snapshot instead of whatever configuration happens to be current later.
- Dockerfile deployments use revision-specific image references.
- Failed candidate containers/images are cleaned on startup/healthcheck failure without deleting previously healthy revision artifacts.
- SQLite migrations remain additive/backward-safe for existing v0.2 databases.
- CI now cancels superseded `dev` workflow runs so only the newest integration head consumes full gate capacity.

### Security
- Revision snapshots never persist secret plaintext. Secret material required for exact rollback remains AES-256-GCM ciphertext with an integrity fingerprint.
- Retry/rollback source commits are validated before target-side detached checkout.
- Cancellation fails closed once the destructive replacement phase begins.
- Worker phase transitions are guarded by execution ownership so superseded workers cannot advance a newer lease.

### Changed
- Deployment lifecycle now supports the terminal `cancelled` state in addition to `success` and `failed`.
- A deployment retry is always a new Deployment row; historical attempts are never reopened.
- Rollback is represented as a normal auditable deployment attempt.
- Application detail responses/UI surface current and previous known-good revision state.

### Known limitations
- v0.3 still uses **stop-old -> start-new** replacement. Zero-downtime blue/green traffic switching is the v0.4 milestone.
- The control plane remains SQLite-first/single-node; PostgreSQL multi-worker mode remains planned for v0.8.
- Execution leases are bounded phase ownership, not distributed heartbeats; the default lease is intentionally much longer than a single remote build phase.

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
