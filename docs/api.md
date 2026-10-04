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
- `/api/deployments/*` — history, logs, cancel, retry.
- `/api/webhooks/*` — signed GitHub/GitLab/Gitea deployment/preview events.
- `/api/operations/*` — metrics, operational health/events, target health, backups, polling.
- `/api/organizations/*` — organizations, memberships, tokens, audit, server pools, private registries, legacy claiming.

The exhaustive route and method inventory is intentionally kept in the OpenAPI file instead of duplicated here.
