# MOONSHIPS — High-Level Roadmap

> **Deploy. Control. Own your infrastructure.**

Moonships follows a disciplined, SQLite-first progression from zero-infrastructure single-node deployments to distributed cluster scale.

---

### Phase 0 — Foundation & Architecture
- Establish Loco.rs SQLite scaffold and persistent SQLite background queue.
- Implement Antigravity development discipline, repository policy, and CI quality gates.
- Configure repository governance, security disclosure policies, and contributor guidelines.

### Phase 1 — SQLite Mini-PaaS (Current MVP Target)
- Canonical SQLite domain entities (`Server`, `Project`, `Environment`, `Application`, `EnvironmentVariable`, `Domain`, `Deployment`, `DeploymentLog`).
- Remote infrastructure primitives (`SshService` preflight) plus local-host deployment services (`GitService`, `DockerService`, `ProxyService`, `CryptoService`).
- v0.1 deployment execution targets the control-plane Docker host; selected remote-server execution is tracked in #46.
- Asynchronous `DeploymentWorker` running on Loco's SQLite persistent queue.
- Deployment state machine with non-overlapping locking and sequential log persistence.
- Server preflight checks and HTTP healthchecks.
- Modern React operational dashboard.

### Phase 2 — Remote Target Execution & Git Automation
- Execute deployment build/run/healthcheck on the selected remote server (#46).
- Remove the control-plane Docker socket requirement for remote deployments.

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
