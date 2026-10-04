# Deployment Engine & Orchestration

Moonships v0.2 transforms Git source code into Docker containers on the **selected remote server**. The control plane records state and queues work in SQLite, while Git/Docker/healthcheck operations execute over SSH on the target.

## Orchestration Flow

```
POST /api/applications/:id/deploy  (JWT required)
              |
              v
     Create Deployment Record
              |
              v
     Enqueue into SQLite Worker Queue
              |
              v
     Return HTTP 202 Accepted (Deployment ID)
              |
              v
     DeploymentWorker
              |
              v
     selected Server -> hardened SSH session
              |
              v
     queued
       -> connecting      SSH + remote Docker preflight
       -> cloning         remote Git clone/fetch/checkout
       -> building        remote Docker build or image pull
       -> stopping_old    stop/remove previous container
       -> starting_new    run replacement with env + labels
       -> healthchecking  target-local HTTP retry loop
       -> success | failed
```

The deployment log records the selected server identity so operators can distinguish the execution target without exposing credentials.

## Remote Execution Boundary

- SSH host/port/username values are validated before command execution.
- SSH uses strict host-key checking with an isolated `known_hosts` file. An optional configured SHA256 fingerprint turns host discovery into explicit pinning.
- Remote commands have bounded timeouts.
- Repository/Docker paths and environment-variable keys are validated.
- Values inserted into remote command strings are shell-quoted.
- Secret environment values are transferred through SSH stdin to a temporary remote env file and are not placed in persisted command logs.
- The production control plane does **not** require the host Docker socket.

## Failure Categories

Deployments persist stable error codes for operational diagnosis, including:

- `SSH_CONNECT_FAILED`
- `REMOTE_DOCKER_UNAVAILABLE`
- `REMOTE_GIT_FAILED`
- `REMOTE_DOCKER_BUILD_FAILED`
- `REMOTE_DOCKER_PULL_FAILED`
- `REMOTE_DOCKER_REPLACE_FAILED`
- `REMOTE_DOCKER_RUN_FAILED`
- `SECRET_DECRYPT_FAILED`
- `REMOTE_HEALTHCHECK_FAILED`

## Concurrency Policy

Moonships enforces one active deployment per application. A new deployment is rejected with HTTP 409 while another deployment is in an active state such as `queued`, `connecting`, `cloning`, `building`, `stopping_old`, `starting_new`, or `healthchecking`.

## Replacement Semantics

v0.2 uses **stop-old -> start-new** replacement. This is intentionally documented as a non-zero-downtime strategy. Blue/green or rolling replacement is future work.

## Log Safety

Deployment logs use monotonic sequence numbers and stdout/stderr/system streams. Known secret environment values are redacted before surfaced runtime errors/container logs are returned. SSH private keys are never persisted in deployment logs.
