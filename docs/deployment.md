# Deployment Engine & Orchestration

The Moonships deployment engine orchestrates the transformation of Git source code into live running Docker containers on target servers.

## Orchestration Flow

```
POST /api/applications/:id/deploy
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
     DeploymentWorker starts on background thread
              |
              v
     DeploymentService State Machine:
       1. cloning       -> Git clone / fetch / checkout commit
       2. building      -> Docker build or image pull
       3. stopping_old  -> Stop & remove previous application container
       4. starting_new  -> Run container with env vars & Traefik labels
       5. healthchecking-> HTTP health check retries against configured path
       6. success       -> Update application status to running
```

## Concurrency Policy: Single Active Deployment

Moonships strictly enforces **one active deployment per application** at a time:
- If a deployment is triggered while another deployment for the same application is in states `queued`, `cloning`, `building`, `stopping_old`, `starting_new`, or `healthchecking`, Moonships rejects the request with **HTTP 409 Conflict**.
- This guarantees no race conditions or overlapping Docker build contexts.

## Real-Time Log Streaming

Every line emitted during the build and run process is recorded into the `deployment_logs` table with:
- `deployment_id`
- `sequence` (integer ordering)
- `stream` (`stdout`, `stderr`, or `system`)
- `message`
- `created_at`

Clients poll or stream logs via `GET /api/deployments/:id/logs`.
Logs are stored as individual database records rather than a monolithic text blob, preventing truncation and memory bloat.
