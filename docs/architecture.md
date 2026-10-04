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
- **`GitService`**: Provides shared Git URL/branch validation helpers.
- **`SshService`**: Establishes strict host-key-checked SSH sessions, decrypts configured keys into temporary files, supports optional SHA256 fingerprint pinning, and enforces bounded execution timeouts.
- **`RemoteRuntime`**: Runs validated/shell-quoted Git synchronization, Docker build/pull/run/lifecycle/log operations, and target-local HTTP healthchecks on the selected server.
- **`DockerService`**: Supplies container/image validation and configuration types; v0.2 application lifecycle operations do not use the control-plane Docker daemon.
- **`ProxyService`**: Generates Traefik routing labels for target containers. The current replacement strategy is stop-old -> start-new; zero-downtime rollout is not yet implemented.
- **`CryptoService`**: Encrypts sensitive secrets and SSH private keys using AES-256-GCM.
