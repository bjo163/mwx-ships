# Moonships REST API Reference

All Moonships API endpoints return JSON payloads conforming to standard formats.

## Standard Formats

### Success Response
```json
{
  "data": { ... },
  "message": "ok"
}
```

### Error Response
```json
{
  "error": {
    "code": "ERROR_CODE",
    "message": "Human readable description"
  }
}
```

## Health Endpoint
- `GET /api/health`: Health status of control plane and SQLite database.

## Servers (`/api/servers`)
- `GET /api/servers`: List all managed servers.
- `POST /api/servers`: Register a new server.
- `GET /api/servers/:id`: Retrieve server details.
- `PUT /api/servers/:id`: Update server configuration.
- `DELETE /api/servers/:id`: Remove server.
- `POST /api/servers/:id/test-connection`: Test SSH connection.
- `POST /api/servers/:id/preflight`: Run comprehensive non-destructive preflight check.

## Projects & Environments (`/api/projects`)
- `GET /api/projects`: List projects.
- `POST /api/projects`: Create project.
- `GET /api/projects/:id`: Get project details and child environments.
- `PUT /api/projects/:id`: Update project.
- `DELETE /api/projects/:id`: Remove project.
- `POST /api/projects/:id/environments`: Add environment (e.g. production, staging).

## Applications (`/api/applications`)
- `GET /api/applications`: List applications.
- `POST /api/applications`: Create application.
- `GET /api/applications/:id`: Application details.
- `PUT /api/applications/:id`: Update application.
- `DELETE /api/applications/:id`: Remove application.
- `POST /api/applications/:id/deploy`: Enqueue deployment (`202 Accepted`).
- `POST /api/applications/:id/start`: Start container.
- `POST /api/applications/:id/stop`: Stop container.
- `POST /api/applications/:id/restart`: Restart container.
- `GET /api/applications/:id/status`: Live container status.
- `GET /api/applications/:id/logs`: Container runtime stdout/stderr.

## Environment Variables (`/api/applications/:id/environment`)
- `GET /api/applications/:id/environment`: List env vars (secrets masked).
- `POST /api/applications/:id/environment`: Set or update variable.
- `DELETE /api/applications/:id/environment/:key`: Delete variable.

## Domains (`/api/applications/:id/domains`)
- `GET /api/applications/:id/domains`: List mapped domains.
- `POST /api/applications/:id/domains`: Map custom hostname.
- `DELETE /api/applications/:id/domains/:domain_id`: Remove domain.

## Deployments (`/api/deployments`)
- `GET /api/deployments`: Global deployment history.
- `GET /api/deployments/:id`: Single deployment status.
- `GET /api/deployments/:id/logs`: Chronological log entries.
