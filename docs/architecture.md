# Moonships Architecture

Moonships is a self-hosted Mini-PaaS **control plane**. It stores desired state, authorizes operators, schedules durable deployment work, and orchestrates Git/Docker/Compose/Traefik operations over SSH on selected Linux targets.

```
                         MOONSHIPS CONTROL PLANE
             ┌──────────────────┼──────────────────┐
             │                  │                  │
        React Dashboard      REST API        Background Workers
                                │                  │
                         Auth / RBAC / Audit       │
                                │                  │
                                └─────────┬────────┘
                                          v
                       SQLite (single node, default)
                                OR
                   PostgreSQL (multi-worker scale mode)
                                          │
                              durable queue + CAS/leases
                                          │
                                          v
                              DeploymentService
                         Git / SSH / Docker / Compose
                                          │
                                          v
                               Target Linux Server
                                  Docker + Traefik
                           blue/green or Compose runtime
```

## 1. Control Plane vs Runtime

Moonships never treats the control-plane host as the application runtime. Source synchronization, image build/pull, Compose lifecycle, container lifecycle, healthchecks, and managed Traefik changes execute on the selected remote target over SSH.

## 2. Persistence Modes

### SQLite-first
SQLite is the default and supported production mode for one writable control-plane node. It keeps installation and recovery simple and supports verified online backups.

### PostgreSQL scale adapter
PostgreSQL is optional for installations requiring multiple Moonships workers/control-plane processes. In scale mode both `DATABASE_URL` and the persistent queue use PostgreSQL. The same backend suite runs on both databases.

The verified migration path is documented in [postgresql-scale.md](postgresql-scale.md).

## 3. Durable Work & Distributed Ownership

Deployment requests become persistent Deployment rows and durable queue jobs.

Correctness is layered:
1. `applications.active_deployment_id` is claimed with a conditional compare-and-set update;
2. a worker claims the Deployment with a random execution token and expiry lease;
3. long-running executions renew the lease periodically;
4. every state transition requires the current token;
5. after a worker crash, lease renewal stops and a later worker may reclaim stale work.

Webhook delivery/source-intent uniqueness prevents source-control retries from multiplying deployment intents.

## 4. Immutable Revisions & Traffic

A deployment is bound to an immutable revision snapshot containing source/runtime identity while secret material remains ciphertext/fingerprinted. Applications retain current/previous known-good revision pointers.

Managed-ingress workloads use revision-specific candidates:
candidate start → healthcheck → atomic Traefik route switch → previous runtime drain.

Explicit host-port deployments retain a compatibility replacement path.

## 5. Workload Types

Moonships supports:
- single-container Dockerfile builds;
- prebuilt images, including encrypted private-registry credentials;
- Docker Compose application projects with build/pull/up/down/status/health/log aggregation.

Server pools add required tags, capacity accounting, weights, and deterministic placement.

## 6. Organization Security

Projects and servers may be owned by organizations. Owner/Admin/Deployer/Viewer permissions protect resource actions. JWT sessions are revocable, organization API tokens are scoped and hashed, and sensitive operations are appended to an immutable audit trail.

## 7. Git Automation

GitHub, GitLab, and Gitea integrations use encrypted credentials and signed webhook verification. Delivery/source-intent dedupe keeps events idempotent. Pull/merge-request previews create isolated Environment/Application records and deterministic preview routing.

## 8. Operations & Recovery

Moonships exposes bounded operational health/metrics, target-health history, retention state, queue/lease health, and deduplicated notifications. SQLite mode has scheduled verified backups and restore drills. PostgreSQL backup/HA belongs to the database operator/provider.

## 9. Release Boundary

`dev` is the integration branch. `main` is production. Releases publish from `main`, push versioned + `:latest` GHCR images, pull the published image back, smoke-test `/api/health`, then create the annotated tag and GitHub Release.
