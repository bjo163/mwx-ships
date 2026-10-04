# Server Preflight Inspection

Before deploying workloads to a registered server, Moonships can execute a non-destructive target check through:

`POST /api/servers/:id/preflight`

The route requires JWT authentication.

## Checks Performed in v0.2

1. **SSH connectivity and authentication** using the server's configured encrypted private key.
2. **SSH host verification** through strict host-key checking and optional SHA256 fingerprint pinning.
3. **Docker availability** via `docker --version`.
4. **Docker daemon/user access** via `docker info`.
5. **Disk availability** on the target root filesystem.
6. **Available memory** in MB.
7. **CPU core count**.

The report marks the server healthy only when SSH succeeds, Docker is installed/running for the deploy user, and no blocking issue is detected.

## Non-Destructive Policy

Preflight does not install packages, reconfigure Docker, reboot the machine, or mutate application containers. It only runs read-only inspection commands required to determine target readiness.
