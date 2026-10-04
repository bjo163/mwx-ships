# MOONSHIPS — High-Level Roadmap

> **Deploy. Control. Own your infrastructure.**

Moonships follows a disciplined, SQLite-first progression from zero-infrastructure single-node deployments to distributed cluster scale.

---

### Phase 0 — Foundation & Architecture
- Establish Loco.rs SQLite scaffold and persistent SQLite background queue.
- Implement Antigravity development discipline, repository policy, and CI quality gates.
- Configure repository governance, security disclosure policies, and contributor guidelines.

### Phase 1 — SQLite Mini-PaaS (v0.1 Complete)
- Canonical SQLite domain entities (`Server`, `Project`, `Environment`, `Application`, `EnvironmentVariable`, `Domain`, `Deployment`, `DeploymentLog`).
- Remote infrastructure primitives (`SshService` preflight) plus deployment services (`GitService`, `DockerService`, `ProxyService`, `CryptoService`).
- v0.1 established the single-node SQLite control plane and local-host deployment baseline.
- Asynchronous `DeploymentWorker` running on Loco's SQLite persistent queue.
- Deployment state machine with non-overlapping locking and sequential log persistence.
- Server preflight checks and HTTP healthchecks.
- Modern React operational dashboard.

### Phase 2 — Remote Target Execution & Control-Plane Security (v0.2 Complete)
- Execute deployment Git sync/build/run/log/status/healthcheck on the selected remote server (#46).
- Remove the control-plane Docker socket requirement for application deployments.
- Protect operational management routes with JWT (#47).
- Add SSH host-key verification, optional fingerprint pinning, bounded timeouts, stable failure categories, and secret-safe transfer/logging.

### Phase 2B — Git Automation & CI Webhooks
- Automated push webhooks for GitHub, GitLab, and Gitea.
- Branch-based automatic deployments and ephemeral preview environments.
- Git commit metadata synchronization and build triggers.

### Phase 3 — Domains & Automated HTTPS
- Automated Traefik reverse proxy orchestration.
- Let's Encrypt automated TLS certificate issuance and renewal.
- Custom domain routing, path prefixing, and SSL termination.

### Phase 4 — PostgreSQL Scale Adapter
- Abstract database repository abstraction preserving identical business domain models.
- Dual SQLite / PostgreSQL configuration driver.
- Seamless data migration tooling from SQLite to PostgreSQL for high-concurrency clusters.

### Phase 5 — Multi-User & Organization RBAC
- Team workspaces, organization isolation, and Role-Based Access Control (Owner, Admin, Deployer, Viewer).
- Audit logging for administrative operations and configuration mutations.

### Phase 6 — Advanced Infrastructure & Multi-Node
- Docker Compose multi-container application workloads.
- Zero-downtime rolling updates and automated one-click instant rollbacks.
- Multi-server distributed worker pools and private container registry integrations.
- Prometheus metrics exporter and structured observability endpoints.
