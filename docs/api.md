# Moonships REST API Reference

The machine-readable **v1 compatibility contract** is [openapi-v1.json](openapi-v1.json). It is checked into the repository and exercised by CI so documented route/method pairs cannot disappear unnoticed.

## Compatibility Rules

For v1.x:
- documented path + HTTP method pairs are stable;
- additive endpoints and additive response fields are allowed;
- removing or repurposing a documented operation is a breaking change;
- authentication/authorization may become stricter when required to fix a security vulnerability.

## Authentication

`GET /api/health` stays public for liveness.

Operational routes accept authenticated principals:
- revocable JWT sessions;
- scoped organization API tokens where the requested permission is allowed.

Git-provider webhooks are public HTTP endpoints but authenticate using provider-specific signatures/tokens rather than JWT.

## Response Shape

Typical success envelope:

```json
{
  "data": {},
  "message": "ok"
}
```

Typical error envelope:

```json
{
  "error": {
    "code": "ERROR_CODE",
    "message": "Human readable description"
  }
}
```

Individual Loco authentication routes may retain framework-compatible error structures where already part of the contract.

## API Areas

- `/api/auth/*` — registration, login, session inspection/revocation, recovery, magic links.
- `/api/health` — public liveness/database-mode health.
- `/api/servers/*` — remote targets, SSH connection test, preflight.
- `/api/projects/*` — projects and environments.
- `/api/applications/*` — application configuration, deploy/rollback/lifecycle, secrets, domains, Git integrations.
- `POST /api/source-inspection` — RBAC-scoped passive repository inspection on the selected target server; detects Compose/Dockerfile/static signals without executing repository build scripts.
- `/api/deployments/*` — history, logs, cancel, retry.
- `/api/webhooks/*` — signed GitHub/GitLab/Gitea deployment/preview events.
- `/api/operations/*` — metrics, operational health/events, target health, backups, polling.
- `/api/organizations/*` — organizations, memberships, tokens, audit, server pools, private registries, legacy claiming.

The exhaustive route and method inventory is intentionally kept in the OpenAPI file instead of duplicated here.

## Source Inspection

Before an application is created, an authenticated caller with application-management permission can inspect repository metadata through `POST /api/source-inspection`.

The request supplies `project_id`, `server_id`, `git_repository`, and optional `git_branch` (default `main`). The project and target server must belong to the same organization.

Inspection happens on the selected remote target, not on the Moonships control plane. Moonships shallow-clones into a temporary inspection workspace, reads only an allowlist of small non-symlink metadata files, returns a typed strategy/confidence/reason result, and removes the workspace. Repository package/build scripts are never executed during inspection.

Supported initial signals are Compose files, common Dockerfile locations, root static `index.html`, Vite metadata, package-manager lockfiles, and numeric Dockerfile `EXPOSE` hints. Ambiguous multiple Compose/Dockerfile definitions return `unresolved` so the user can explicitly choose rather than relying on hidden guessing.

## Deployment Plans

Moonships can persist deployment intent **before queueing worker execution**.

`POST /api/deployment-plans` accepts an `application_id`, full Git `commit_hash`, and optional commit message. The endpoint validates the application configuration before worker execution and persists the plan using the existing immutable deployment revision model.

The response includes a stable `plan_fingerprint` and `revision_id`. Environment variables expose only key, secret classification, and value fingerprint; plaintext or encrypted secret material is never returned.

`POST /api/deployment-plans/{revision_id}/deploy` queues the exact persisted plan. Execution checks out the plan's exact commit and verifies it matches before build/start. If application settings change after planning, the queued planned deployment still uses the immutable revision snapshot that was reviewed.

Initial pre-queue validation covers Git source syntax, workload/build strategy, safe relative build paths, container/published/health ports, healthcheck path, hostname validity, duplicate hostname ownership, published-port conflicts on the same server, required prebuilt-image input, and the current Compose/managed-ingress compatibility boundary.

## Managed Services

Moonships v1.2 treats PostgreSQL, MySQL, MariaDB, and Redis as first-class **managed service templates** without turning the control plane into a database operator.

`GET /api/services/templates` returns the supported pinned templates. `POST /api/services` declares a service on an authorized target server and creates a dedicated deletion-protected persistent volume. Generated credentials are encrypted through `CryptoService`; API views expose connection metadata and a secret reference, never plaintext or stored ciphertext.

Service lifecycle is explicit:

- `POST /api/services/{id}/start` ensures Docker, the private Moonships network, and the protected volume on the selected SSH target, pulls the pinned image, starts the container, and waits for a bounded service-specific readiness check.
- `POST /api/services/{id}/stop` and `POST /api/services/{id}/restart` control only the service container; persistent data is not removed.
- `GET /api/services/{id}/status` reports observed container state plus readiness.
- `GET /api/services/{id}/logs` returns logs with known generated credentials redacted.

`POST /api/services/{id}/bindings` links a service to a single-container application on the same organization and target server. Moonships writes host, internal port, optional database/user, and an encrypted password into the application's environment namespace using a configurable prefix. The application is then redeployed so the binding becomes part of its immutable revision snapshot.

Managed services and ordinary single-container applications share the internal `moonships-ingress` Docker network, so service hostnames remain private and no database port needs to be published. Compose application bindings remain intentionally unsupported in this focused v1.2 scope.

Persistent-volume APIs are exposed under `/api/volumes`. Volume declarations have stable identity, deletion protection, explicit attach/detach semantics, and cannot be implicitly destroyed by application/service redeploys.

All release acceptance remains reproducible through CI/local Docker and does not require a real VPS.

### Stateful backup and restore

Managed service backups use a provider-neutral **offline Docker-volume snapshot** contract rather than database-specific operator APIs. This intentionally keeps Moonships in the Mini-PaaS role.

`POST /api/services/{id}/backups` performs a bounded sequence on the selected SSH target:

1. if the service is running, stop its container so the volume is filesystem-consistent;
2. archive the existing protected Docker volume with the pinned `MOONSHIPS_BACKUP_HELPER_IMAGE` (default `alpine:3.22.2`);
3. record remote artifact size + SHA-256 in control-plane metadata tied to organization, service, server and volume identity;
4. restore the artifact into a temporary isolated volume;
5. boot the same pinned service image against that temporary volume and require its normal readiness check to pass;
6. remove the temporary verification runtime/volume;
7. resume the original service if it was running.

The safe API representation exposes checksum, size, status, verification, protection and timestamps. It does **not** expose the remote artifact filesystem path or service credentials.

`POST /api/services/{id}/backups/{backup_id}/restore` has stronger destructive guards:

- only successful, verified backups are accepted;
- checksum/size/path identity is revalidated against the remote artifact;
- the target managed service must be stopped;
- `x-moonships-confirmation` must exactly equal the managed service name;
- the backup is restored into an isolated verification volume and boot-tested **before** the protected real volume is overwritten;
- after the real volume is restored, Moonships recreates the service runtime and requires readiness before marking the restore successful.

Backup retention defaults to the newest 7 successful backups per service through `MOONSHIPS_SERVICE_BACKUP_RETENTION`. Older unprotected backups are pruned best-effort; protected backups are skipped. Backup deletion itself requires protection to be disabled and `x-moonships-confirmation: backup-{id}`.

Current v1.2 backup artifacts live on the selected target server under Moonships-managed storage. This is a verified local recovery primitive, not an off-site disaster-recovery substitute.
