# ADR 0002: SQLite-First Canonical MVP Storage

## Status
Accepted

## Context
Most self-hosted PaaS and control plane solutions require running PostgreSQL, Redis, or external message brokers before the application can even start. This introduces operational friction, memory bloat, high initial setup overhead, and complex disaster recovery requirements for single-server operators.

We required an embedded, zero-infrastructure storage backend capable of handling relational schemas, transactional migrations, and persistent background job queues.

## Decision
We establish **SQLite 3** (configured with Write-Ahead Logging `WAL` mode) as the canonical, first-class storage engine for Moonships MVP:
1. **Zero External DB Infrastructure**: The initial Moonships installation runs entirely out of a single persistent file: `data/moonships.sqlite`.
2. **Unified State & Queue**: Both relational domain data (servers, projects, applications, deployments) and the background worker queue share SQLite storage.
3. **Connection Pooling & Concurrency Controls**: Using SQLx and SeaORM SQLite pool configurations with appropriate busy timeouts to guarantee reliable concurrent read and serialized write operations.
4. **Transparent Backup**: Online backups are performed atomically using SQLite's native Online Backup API (`.backup` or `VACUUM INTO`).

## Consequences
- Developers can clone the repository, run `cargo loco db migrate`, and start Moonships without installing PostgreSQL or Docker containers for the database.
- Memory footprint stays minimal (<50MB RAM for the entire control plane).
- High-concurrency multi-worker clusters are out-of-scope for the MVP; SQLite is explicitly documented as an MVP and small-scale deployment engine, paving the way for a future PostgreSQL scale adapter (see ADR 0005).
