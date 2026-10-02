# Docker Service Integration

The `DockerService` manages container lifecycle on target hosts via structured execution.

## Supported Build Types

1. **`dockerfile`**:
   - Clones Git repository.
   - Builds image using `docker build -t <tag> -f <dockerfile_path> <docker_context>`.
   - Tags image deterministically with application slug and deployment timestamp/commit.
2. **`prebuilt_image`**:
   - Pulls public or registry-authenticated image via `docker pull <image>`.
   - Runs directly without local compilation.

## Container Running Invariants

When starting a container:
- Containers run with restart policy `unless-stopped`.
- Internal container ports map to dynamically allocated or user-configured host ports.
- Decrypted environment variables pass via structured environment parameters.
- Traefik routing labels attach directly to the container definition.
- Container names strictly follow `moonships-app-<slug>`.

## Container Runtime Operations
- `POST /api/applications/:id/start`: Start stopped container.
- `POST /api/applications/:id/stop`: Gracefully stop container.
- `POST /api/applications/:id/restart`: Restart container.
- `GET /api/applications/:id/status`: Inspect live container status.
- `GET /api/applications/:id/logs`: Read recent stdout/stderr output.
