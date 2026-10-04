# Runbook — Target Server Loss

## Signals
- Dashboard shows target offline/error.
- `/api/operations/metrics` reports offline/error targets.
- A `target_health` operational event and optional alert is emitted.

## Immediate actions
1. Do not retry deployments repeatedly while SSH/Docker is unavailable.
2. Check the server's out-of-band console, network path, SSH service, Docker daemon, disk, and memory.
3. Compare the expected SSH host fingerprint before accepting a rebuilt/replaced host.
4. Run `POST /api/servers/:id/preflight` after connectivity is restored.

## Recovery
- Queued deployments can remain queued.
- A deployment with an expired execution lease is recoverable by the worker from its immutable revision.
- If the host was rebuilt, restore application data independently; Moonships deployment revisions restore application runtime configuration, not application databases/volumes.
- For a permanently lost target, register a replacement server and intentionally move applications only after verifying credentials and domains.

## Escalation
Treat unknown SSH host-key changes, repeated authentication failures, or unexplained data loss as a security incident.
