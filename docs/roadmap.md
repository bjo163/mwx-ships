# Moonships Platform Roadmap

Moonships follows a disciplined, phased release strategy prioritizing SQLite reliability before multi-cluster scaling.

## Phase 0: Foundation (Milestone M0)
- [x] Loco.rs 1.2 application initialization with Rust 2021 edition.
- [x] Antigravity repository governance, skills, and builder/reviewer/security custom agents.
- [x] GitHub issue backlog sync (#1–#32) and milestone tracking.
- [x] CI workflows and multi-stage containerization.

## Phase 1: SQLite Domain & Mini-PaaS Core (Milestones M1–M4)
- [x] SQLite-first schema migration with WAL configuration.
- [x] Core domain entities (Servers, Projects, Environments, Applications, Secrets, Domains, Deployments, Logs).
- [x] Infrastructure services (`CryptoService`, `GitService`, `SshService`, `DockerService`, `ProxyService`).
- [x] Persistent deployment worker queue (`DeploymentWorker`) via SQLite.
- [x] Non-destructive server preflight checks (`POST /api/servers/:id/preflight`).
- [x] Single active deployment concurrency locking (409 Conflict).
- [x] AES-256-GCM secret encryption and API masking.
- [x] Modern React dashboard with real-time log terminal and deployment tracking.

## Phase 2: Git Webhooks & Automation (Milestone M8)
- [ ] Ingress webhook endpoints for GitHub, GitLab, and Gitea.
- [ ] Automatic deployment triggers on branch push.
- [ ] Commit status reporting back to Git provider.

## Phase 3: Traefik & Automated Let's Encrypt (Milestone M8)
- [ ] Auto-provisioned Traefik container on target hosts.
- [ ] Automatic DNS-01 and HTTP-01 Let's Encrypt SSL/TLS certificates.

## Phase 4: PostgreSQL Scale Adapter (Milestone M7)
- [ ] PostgreSQL driver toggle in configuration.
- [ ] High-concurrency multi-worker cluster support.
- [ ] Database-agnostic SeaORM execution verified.

## Phase 5: Advanced Platform Operations
- [ ] Instant rollback to previous deployment containers.
- [ ] Docker Compose multi-container application definitions.
- [ ] Role-Based Access Control (RBAC) and team workspaces.
