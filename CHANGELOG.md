# Changelog

All notable changes to **Moonships** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.9.0] - 2026-10-05

### Added
- **Supported Upgrade Matrix**: automated historical-schema fixtures prove direct forward upgrades from v0.2, v0.3, v0.4, v0.5, v0.6, v0.7, and v0.8 into the current schema while preserving application identity and encrypted secret bytes.
- **GA Resilience Suite**: a 128-attempt deployment state-machine soak, lifecycle failure injection across every active deployment phase, and stale-worker lease recovery run as permanent CI gates.
- **Frozen v1 API Contract**: `docs/openapi-v1.json` is the machine-readable v1 route/method compatibility contract and is exercised against the live router in CI.
- **Upgrade & Compatibility Policy**: documented backup-before-upgrade, forward-only migration, downgrade/restore, SQLite-to-PostgreSQL cutover, and v1 compatibility guarantees.
- **v1 Threat Model**: documented assets, trust zones, residual risks, P0/P1 release-blocking definitions, and explicit v1 scope boundaries.
- **Reliability Budgets**: checked-in CI/runtime reliability budgets cover deployment soak, concurrency ownership, stale lease recovery, restore, upgrade, and published-image health.
- **Dependency Policy Gate**: security CI rejects third-party Rust packages without license metadata and rejects unresolved Git dependencies that are not pinned to a concrete commit.
- **Supply-Chain Release Artifacts**: v0.9+ release workflow emits BuildKit SBOM/provenance metadata, a signed GitHub/Sigstore build-provenance attestation, SPDX JSON SBOM, and SHA-256 release checksums.

### Reliability
- SQLite full tests, PostgreSQL full tests, and the verified SQLite→PostgreSQL migration drill remain mandatory release gates.
- Docker release smoke uses the exact published OCI digest rather than only a mutable tag.
- The release smoke verifies OCI version and source-revision labels before booting the control plane.
- Frozen protected API routes are checked for existence and unauthenticated rejection.
- No feature scope was added after v0.8; v0.9 is exclusively a GA hardening/release-candidate release.

### Security
- Secret scanning, Rust advisory audit, dependency provenance/license policy, RBAC regression coverage, and threat-model review are release-blocking gates.
- Published release images are associated with build provenance attestations bound to their OCI digest.
- SPDX SBOM and checksum files are attached to GitHub Releases for external verification.
- GA cannot proceed with unresolved P0/P1 correctness or security defects under the v1 threat-model severity policy.

### Compatibility
- Direct forward schema upgrades are tested from every released schema milestone v0.2 through v0.8.
- The v1 route/method contract is frozen; v1.x may add endpoints/response fields but may not silently remove or repurpose documented operations.
- Database downgrade after a schema-changing upgrade is not supported; restore the verified pre-upgrade backup instead.

### Known limitations
- PostgreSQL HA/backup orchestration remains the database operator/provider responsibility.
- DNS provider automation, Kubernetes/service-mesh orchestration, enterprise SSO, and application data-volume/database backups remain outside the v1 core scope.
- The API contract freezes routes/methods and compatibility behavior; full generated request/response schemas can be expanded additively after GA.

## [0.8.0] - 2026-10-05

### Added
- **PostgreSQL Scale Mode**: Moonships supports PostgreSQL for relational state and the persistent queue when multiple control-plane workers share one deployment.
- **Dual-Database CI**: the complete Rust migration/domain/API suite runs on both SQLite and PostgreSQL.
- **Verified SQLite → PostgreSQL Migration**: `scripts/migrate_sqlite_to_postgres.py` performs dry-run schema parity, dependency ordering, row counts, normalized SHA-256 checksums, import, sequence reset, and foreign-key validation.
- **Migration Drill Gate**: CI creates real SQLite/PostgreSQL schemas, seeds SQLite, runs dry-run + confirmed migration, and verifies the imported data.
- **Distributed Deployment Ownership**: application-level compare-and-set ownership plus execution-token leases prevent multiple workers from executing the same deployment intent.
- **Execution Lease Heartbeat**: active workers periodically renew leases during long remote phases; dead workers naturally stop renewing and stale attempts can be reclaimed.
- **Concurrency Regression**: a 32-way concurrent deployment-claim test proves exactly one application-lock winner on both SQLite and PostgreSQL.
- **Server Pools & Placement**: applications can target server pools with required tags, capacity units, weighted deterministic placement, and current utilization accounting.
- **Compose Workloads**: multi-container Docker Compose workloads support prepare/build/pull/up/down/status/health/log aggregation.
- **Private Registry Credentials**: organization-scoped registry credentials are encrypted at rest and transferred through stdin-backed temporary Docker configuration.
- **PostgreSQL Operations Guide**: explicit scale-mode configuration, cutover, verification, and rollback boundaries are documented in `docs/postgresql-scale.md`.

