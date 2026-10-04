# MOONSHIPS

> **Deploy. Control. Own your infrastructure.**

Moonships is an independent, self-hosted **Mini-PaaS and deployment control plane** built in Rust with Loco.rs, SeaORM, Docker, and Traefik. SQLite is the first-class single-node default; PostgreSQL is the optional v0.8 scale adapter for multi-worker control planes. Conceptually inspired by modern self-hosted tools like Coolify, Moonships is an original, production-grade control plane that gives you full ownership over your servers and workloads.

> **Important Architecture Principle:**  
> **Moonships MVP is strictly SQLite-first.**  
> Zero external database infrastructure (no PostgreSQL, no Redis) is required for local development, CI testing, or single-node production deployment. PostgreSQL is available as the optional scale adapter for shared-state multi-worker control planes; SQLite remains the first-class single-node default.

---

## Table of Contents

- [What is Moonships?](#what-is-moonships)
- [Architecture](#architecture)
- [Why SQLite-First?](#why-sqlite-first)
- [Features](#features)
- [MVP Status](#mvp-status)
- [Requirements](#requirements)
- [Quick Start (Docker Compose)](#quick-start-docker-compose)
- [Native Development Quick Start](#native-development-quick-start)
- [Configuration](#configuration)
- [First Deployment Guide](#first-deployment-guide)
- [REST API Reference](#rest-api-reference)
- [Testing](#testing)
- [Security Posture](#security-posture)
- [Documentation Directory](#documentation-directory)
- [Roadmap & Planned Phases](#roadmap--planned-phases)
- [Known Limitations](#known-limitations)
- [Contributing](#contributing)
- [Support](#support)
- [License](#license)

---

## What is Moonships?

Moonships is a **deployment control plane**, not a container runtime. It provides a web dashboard and REST API to configure remote Linux hosts, connect Git repositories, build Docker images, configure custom domains with Traefik routing, inject encrypted secrets, and orchestrate remote container deployments over secure SSH.

---

## Architecture

```
                         MOONSHIPS CONTROL PLANE
                                    |
                    ┌───────────────┴───────────────┐
                    |                               |
              React Dashboard                   REST API
            (Tailored Dark Theme)                   |
                                                    v
                                                Loco.rs
                                                    |
               ┌────────────────────────────────────┼───────────────────┐
               |                                    |                   |
               v                                    v                   v
        SQLite Database                     Persistent Queue           Auth
     (data/moonships.sqlite)               (_loco_queue table)     (JWT / Admin)
               |                                    |
               |                                    v
               |                            DeploymentWorker
               |                                    |
               |                                    v
               |                            DeploymentService
               |                              /     |     \
               |                            Git    SSH   Docker
               |                             |      |      |
               └─────────────────────────────┼──────┼──────┤
                                             |      |      |
                                             v      v      v
                                          Target Linux Server (Docker Host)
                                                    |
                                                 Traefik
                                         (Reverse Proxy Ingress)
                                                    |
                                      ┌─────────────┴─────────────┐
                                      |                           |
                                App Container               App Container
```

### Core Architecture Principles
1. **Remote Runtime Boundary**: v0.2 executes application Git/Docker/healthcheck operations on the selected Linux server over SSH. The control plane does not mount the host Docker socket.
2. **Durable Queue Engine**: SQLite-backed queueing is the default single-node mode; PostgreSQL-backed queueing is available for multi-worker scale deployments.
3. **Validated Remote Shell Boundary**: Remote commands are composed only from validated inputs and shell-quoted values, with strict SSH host-key checking and bounded execution timeouts.
4. **Least-Privilege Control Plane**: Organization RBAC, scoped API tokens, session revocation, immutable audit events, and abuse controls guard operational actions.
5. **Recoverable Runtime**: Immutable revisions, blue/green managed ingress, rollback, execution leases/heartbeats, backup/restore, and operational health make failure a first-class path.

---

## Why SQLite-First?

Traditional PaaS platforms require spinning up PostgreSQL, Redis, and message broker containers before they can boot. For solo developers, small teams, homelabs, and edge deployments, this introduces massive memory overhead and fragile disaster recovery.

**SQLite remains the simple default:**
- **Zero External DB Dependencies**: single-node installations can keep all state in `data/moonships.sqlite`.
- **Atomic Online Backups**: point-in-time backups use the SQLite Online Backup API.
- **Low Operational Footprint**: ideal for solo developers, small teams, homelabs, and edge deployments.
- **Optional Scale Adapter**: PostgreSQL can replace both relational state and queue persistence when multiple Moonships control-plane workers are required.

---

## Features

- **Server Inventory & Preflight**: Register remote Linux machines over SSH with encrypted private keys, optional SHA256 host-fingerprint pinning, connection tests, and non-destructive preflight checks.
- **Project & Environment Isolation**: Organize services into projects with dedicated environments (`production`, `staging`, `preview`).
- **Flexible Build Types**: Build directly from Git repositories using `Dockerfile` or deploy prebuilt images from container registries.
- **Recoverable Deployment State Machine**: Remote lifecycle supports immutable revisions, `cancelled`, retry provenance, execution leases, and deterministic rollback to the previous known-good revision.
- **Concurrency & Recovery Locking**: Maximum 1 active deployment per application plus execution leases prevent duplicate worker delivery and allow stale interrupted work to be reclaimed.
- **Sequential Real-Time Logs**: Log entries streamed into SQLite with monotonic sequence numbers and stdout/stderr stream separation.
- **Automated Healthchecks**: HTTP polling retries ensure applications respond with 200 OK before marking deployments successful.
- **AES-256-GCM Secret Protection**: SSH private keys and sensitive runtime environment variables are encrypted at rest and masked in API responses.
- **Traefik Reverse Proxy Routing**: Automatic generation of Traefik router and load balancer labels for custom domain SSL/TLS ingress.
- **Modern React Dashboard**: Sleek, responsive dark glassmorphism dashboard built with React 19, Lucide icons, and real-time terminal views.

---

## MVP Status

| Milestone | Scope | Status |
|---|---|---|
| **M0 Foundation** | Antigravity governance, CI workflows, containerization | Complete |
| **M1 SQLite Domain** | SeaORM schema migrations, domain entities | Complete |
| **M2 Remote Infrastructure** | Git, SSH, Docker, and Proxy services | Complete |
| **M3 Deployment Engine** | DeploymentService, persistent SQLite worker | Complete |
| **M4 Dashboard** | React dashboard, live terminal log viewer | Complete |
| **M5 Security** | AES-256-GCM encryption, secret masking, injection validation | Complete |
| **M6 Documentation** | Architectural Decision Records, runbooks, guides | Complete |
| **M7 Deployment Reliability** | Immutable revisions, cancel/retry, rollback, worker leases, retention | Complete (v0.3) |
| **M8 Managed Ingress** | Blue/green, managed Traefik, TLS lifecycle | Complete (v0.4) |
| **M9 Git Automation** | Signed webhooks, provider status, previews | Complete (v0.5) |
| **M10 Production Operations** | Backup/restore, observability, notifications | Complete (v0.6) |
| **M11 Teams & RBAC** | Organizations, roles, API tokens, audit | Complete (v0.7) |
| **M12 Scale Adapter** | PostgreSQL multi-worker, Compose, registries, placement | Complete (v0.8) |
| **M13 GA Hardening** | Upgrade matrix, resilience drills, frozen API, SBOM/provenance | Release candidate (v0.9) |
| **M14 Production GA** | Stable v1 support/upgrade/security contract | Planned (v1.0) |

---

## Requirements

### Host Control Plane
- **OS**: Linux, macOS, or Windows
- **Rust**: 1.94+ (required by the current Loco/SeaORM dependency set)
- **Node.js**: 20+ (for building frontend assets)
- **Docker**: Optional (required if running Moonships in a container)

### Remote Target Hosts (v0.2 execution targets)
- **OS**: Ubuntu 22.04/24.04, Debian 11/12, Rocky Linux 9, or RHEL 9
- **Access**: OpenSSH server (`sshd`)
- **Runtime**: Docker Engine 24.0+ available to the configured deploy user.
- **Utilities**: Git and `curl` available on the target host for source synchronization and HTTP healthchecks.
- **Host identity**: For production, configure `known_host_fingerprint` from an out-of-band trusted source.

---

## Quick Start (Docker Compose)

The fastest way to deploy Moonships in production:

```bash
# 1. Clone repository
git clone https://github.com/bjo163/mwx-ships.git
cd mwx-ships

# 2. Configure environment
cp .env.example .env
# Edit .env and set BOTH required production secrets:
# ENCRYPTION_KEY: generate with `openssl rand -hex 32` (64 hex chars)
# JWT_SECRET: generate a separate strong random value
# docker compose intentionally refuses to start when either value is missing
# port 5150 remains bound to 127.0.0.1 by default; opt in explicitly to wider exposure

# 3. Launch control plane
docker compose up -d

# 4. Open dashboard
open http://localhost:5150
```

---

## Native Development Quick Start

```bash
# 1. Clone and enter repo
git clone https://github.com/bjo163/mwx-ships.git
cd mwx-ships

# 2. Configure local environment
cp .env.example .env
mkdir -p data

# 3. Build frontend assets
cd frontend
npm install
npm run build
cd ..

# 4. Run database migrations
cargo loco db migrate

# 5. Start server and persistent background worker
cargo loco start --server-and-worker
```

---

## Configuration

Settings are configured via `config/*.yaml` and environment variables:

| Variable | Description | Default |
|---|---|---|
| `DATABASE_URL` | SQLite connection URI | `sqlite://data/moonships.sqlite?mode=rwc` |
| `QUEUE_URL` | Persistent queue URI; SQLite by default, PostgreSQL in scale mode | `sqlite://data/moonships.sqlite?mode=rwc` |
| `QUEUE_KIND` | Queue adapter (`Sqlite` or `Postgres`) | `Sqlite` |
| `MOONSHIPS_DEPLOYMENT_HEARTBEAT_SECS` | Multi-worker execution-lease heartbeat | `30` |
| `ENCRYPTION_KEY` | 32-byte hex key for AES-256-GCM encryption | Required in production |
| `PORT` | Control plane HTTP port | `5150` |
| `JWT_SECRET` | Secret key for JWT signing | Configured in yaml |

---

## First Deployment Guide

1. **Add a Server**:
   - Navigate to **Servers** -> **Add Server**.
   - Input your Linux host IP, SSH port, user, and SSH private key.
   - For production, also provide the server's trusted SHA256 SSH host fingerprint.
   - Click **Preflight Check** to verify SSH, Docker Engine, memory, disk space, and CPU.
2. **Create Project & Environment**:
   - Navigate to **Projects** -> **New Project** (e.g. `My SaaS`).
   - Add an environment (e.g. `production`).
3. **Create Application**:
   - Navigate to **Applications** -> **Create Application**.
   - Provide Git repo URL (`https://github.com/bjo163/mwx-ships.git`) and branch (`main`).
   - Specify internal port (e.g. `80`) and optional published port.
4. **Trigger Deployment**:
   - Click **Deploy**. The request returns `202 Accepted` and enqueues into SQLite.
   - Open the **Deployments** tab to watch live build logs stream in real time.
   - In v0.3, the selected server executes the immutable revision; failed/cancelled attempts can be retried and a previous known-good revision can be rolled back.

---

## REST API Reference

Full REST reference is available in [docs/api.md](docs/api.md).

- `GET /api/health`: Health status of control plane and SQLite database.
- `GET /api/servers`: List managed servers.
- `POST /api/servers/:id/preflight`: Run non-destructive server preflight checks.
- `GET /api/applications`: List applications.
- `POST /api/applications/:id/deploy`: Enqueue deployment (`202 Accepted`).
- `POST /api/applications/:id/rollback`: Queue rollback to the previous known-good revision.
- `POST /api/deployments/:id/cancel`: Cancel a deployment before destructive replacement begins.
- `POST /api/deployments/:id/retry`: Queue a new attempt preserving immutable source intent.
- `GET /api/applications/:id/environment`: List environment variables (secrets masked).
- `GET /api/deployments/:id/logs`: Stream sequential build logs.

---

## Testing

Moonships maintains automated test coverage across services, models, workers, and security:

```bash
# Run unit and integration tests (SQLite by default)
cargo test -j 2

# Run service tests specifically
cargo test services -j 2

# CI additionally runs the full suite against PostgreSQL and executes
# a verified SQLite -> PostgreSQL migration drill.
```

---

## Security Posture

- **No Plaintext Private Keys**: Encrypted at rest using AES-256-GCM.
- **No Secret Leakage**: Secrets masked to `••••••••` in API outputs and stripped from logs.
- **Authenticated & Authorized Management API**: Operational routes require JWT or scoped API tokens plus organization RBAC; `/api/health` remains public.
- **Hardened SSH Boundary**: Strict host-key checking, optional SHA256 fingerprint pinning, bounded timeouts, and validated/shell-quoted remote inputs.
- **No Control-Plane Docker Socket**: v0.2 does not mount `/var/run/docker.sock` or run application Docker commands locally.
- **Validation**: Strict validation on SSH targets, container/image names, Docker paths, environment keys, Git branches, and hostnames.

---

## Documentation Directory

Comprehensive technical guides are available in the [`docs/`](docs/) directory:

- [Architecture Guide](docs/architecture.md)
- [SQLite Reliability Guide](docs/sqlite.md)
- [Backup & Disaster Recovery Runbook](docs/backup-and-recovery.md)
- [Deployment Engine & State Machine](docs/deployment.md)
- [Target Server Requirements](docs/server-requirements.md)
- [Server Bootstrap Guide](docs/server-bootstrap.md)
- [Server Preflight Inspection](docs/preflight.md)
- [Security Policies](docs/security.md)
- [Docker Service Guide](docs/docker.md)
- [Git Integration Guide](docs/git.md)
- [Proxy & Traefik Routing](docs/proxy.md)
- [REST API Reference](docs/api.md)
- [Operations Runbook](docs/operations.md)
- [Troubleshooting Runbook](docs/troubleshooting.md)
- [Platform Roadmap](docs/roadmap.md)
- [v1.0 Grand Plan](docs/grand-plan-v1.md)
- [Branch Strategy](docs/branch-strategy.md)
- [ADR 0001: Control Plane Architecture](docs/adr/0001-control-plane.md)
- [ADR 0002: SQLite-First Canonical Storage](docs/adr/0002-sqlite-first.md)
- [ADR 0003: SQLite Worker Queue](docs/adr/0003-sqlite-worker-queue.md)
- [ADR 0004: Docker Runtime & Traefik](docs/adr/0004-docker-runtime.md)
- [PostgreSQL Scale Mode](docs/postgresql-scale.md)
- [v1 Upgrade & Compatibility Policy](docs/upgrade-policy.md)
- [v1 Threat Model](docs/threat-model-v1.md)
- [v1 Reliability Budgets](docs/reliability-budgets.md)
- [Frozen v1 OpenAPI Contract](docs/openapi-v1.json)
- [ADR 0005: PostgreSQL Scale Adapter](docs/adr/0005-postgresql-future-adapter.md)

---

## Network Exposure & Production Security Boundary

The management API requires authenticated, authorized access on operational routes and keeps `GET /api/health` public. Docker Compose still publishes Moonships to **127.0.0.1:5150 only** by default as defense in depth.

If you intentionally expose Moonships beyond localhost, use TLS and a trusted network boundary, rotate `JWT_SECRET` and `ENCRYPTION_KEY`, and pin remote-server SSH fingerprints where possible.

---

## Known Limitations

- **Single Active Deployment per Application**: Concurrent deployments to the same application return 409 Conflict.
- **Database Topology**: SQLite is supported for one writable Moonships control-plane node. Use PostgreSQL state + PostgreSQL queue mode for multiple workers; Moonships does not operate PostgreSQL HA/backups for you.
- **Identity Federation**: organization RBAC, API tokens, and session revocation are supported; enterprise SSO/federation is outside the v1 core scope.
- **SSH Trust Bootstrap**: Without an explicit pinned fingerprint, first host-key discovery is trust-on-first-use. Pin fingerprints for production targets.
- **Managed-Ingress Boundary**: blue/green health-before-traffic applies to Moonships-managed ingress workloads. Explicit host-port deployments retain compatibility replacement semantics.

---

### v0.4 Managed Ingress

Applications with domains and no explicit published host port use Moonships-managed Traefik and revision-specific blue/green runtimes. A candidate must pass its configured healthcheck before Moonships atomically switches the Traefik route; the previous runtime is drained only after the switch. HTTPS domains use Let's Encrypt HTTP-01 when `MOONSHIPS_ACME_EMAIL` is configured.

### v0.8 PostgreSQL Scale Mode

SQLite remains the default single-node production mode. Set `DATABASE_URL`, `QUEUE_KIND=Postgres`, and `QUEUE_URL` to PostgreSQL for multi-worker operation. Moonships tests the full backend suite on both databases and runs a real SQLite→PostgreSQL migration drill in CI. See [docs/postgresql-scale.md](docs/postgresql-scale.md).

---

## Development Branches

The canonical repository intentionally uses only two working branches:

- `dev` — integration/development; all normal work lands here.
- `main` — production; only green `dev -> main` promotions are merged.

Release branches and feature branches are intentionally avoided. See [Branch Strategy](docs/branch-strategy.md).

---

## Contributing

Please review [CONTRIBUTING.md](CONTRIBUTING.md) and our [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) before submitting pull requests.

---

## Support

For bug reports and deployment troubleshooting, please consult [SUPPORT.md](SUPPORT.md) or open an issue on GitHub using our [issue templates](.github/ISSUE_TEMPLATE/).

---

## License

Moonships is open source software licensed under the [Apache License, Version 2.0](LICENSE).
