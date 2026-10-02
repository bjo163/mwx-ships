# Moonships Architecture

Moonships is a lightweight, independent, self-hosted Mini-PaaS and deployment control plane built in Rust with Loco.rs, SQLite-first persistence, SeaORM, Docker, and Traefik.

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

## Core Components

### 1. Control Plane vs Runtime
Moonships is strictly a **control plane**, not a container runtime. It coordinates source code retrieval, image building, and container life cycles over SSH on target Linux hosts without running user workloads inside the control plane process.

### 2. SQLite-First Engine
Moonships requires **zero external database infrastructure** (no PostgreSQL, no Redis). All relational application state and the persistent deployment queue reside in `data/moonships.sqlite` configured in Write-Ahead Logging (WAL) mode.

### 3. Asynchronous Worker & Durability
Deployments are long-running operations. When a client triggers `POST /api/applications/:id/deploy`, the controller records a `Deployment` row in state `queued`, pushes `{ deployment_id }` onto Loco's `BackgroundQueue`, and immediately responds with `202 Accepted`.
The `DeploymentWorker` processes the job sequentially, ensuring that even if Moonships restarts, jobs remain persistent in SQLite and resume automatically.

### 4. Infrastructure Services
- **`GitService`**: Validates URLs, clones repositories, checks out branches/commit SHAs, and extracts commit metadata.
- **`SshService`**: Executes remote commands over SSH safely without arbitrary shell interpolation.
- **`DockerService`**: Validates container names, builds Dockerfiles, starts/stops containers, and reads runtime logs.
- **`ProxyService`**: Generates Traefik dynamic labels for automated zero-downtime routing and TLS.
- **`CryptoService`**: Encrypts sensitive secrets and SSH private keys using AES-256-GCM.