### Reliability
- SQLite remains the supported default for a single control-plane node.
- PostgreSQL mode uses the same domain/API semantics and test suite as SQLite.
- Application deployment ownership is an atomic database operation, not an in-process mutex.
- Execution phase transitions require the current lease token; a superseded worker cannot advance state after ownership changes.
- Compose lifecycle/status/log operations aggregate all containers in the Compose project.
- Placement selection is deterministic for identical capacity/tag/utilization inputs.

### Security
- Private registry passwords remain AES-256-GCM ciphertext at rest and are excluded from safe API output.
- Docker registry authentication uses `--password-stdin` with an ephemeral isolated Docker config.
- Compose environment material is transferred through SSH stdin into restrictive temporary files.
- PostgreSQL migration does not decrypt Moonships ciphertext; the destination requires the same `ENCRYPTION_KEY`.

### Configuration
- `QUEUE_KIND=Postgres` enables the PostgreSQL queue adapter in scale mode.
- `DATABASE_URL` and `QUEUE_URL` should point at the shared PostgreSQL deployment for multi-worker installations.
- `MOONSHIPS_DEPLOYMENT_HEARTBEAT_SECS` controls periodic execution-lease renewal.
- `DB_AUTO_MIGRATE=false` is recommended for multi-instance deployments; run migrations as a separate release step.

### Known limitations
- Moonships does not operate PostgreSQL HA/backups itself; scale-mode database availability and backup policy remain the operator/database-provider responsibility.
- There is no automatic PostgreSQL → SQLite merge after new writes have been accepted on PostgreSQL; retain the pre-cutover SQLite backup until acceptance is complete.
- Compose support manages one application-level Compose project; Kubernetes/service-mesh orchestration remains outside the v1 core scope.

## [0.7.0] - 2026-10-05

### Added
- **Organizations & Workspaces**: projects and servers can be owned by organizations with a backward-compatible legacy-claim flow.
- **Role-Based Access Control**: Owner, Admin, Deployer, and Viewer membership roles enforce route-level permissions across projects, servers, applications, deployments, domains, Git integrations, and operational actions.
- **Scoped API Tokens**: hashed organization API tokens support least-privilege scopes, expiry, last-used tracking, reveal-once creation, and revocation.
- **Session Revocation**: JWT sessions carry a session version; operators can invalidate previously issued sessions without changing a password.
- **Immutable Administrative Audit Trail**: sensitive and destructive operations record actor, organization, resource, outcome, request correlation, and bounded metadata.
- **Authentication Abuse Controls**: login/sensitive-action rate limiting and confirmation requirements protect high-impact organization operations.
- **Organization Administration API/UI**: membership management, API-token lifecycle, audit history, and legacy resource claiming are available through authenticated organization routes.

### Reliability
- Cross-organization authorization fails closed.
- Organizations must retain at least one active owner.
- Revoked/expired API tokens stop authenticating and token plaintext is never persisted.
- Existing single-admin installations can explicitly claim legacy unowned projects and servers into an organization.

### Security
- API-token authentication uses only stored token hashes and scoped authorization.
- Sensitive organization actions require explicit confirmation metadata and are audit-attributed.
- Viewer/Deployer/Admin/Owner permissions are tested independently; scoped API tokens cannot inherit permissions outside their declared scopes.
- Authentication rate-limit keys are hashed before persistence.

### Known limitations
- v0.7 remains single-control-plane by default; PostgreSQL multi-worker scale mode is finalized in v0.8.
- Enterprise identity federation/SSO is outside the v1.0 core scope.

## [0.6.0] - 2026-10-04

### Added
- **Operational Metrics & Health**: authenticated metrics/health APIs expose deployment queue state, outcomes, stale leases, target availability, disk pressure, and backup health with bounded cardinality.
- **Target Health Timeline**: periodic and manual preflight results are persisted as server health samples and operational events.
- **Verified SQLite Backups**: scheduled/manual online SQLite backups support retention, integrity verification, optional whole-export AES-256-GCM encryption, and persistent backup history.
- **Restore Drill**: automated tests restore an encrypted backup into a clean database and verify schema, application records, immutable deployment revisions, and encrypted secrets.
- **Operator Backup Tasks/API/UI**: operators can queue backups, inspect history, verify exports, and run documented recovery procedures.
- **Operational Notifications**: deduplicated webhook/email alerts cover target loss, disk pressure, stale deployment leases, repeated deployment failures, and backup failures.
- **Incident Runbooks**: documented restore, target-loss, stuck-deployment, notification, and secret-rotation procedures.
- **Operations Dashboard Signals**: dashboard/settings surfaces target, backup, queue, stale-work, and retention state.

