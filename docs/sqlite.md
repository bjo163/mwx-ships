# SQLite-First Architecture & Reliability Guide

## Why SQLite-First?

Moonships deliberately uses **SQLite 3** as its canonical MVP database. The goal is simple:
> **Zero external database infrastructure for the first Moonships deployment.**

Unlike traditional PaaS systems that mandate PostgreSQL, Redis, and message broker containers before first boot, Moonships runs entirely from a single file: `data/moonships.sqlite`.

### Advantages
1. **Zero Maintenance Overhead**: No PostgreSQL version migrations, no vacuum tuning, no external connection brokers.
2. **Instant Local Development**: Clone the repo, run migrations, and start the app immediately.
3. **Atomic Backups**: The entire system state (applications, servers, deployment logs, and queues) can be backed up with a single SQLite command.
4. **Low Resource Footprint**: Runs easily on small 512MB or 1GB VPS instances.

## SQLite Configuration & WAL Mode

Moonships configures SQLite with:
- **Journal Mode**: `WAL` (Write-Ahead Logging). This allows concurrent readers while a writer writes to the journal.
- **Busy Timeout**: 5000ms. Prevents transient `SQLITE_BUSY` errors under concurrent worker execution.
- **Synchronous**: `NORMAL` mode in production for high throughput with ACID safety.

Connection URI example:
```text
sqlite://data/moonships.sqlite?mode=rwc
```

## Persistent Worker Queue

Moonships configures Loco's `BackgroundQueue` with `queue.kind: Sqlite`.
Deployments do not use ephemeral in-memory queues (`BackgroundAsync`). When a deployment job is enqueued:
1. An entry is written to the SQLite queue table.
2. If Moonships crashes or is restarted, the job remains persisted on disk.
3. Upon control plane startup, the worker picks up pending jobs and executes them.

## Production Concurrency Limitations & Future Path

While SQLite is outstanding for single-node deployments and small teams:
- **SQLite is not designed for distributed multi-writer control planes across multiple nodes.**
- When scaling to multi-operator environments with multiple concurrent control plane workers, Moonships plans a **PostgreSQL Scale Adapter** (Milestone M7).
- SeaORM abstracts database queries so the transition to PostgreSQL will require zero business logic changes.
