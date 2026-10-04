# MOONSHIPS

> **Deploy. Control. Own your infrastructure.**

Moonships is an independent, self-hosted **Mini-PaaS and deployment control plane** built in Rust with Loco.rs, SQLite-first persistence, SeaORM, Docker, and Traefik. Conceptually inspired by modern self-hosted tools like Coolify, Moonships is an original, production-grade control plane that gives you full ownership over your servers and workloads.

> **Important Architecture Principle:**  
> **Moonships MVP is strictly SQLite-first.**  
> Zero external database infrastructure (no PostgreSQL, no Redis) is required for local development, CI testing, or single-node production deployment. PostgreSQL is planned as a future scale adapter for multi-operator clusters.

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

Moonships is a **deployment control plane**, not a container runtime. It provides a web dashboard and REST API to configure remote Linux hosts, connect Git repositories, build Docker images, configure custom domains with Traefik routing, inject encrypted secrets, and orchestrate zero-downtime container deployments over secure SSH.

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
1. **Control Plane Isolation**: Moonships directs remote Docker hosts; user application crashes never impact the control plane.
2. **Persistent Queue Engine**: Deployments run as durable background tasks using Loco's SQLite-backed `BackgroundQueue`. In-flight jobs survive control plane restarts.
3. **Structured Non-Shell Execution**: Remote commands are executed via structured argument vectors—arbitrary shell string concatenation is strictly banned.

---

## Why SQLite-First?

Traditional PaaS platforms require spinning up PostgreSQL, Redis, and message broker containers before they can boot. For solo developers, small teams, homelabs, and edge deployments, this introduces massive memory overhead and fragile disaster recovery.

**Moonships eliminates database bloat:**
- **Zero External DB Dependencies**: All state lives in `data/moonships.sqlite`.
- **Atomic Online Backups**: Point-in-time backups via native SQLite Online Backup API.
- **Low Footprint**: Operates comfortably on 512MB RAM VPS instances.
- **WAL Concurrency**: Write-Ahead Logging mode enables non-blocking concurrent reads and safe serialized writes.

---

## Features

- **Multi-Server Management**: Register remote Linux machines over SSH with connection tests and non-destructive preflight checks.
- **Project & Environment Isolation**: Organize services into projects with dedicated environments (`production`, `staging`, `preview`).
- **Flexible Build Types**: Build directly from Git repositories using `Dockerfile` or deploy prebuilt images from container registries.
- **Durable Deployment State Machine**: 6-stage lifecycle (`queued` -> `cloning` -> `building` -> `stopping_old` -> `starting_new` -> `healthchecking` -> `success`).
- **Concurrency Locking**: Maximum 1 active deployment per application (409 Conflict rejection) prevents overlapping builds.
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
| **M7 PostgreSQL Adapter** | Multi-worker scale adapter | Planned |
| **M8 Advanced Platform** | Webhooks, rollback, compose, RBAC | Planned |

---

## Requirements

### Host Control Plane
- **OS**: Linux, macOS, or Windows
- **Rust**: 1.94+ (required by the current Loco/SeaORM dependency set)
- **Node.js**: 20+ (for building frontend assets)
- **Docker**: Optional (required if running Moonships in a container)

### Target Deployment Hosts
- **OS**: Ubuntu 22.04/24.04, Debian 11/12, Rocky Linux 9, or RHEL 9
- **Access**: OpenSSH server (`sshd`)
- **Runtime**: Docker Engine 24.0+ and `docker-compose-plugin`

---

## Quick Start (Docker Compose)

The fastest way to deploy Moonships in production:

```bash
# 1. Clone repository
git clone https://github.com/bjo163/mwx-ships.git
cd mwx-ships

# 2. Configure environment
cp .env.example .env
# Edit .env and supply a secure 32-byte hex ENCRYPTION_KEY

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
| `QUEUE_URL` | Persistent SQLite queue URI | `sqlite://data/moonships.sqlite?mode=rwc` |
| `ENCRYPTION_KEY` | 32-byte hex key for AES-256-GCM encryption | Required in production |
| `PORT` | Control plane HTTP port | `5150` |
| `JWT_SECRET` | Secret key for JWT signing | Configured in yaml |

---

## First Deployment Guide

1. **Add a Server**:
   - Navigate to **Servers** -> **Add Server**.
   - Input your Linux host IP, SSH port, user, and SSH private key.
   - Click **Preflight Check** to verify SSH, Docker Engine, memory, and disk space.
2. **Create Project & Environment**:
   - Navigate to **Projects** -> **New Project** (e.g. `My SaaS`).
   - Add an environment (e.g. `production`).
3. **Create Application**:
   - Navigate to **Applications** -> **Create Application**.
   - Provide Git repo URL (`https://github.com/bjo163/mwx-ships.git`) and branch (`master`).
   - Specify internal port (e.g. `80`) and optional published port.
4. **Trigger Deployment**:
   - Click **Deploy**. The request returns `202 Accepted` and enqueues into SQLite.
   - Open the **Deployments** tab to watch live build logs stream in real time.

---

## REST API Reference

Full REST reference is available in [docs/api.md](docs/api.md).

- `GET /api/health`: Health status of control plane and SQLite database.
- `GET /api/servers`: List managed servers.
- `POST /api/servers/:id/preflight`: Run non-destructive server preflight checks.
- `GET /api/applications`: List applications.
- `POST /api/applications/:id/deploy`: Enqueue deployment (`202 Accepted`).
- `GET /api/applications/:id/environment`: List environment variables (secrets masked).
- `GET /api/deployments/:id/logs`: Stream sequential build logs.

---

## Testing

Moonships maintains automated test coverage across services, models, workers, and security:

```bash
# Run unit and integration tests
cargo test -j 2

# Run service tests specifically
cargo test services -j 2
```

---

## Security Posture

- **No Plaintext Private Keys**: Encrypted at rest using AES-256-GCM.
- **No Secret Leakage**: Secrets masked to `••••••••` in API outputs and stripped from logs.
- **No Unsafe Shell Concatenation**: Commands execute with discrete arguments.
- **Validation**: Strict validation on container names, image tags, branches, and RFC 1123 hostnames.

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
- [ADR 0001: Control Plane Architecture](docs/adr/0001-control-plane.md)
- [ADR 0002: SQLite-First Canonical Storage](docs/adr/0002-sqlite-first.md)
- [ADR 0003: SQLite Worker Queue](docs/adr/0003-sqlite-worker-queue.md)
- [ADR 0004: Docker Runtime & Traefik](docs/adr/0004-docker-runtime.md)
- [ADR 0005: Future PostgreSQL Adapter](docs/adr/0005-postgresql-future-adapter.md)

---

## Known Limitations

- **Single Active Deployment per Application**: Concurrent deployments to the same application return 409 Conflict.
- **Single-Node Control Plane**: SQLite is optimized for single-node deployments; multi-writer active-active control planes are deferred to the PostgreSQL scale adapter (Milestone M7).
- **Authentication**: MVP implements single-operator administration; multi-tenant teams and RBAC are scheduled for Milestone M8.

---

## Contributing

Please review [CONTRIBUTING.md](CONTRIBUTING.md) and our [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) before submitting pull requests.

---

## Support

For bug reports and deployment troubleshooting, please consult [SUPPORT.md](SUPPORT.md) or open an issue on GitHub using our [issue templates](.github/ISSUE_TEMPLATE/).

---

## License

Moonships is open source software licensed under the [Apache License, Version 2.0](LICENSE).