### Reliability
- Scheduled and manual backups serialize through a process lock.
- Backup retention prevents unbounded archive growth.
- Alert claims use a cooldown fingerprint to prevent notification storms.
- Backup/notification failures never rewrite an otherwise successful deployment result.
- Restore tests use isolated SeaORM databases and do not mutate Loco process-global DATABASE_URL/QUEUE_URL configuration.

### Security
- Encrypted backup exports use authenticated AES-256-GCM encryption.
- Backup files are written with restrictive permissions on Unix.
- Operational APIs require authenticated control-plane access.
- Notification payloads contain bounded operational context and no decrypted application secrets.

### Configuration
- `MOONSHIPS_BACKUP_INTERVAL_SECS`, `MOONSHIPS_BACKUP_DIR`, `MOONSHIPS_BACKUP_ENCRYPT`, and `MOONSHIPS_BACKUP_RETENTION` control backup scheduling/storage.
- `MOONSHIPS_OPERATIONS_POLL_INTERVAL_SECS` controls target health polling.
- `MOONSHIPS_ALERT_WEBHOOK_URL`, `MOONSHIPS_ALERT_EMAIL`, and `MOONSHIPS_ALERT_COOLDOWN_SECS` configure alert delivery/deduplication.

### Known limitations
- v0.6 remains SQLite-first/single-control-plane; PostgreSQL multi-worker mode is the v0.8 milestone.
- Application data volumes/databases are outside Moonships control-plane backup scope.
- Automated ENCRYPTION_KEY re-encryption/rotation remains a documented operator procedure rather than an in-place command.

## [0.5.0] - 2026-10-04

### Added
- **Git Provider Integrations**: GitHub, GitLab, and Gitea integrations persist normalized repository identity, capability metadata, encrypted provider tokens, encrypted webhook secrets, and optional Git usernames.
- **Signed Webhook Ingestion**: GitHub/Gitea HMAC signatures and GitLab webhook tokens are verified before event processing; forged requests are rejected.
- **Webhook Intent Ledger**: Provider delivery IDs are persisted and replay-safe, while push/preview commit intent keys prevent duplicate deployments even when a provider retries with a different delivery ID.
- **Source-Control Deployment Provenance**: Webhook deployments are recorded with explicit trigger provenance and provider delivery linkage.
- **Provider Commit Status**: Pending/success/failure deployment state is reported back to supported providers with bounded HTTP timeouts.
- **Private Repository Authentication**: HTTPS Git clone/fetch can use encrypted provider credentials through remote `GIT_ASKPASS`; tokens travel over SSH stdin and are never embedded in repository URLs or process arguments.
- **Preview Deployments**: Pull/merge requests create deterministic preview identities with isolated Environment/Application records, copied encrypted runtime configuration, deterministic hostnames, and preview deployment provenance.
- **Preview Teardown**: Close/merge events remove managed routes and preview runtimes, then clean preview Application/Environment records deterministically.
- **Git Integration API**: JWT-protected application endpoints expose safe integration metadata while provider tokens and webhook secrets remain write-only.

### Reliability
- Duplicate provider deliveries are idempotent by both delivery identity and source commit intent.
- Failed webhook deliveries remain retryable without reopening completed deliveries.
- Preview update events fail closed while a prior preview deployment is active.
- Provider status callback failure never changes an otherwise successful Moonships deployment.
- Preview applications inherit encrypted source credentials/configuration without decrypt/re-encrypt churn.

### Security
- Provider secrets are AES-256-GCM encrypted at rest and never returned by safe API serialization.
- Remote private-repository credentials use temporary `0600` token files and an ephemeral `GIT_ASKPASS` script that is removed by a shell trap.
- Git credentials are added to deployment redaction inputs before remote Git failures are persisted.
- Preview webhook integrations are disabled on derived preview applications to avoid creating unintended webhook ingress surfaces.

### Configuration
- `MOONSHIPS_PREVIEW_BASE_DOMAIN` defines the DNS suffix for deterministic preview hostnames and is required for preview creation.
- `MOONSHIPS_PREVIEW_HTTPS=true` enables HTTPS on generated preview domains; managed ACME requirements from v0.4 still apply.

### Known limitations
- Preview creation expects wildcard/appropriate DNS for `MOONSHIPS_PREVIEW_BASE_DOMAIN`; Moonships does not create DNS provider records in v0.5.
- Provider status callbacks are best-effort and intentionally do not fail a deployment when the external Git provider is unavailable.
- Preview teardown returns a conflict while its deployment is actively running; the provider can retry the close delivery after the deployment reaches a terminal state.

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
