# ADR 0003: Persistent Deployment Worker Queue via SQLite

## Status
Accepted

## Context
Deployments involve long-running asynchronous workflows: cloning Git repositories over the network, building Docker container images, stopping legacy containers, starting new containers, and performing HTTP healthcheck retries.

Using ephemeral in-memory worker queues (`BackgroundAsync`) means any server restart or process crash causes in-flight and queued deployments to vanish without trace. Requiring Redis or RabbitMQ adds external dependencies that violate our SQLite-first principle.

## Decision
We utilize Loco's `BackgroundQueue` engine with the SQLite storage backend (`queue.kind: Sqlite`):
1. **Durable Job Enqueuing**: All deployment requests insert a serialized job payload containing strictly the `{ deployment_id }` integer into the persistent SQLite queue table.
2. **Crash Resilience**: If Moonships restarts or crashes during a deployment sequence, queued jobs remain in SQLite storage and are automatically picked up by workers upon reboot.
3. **Thin Payload Policy**: Job payloads contain no private keys, passwords, database connections, or raw logs. The worker deserializes only the `deployment_id` and loads all context fresh from the database.

## Consequences
- Guaranteed persistence across process restarts without Redis.
- Jobs are executed reliably with retry policies and audit trails.
- Queue polling is tuned to prevent database lock contention.
